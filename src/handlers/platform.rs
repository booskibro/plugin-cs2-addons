//! POST /servers/{id}/platform/install — one-click install/update of
//! Metamod:Source or CounterStrikeSharp.
//!
//! Both archives are downloaded BY THE NODE (curl/wget) and unpacked there
//! (tar/unzip): the CSS with-runtime bundle is far past the panel's 10MB
//! plugin-HTTP cap, and the node-side path also keeps big buffers out of the
//! wasm heap. Only the release lookup goes through panel HTTP.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{PlatformInstallRequest, PlatformInstallResponse};
use crate::source2::{self, paths};

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: PlatformInstallRequest = parse_json_body(body)?;

    if !ctx.node_os.is_empty() && !ctx.node_os.eq_ignore_ascii_case("linux") {
        return Err(ApiError::unprocessable(
            "LINUX_NODE_REQUIRED",
            "platform install downloads and unpacks on the node and supports linux nodes only",
        ));
    }

    if !matches!(request.kind.as_str(), "metamod" | "css") {
        return Err(ApiError::bad_request(format!(
            "unknown platform kind {:?}; expected \"metamod\" or \"css\"",
            request.kind
        )));
    }

    // Updates overwrite in place — keep a way back.
    super::snapshots::try_auto_snapshot(host, &ctx, actor, None);

    let (version, gameinfo_patched) = match request.kind.as_str() {
        "metamod" => install_metamod(host, &ctx)?,
        _ => install_css(host, &ctx)?,
    };

    super::audit::record(
        host,
        ctx.server_id,
        actor,
        "platform-install",
        &format!("{} {version}", request.kind),
    );

    Ok(json_response(
        200,
        &PlatformInstallResponse {
            kind: request.kind,
            version,
            gameinfo_patched,
        },
    ))
}

/// The daemon shellquote-splits exec commands and we cannot verify its quote
/// handling from here, so paths that would split wrong are refused outright
/// instead of corrupting a command (the a2fa179 lesson: no shell, no games).
fn require_exec_safe(path: &str) -> Result<(), ApiError> {
    if path.chars().any(|c| c.is_whitespace() || c == '"' || c == '\'') {
        return Err(ApiError::unprocessable(
            "UNSAFE_PATH",
            format!(
                "server path {path:?} contains whitespace or quotes; node-side commands cannot handle it safely"
            ),
        ));
    }
    Ok(())
}

fn install_metamod<H: HostApi>(host: &mut H, ctx: &ServerCtx) -> Result<(String, bool), ApiError> {
    let release = super::updates::fetch_metamod_latest(host)
        .map_err(|err| ApiError::unprocessable("RELEASE_LOOKUP_FAILED", err))?;

    let archive_abs = download_to_node(host, ctx, &release.download_url, "mmsource.tar.gz")?;
    let untar = format!("tar -xzf {archive_abs} -C {}", ctx.game_abs);
    let result = host.exec(ctx.node_id, &untar, None)?;
    let _ = host.remove(ctx.node_id, &archive_abs, false);
    if result.exit_code != 0 {
        return Err(ApiError::unprocessable(
            "EXTRACT_FAILED",
            format!("tar failed (exit {}): {}", result.exit_code, result.output),
        ));
    }

    // The tarball ships addons/metamod; wiring gameinfo.gi completes the install.
    let patched = match super::repair::repair(host, ctx) {
        Ok(changed) => changed,
        Err(err) => {
            host.log_error(&format!("gameinfo patch after metamod install failed: {}", err.message));
            false
        }
    };
    Ok((release.version, patched))
}

fn install_css<H: HostApi>(host: &mut H, ctx: &ServerCtx) -> Result<(String, bool), ApiError> {
    let release = super::updates::fetch_github_latest(
        host,
        super::updates::CSS_REPO,
        super::updates::CSS_ASSET_PATTERNS,
    )
    .map_err(|err| ApiError::unprocessable("RELEASE_LOOKUP_FAILED", err))?;
    let url = release.download_url.ok_or_else(|| {
        ApiError::unprocessable(
            "NO_MATCHING_ASSET",
            "latest CounterStrikeSharp release has no with-runtime linux asset",
        )
    })?;

    let archive_abs = download_to_node(host, ctx, &url, "counterstrikesharp.zip")?;
    let unzip_result = unzip_on_node(host, ctx, &archive_abs)?;
    let _ = host.remove(ctx.node_id, &archive_abs, false);
    unzip_result?;

    let css_abs = paths::join(&ctx.game_abs, source2::CSS_DIR);
    if !host.stat(ctx.node_id, &css_abs)?.is_some_and(|s| s.is_dir) {
        return Err(ApiError::unprocessable(
            "EXTRACT_FAILED",
            "archive extracted but addons/counterstrikesharp did not appear; unexpected release layout",
        ));
    }

    // Python's zipfile drops unix modes; make the runtime executable again.
    for rel in ["dotnet/dotnet", "bin/linuxsteamrt64/counterstrikesharp.so"] {
        let target = paths::join(&css_abs, rel);
        if host.stat(ctx.node_id, &target)?.is_some() {
            let _ = host.chmod(ctx.node_id, &target, 0o755);
        }
    }

    // CSS needs Metamod wired; patch if the file is patchable, else surface later.
    let patched = super::repair::repair(host, ctx).unwrap_or(false);
    Ok((release.version, patched))
}

/// Downloads a URL into the server's scratch dir with curl, falling back to
/// wget. Returns the absolute archive path.
fn download_to_node<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    url: &str,
    file_name: &str,
) -> Result<String, ApiError> {
    require_exec_safe(&ctx.root_abs)?;
    require_exec_safe(&ctx.game_abs)?;
    require_exec_safe(url)?;
    let scratch_abs = paths::join(&ctx.root_abs, source2::DOWNLOAD_SCRATCH_DIR);
    if host.stat(ctx.node_id, &scratch_abs)?.is_none() {
        host.mk_dir(ctx.node_id, &scratch_abs)?;
    }
    let archive_abs = paths::join(&scratch_abs, file_name);

    let attempts = [
        format!("curl -fsSL --retry 2 -o {archive_abs} {url}"),
        format!("wget -q -O {archive_abs} {url}"),
    ];
    let mut last_failure = String::new();
    for command in &attempts {
        match host.exec(ctx.node_id, command, None) {
            Ok(result) if result.exit_code == 0 => {
                if host
                    .stat(ctx.node_id, &archive_abs)?
                    .is_some_and(|s| !s.is_dir && s.size > 0)
                {
                    return Ok(archive_abs);
                }
                last_failure = "download produced an empty file".into();
            }
            Ok(result) => {
                last_failure = format!("exit {}: {}", result.exit_code, result.output);
            }
            Err(err) => {
                last_failure = format!("{err:?}");
            }
        }
    }
    Err(ApiError::unprocessable(
        "DOWNLOAD_FAILED",
        format!("could not download on the node (tried curl and wget): {last_failure}"),
    ))
}

/// Extracts a zip on the node: unzip, then python3, then busybox. The outer
/// Result is transport-level (host call failed); the inner one is the
/// extraction outcome, so the caller can delete the archive either way.
fn unzip_on_node<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    archive_abs: &str,
) -> Result<Result<(), ApiError>, ApiError> {
    let attempts = [
        format!("unzip -o -q {archive_abs} -d {}", ctx.game_abs),
        format!("python3 -m zipfile -e {archive_abs} {}/", ctx.game_abs),
        format!("busybox unzip -o -q {archive_abs} -d {}", ctx.game_abs),
    ];
    let mut last_failure = String::new();
    for command in &attempts {
        match host.exec(ctx.node_id, command, None) {
            Ok(result) if result.exit_code == 0 => return Ok(Ok(())),
            Ok(result) => {
                last_failure = format!("exit {}: {}", result.exit_code, result.output);
            }
            Err(err) => {
                last_failure = format!("{err:?}");
            }
        }
    }
    Ok(Err(ApiError::unprocessable(
        "EXTRACT_FAILED",
        format!(
            "no working unzip tool on the node (tried unzip, python3, busybox): {last_failure}. Install one: apt install unzip"
        ),
    )))
}
