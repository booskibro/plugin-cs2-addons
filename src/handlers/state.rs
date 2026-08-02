//! GET /servers/{id}/state — assembles the Metamod:Source/CSS picture of a server.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response};
use crate::model::{CssPluginEntry, CssState, MetamodState, StatePaths, StateResponse};
use crate::source2::{self, gameinfo, paths};

pub fn handle<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;

    // Metamod:Source: the addons dir plus the gameinfo.gi search-path wiring.
    let metamod_abs = paths::join(&ctx.game_abs, source2::METAMOD_DIR);
    let dir_present = host
        .stat(ctx.node_id, &metamod_abs)?
        .is_some_and(|s| s.is_dir);
    let gameinfo_abs = paths::join(&ctx.game_abs, source2::GAMEINFO_FILE);
    let gameinfo_wired = match host.stat(ctx.node_id, &gameinfo_abs)? {
        Some(stat) if !stat.is_dir => {
            gameinfo::is_metamod_wired(&host.download(ctx.node_id, &gameinfo_abs)?)
        }
        _ => false,
    };

    let css_abs = paths::join(&ctx.game_abs, source2::CSS_DIR);
    let css_installed = host.stat(ctx.node_id, &css_abs)?.is_some_and(|s| s.is_dir);

    let manifest = super::read_manifest(host, &ctx)?;

    // Folder scan: plugins/ (enabled) and plugins/disabled/ (disabled).
    let mut plugins: Vec<(String, bool, bool)> = Vec::new(); // (name, enabled, missing)
    let plugins_abs = super::css_plugins_abs(&ctx);
    collect_plugin_dirs(host, &ctx, &plugins_abs, true, &mut plugins)?;
    collect_plugin_dirs(
        host,
        &ctx,
        &super::css_disabled_abs(&ctx),
        false,
        &mut plugins,
    )?;

    // Manifest entries whose folder has vanished — the "Missing" status
    // (the goldsrc analogue: listed in plugins.ini, file deleted).
    let seen: Vec<String> = plugins.iter().map(|(name, ..)| name.clone()).collect();
    for name in manifest.names() {
        if !seen.iter().any(|existing| existing.eq_ignore_ascii_case(name)) {
            plugins.push((name.to_string(), false, true));
        }
    }

    // Group display order: named groups alphabetically, ungrouped last.
    let mut group_names: Vec<String> = Vec::new();
    for (name, ..) in &plugins {
        if let Some(group) = manifest.group(name)
            && !group_names
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&group))
        {
            group_names.push(group);
        }
    }
    group_names.sort_by(|a, b| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()));

    let mut entries = Vec::new();
    for (name, enabled, missing) in plugins {
        let config_rel = paths::join(
            source2::CSS_CONFIGS_DIR,
            &format!("{name}/{name}.json"),
        );
        let config_abs = paths::join(&ctx.game_abs, &config_rel);
        let has_config = host.stat(ctx.node_id, &config_abs)?.is_some_and(|s| !s.is_dir);

        let group = manifest.group(&name);
        let (group_index, group_title) = match &group {
            Some(title) => {
                let index = group_names
                    .iter()
                    .position(|existing| existing.eq_ignore_ascii_case(title))
                    .unwrap_or(group_names.len()) as u32;
                (index, Some(title.clone()))
            }
            None => (u32::MAX, None),
        };

        entries.push(CssPluginEntry {
            comment: manifest.comment(&name),
            group,
            enabled,
            missing,
            has_config,
            config_path: has_config.then(|| ctx.rel(&config_rel)),
            group_index,
            group_title,
            name,
        });
    }

    let response = StateResponse {
        server_id: ctx.server_id,
        game_code: ctx.game_code.clone(),
        engine: ctx.engine.clone(),
        engine_version: ctx.engine_version.clone(),
        game_dir: ctx.game_dir.clone(),
        paths: StatePaths {
            gameinfo: ctx.rel(source2::GAMEINFO_FILE),
            metamod_dir: ctx.rel(source2::METAMOD_DIR),
            css_dir: ctx.rel(source2::CSS_DIR),
            css_plugins_dir: ctx.rel(source2::CSS_PLUGINS_DIR),
            css_disabled_dir: ctx.rel(&paths::join(
                source2::CSS_PLUGINS_DIR,
                source2::DISABLED_DIR_NAME,
            )),
            css_configs_dir: ctx.rel(source2::CSS_CONFIGS_DIR),
            meta_manifest: ctx.rel(source2::META_MANIFEST),
        },
        metamod: MetamodState {
            installed: dir_present && gameinfo_wired,
            dir_present,
            gameinfo_wired,
        },
        css: CssState {
            installed: css_installed,
            plugins: entries,
        },
    };

    Ok(json_response(200, &response))
}

/// Collects `(name, enabled, missing)` for plugin folders directly inside
/// `dir_abs`. `missing` = the folder lacks its `<name>.dll` (broken layout).
fn collect_plugin_dirs<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    dir_abs: &str,
    enabled: bool,
    out: &mut Vec<(String, bool, bool)>,
) -> Result<(), crate::host_api::HostApiError> {
    let Some(dir_entries) = host.read_dir(ctx.node_id, dir_abs)? else {
        return Ok(());
    };
    for entry in dir_entries.into_iter().filter(|e| e.is_dir) {
        if enabled && entry.name.eq_ignore_ascii_case(source2::DISABLED_DIR_NAME) {
            continue;
        }
        let dll_abs = paths::join(
            &paths::join(dir_abs, &entry.name),
            &format!("{}.dll", entry.name),
        );
        let dll_present = host.stat(ctx.node_id, &dll_abs)?.is_some_and(|s| !s.is_dir);
        out.push((entry.name, enabled, !dll_present));
    }
    Ok(())
}
