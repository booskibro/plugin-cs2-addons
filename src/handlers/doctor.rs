//! GET /servers/{id}/doctor — every cheap server-side health check in one
//! pass. The frontend adds its own checks (RCON reachability, -usercon in the
//! launch command) on top.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response};
use crate::model::{DoctorCheck, DoctorResponse};
use crate::source2::{self, gameinfo, paths, vdf};

fn ok(id: &str, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        id: id.into(),
        status: "ok".into(),
        detail: detail.into(),
    }
}

fn warn(id: &str, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        id: id.into(),
        status: "warn".into(),
        detail: detail.into(),
    }
}

fn fail(id: &str, detail: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        id: id.into(),
        status: "fail".into(),
        detail: detail.into(),
    }
}

pub fn handle<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let mut checks = Vec::new();

    // Metamod dir + gameinfo wiring.
    let metamod_abs = paths::join(&ctx.game_abs, source2::METAMOD_DIR);
    let metamod_dir = host
        .stat(ctx.node_id, &metamod_abs)?
        .is_some_and(|s| s.is_dir);
    checks.push(if metamod_dir {
        ok("metamod", ctx.rel(source2::METAMOD_DIR))
    } else {
        warn("metamod", "addons/metamod is not installed")
    });

    let gameinfo_abs = paths::join(&ctx.game_abs, source2::GAMEINFO_FILE);
    match host.stat(ctx.node_id, &gameinfo_abs)? {
        Some(stat) if !stat.is_dir => {
            let wired = gameinfo::is_metamod_wired(&host.download(ctx.node_id, &gameinfo_abs)?);
            checks.push(match (metamod_dir, wired) {
                (_, true) => ok("gameinfo", "Metamod search path present"),
                (true, false) => fail("gameinfo", "Metamod installed but gameinfo.gi does not load it"),
                (false, false) => ok("gameinfo", "no Metamod to wire"),
            });
        }
        _ => checks.push(fail("gameinfo", "gameinfo.gi is missing")),
    }

    // CSS presence.
    let css_abs = paths::join(&ctx.game_abs, source2::CSS_DIR);
    let css_dir = host.stat(ctx.node_id, &css_abs)?.is_some_and(|s| s.is_dir);
    checks.push(if css_dir {
        ok("css", ctx.rel(source2::CSS_DIR))
    } else {
        warn("css", "CounterStrikeSharp is not installed")
    });

    if css_dir {
        // Folders present in BOTH plugins/ and plugins/disabled/.
        let enabled_names = folder_names(host, &ctx, &super::css_plugins_abs(&ctx), true)?;
        let disabled_names = folder_names(host, &ctx, &super::css_disabled_abs(&ctx), false)?;
        let duplicates: Vec<&String> = enabled_names
            .iter()
            .filter(|name| {
                disabled_names
                    .iter()
                    .any(|other| other.eq_ignore_ascii_case(name))
            })
            .collect();
        checks.push(if duplicates.is_empty() {
            ok("duplicates", "no folder is both enabled and disabled")
        } else {
            fail(
                "duplicates",
                format!(
                    "in both plugins/ and plugins/disabled/: {} - delete one copy",
                    duplicates
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        });

        // Folder layout: <Name>/<Name>.dll must exist.
        let mut broken = Vec::new();
        for (dir_abs, names) in [
            (super::css_plugins_abs(&ctx), &enabled_names),
            (super::css_disabled_abs(&ctx), &disabled_names),
        ] {
            for name in names {
                let dll_abs = paths::join(&paths::join(&dir_abs, name), &format!("{name}.dll"));
                if host.stat(ctx.node_id, &dll_abs)?.is_none() {
                    broken.push(name.clone());
                }
            }
        }
        checks.push(if broken.is_empty() {
            ok("layout", "every plugin folder carries its dll")
        } else {
            warn("layout", format!("missing <Name>/<Name>.dll: {}", broken.join(", ")))
        });

        // Manifest entries whose folder is gone.
        let manifest = super::read_manifest(host, &ctx)?;
        let orphans: Vec<String> = manifest
            .names()
            .filter(|name| {
                !enabled_names.iter().any(|n| n.eq_ignore_ascii_case(name))
                    && !disabled_names.iter().any(|n| n.eq_ignore_ascii_case(name))
            })
            .map(str::to_string)
            .collect();
        checks.push(if orphans.is_empty() {
            ok("orphans", "plugins_meta.json matches the folders on disk")
        } else {
            warn(
                "orphans",
                format!("tracked but folder gone: {}", orphans.join(", ")),
            )
        });
    }

    if metamod_dir {
        // A vdf that exists both live and parked loads unpredictably.
        let vdfs = vdf::list(host, ctx.node_id, &metamod_abs)?;
        let mut seen: Vec<String> = Vec::new();
        let mut ambiguous: Vec<String> = Vec::new();
        for plugin in &vdfs {
            let lower = plugin.name.to_ascii_lowercase();
            if seen.contains(&lower) {
                if !ambiguous.contains(&plugin.name) {
                    ambiguous.push(plugin.name.clone());
                }
            } else {
                seen.push(lower);
            }
        }
        checks.push(if ambiguous.is_empty() {
            ok("vdf", "Metamod plugin aliases are unambiguous")
        } else {
            fail(
                "vdf",
                format!(
                    "both .vdf and .vdf.disabled exist for: {} - delete one",
                    ambiguous.join(", ")
                ),
            )
        });
    }

    // Leftover download scratch from an interrupted platform install.
    let scratch_abs = paths::join(&ctx.root_abs, source2::DOWNLOAD_SCRATCH_DIR);
    if let Some(entries) = host.read_dir(ctx.node_id, &scratch_abs)?
        && !entries.is_empty()
    {
        checks.push(warn(
            "scratch",
            format!(
                "{} leftover download(s) in {}/ from an interrupted install - safe to delete",
                entries.len(),
                source2::DOWNLOAD_SCRATCH_DIR
            ),
        ));
    } else {
        checks.push(ok("scratch", "no leftover downloads"));
    }

    Ok(json_response(200, &DoctorResponse { checks }))
}

fn folder_names<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    dir_abs: &str,
    skip_disabled_dir: bool,
) -> Result<Vec<String>, crate::host_api::HostApiError> {
    let Some(entries) = host.read_dir(ctx.node_id, dir_abs)? else {
        return Ok(Vec::new());
    };
    Ok(entries
        .into_iter()
        .filter(|e| e.is_dir)
        .filter(|e| {
            !(skip_disabled_dir && e.name.eq_ignore_ascii_case(source2::DISABLED_DIR_NAME))
        })
        .map(|e| e.name)
        .collect())
}
