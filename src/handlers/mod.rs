pub mod add;
pub mod attributes;
pub mod audit;
pub mod catalog_routes;
pub mod ctx;
pub mod logs;
pub mod metamod;
pub mod platform;
pub mod remove;
pub mod repair;
pub mod restart;
pub mod snapshots;
pub mod state;
pub mod toggle;
pub mod updates;

#[cfg(test)]
mod tests;

use crate::host_api::{HostApi, HostApiError};
use crate::http::ApiError;
use crate::source2::{self, manifest::Manifest, paths};

use ctx::ServerCtx;

const MANIFEST_FILE_PERMISSIONS: u32 = 0o644;

/// Unix seconds. wasm32-wasip1 backs this with the WASI clock; native tests
/// use the OS clock.
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Absolute path of the CounterStrikeSharp plugins dir.
pub fn css_plugins_abs(ctx: &ServerCtx) -> String {
    paths::join(&ctx.game_abs, source2::CSS_PLUGINS_DIR)
}

/// Absolute path of the disabled-plugins parking dir.
pub fn css_disabled_abs(ctx: &ServerCtx) -> String {
    paths::join(&css_plugins_abs(ctx), source2::DISABLED_DIR_NAME)
}

/// 409 unless addons/counterstrikesharp exists on the server.
fn require_css_installed<H: HostApi>(host: &mut H, ctx: &ServerCtx) -> Result<(), ApiError> {
    let css_abs = paths::join(&ctx.game_abs, source2::CSS_DIR);
    if !host.stat(ctx.node_id, &css_abs)?.is_some_and(|s| s.is_dir) {
        return Err(ApiError::conflict(
            "CSS_NOT_INSTALLED",
            "CounterStrikeSharp is not installed on this server",
        ));
    }
    Ok(())
}

/// Reads plugins_meta.json; a missing file yields an empty manifest.
fn read_manifest<H: HostApi>(host: &mut H, ctx: &ServerCtx) -> Result<Manifest, ApiError> {
    let manifest_abs = paths::join(&ctx.game_abs, source2::META_MANIFEST);
    if host.stat(ctx.node_id, &manifest_abs)?.is_none() {
        return Ok(Manifest::default());
    }
    let bytes = host.download(ctx.node_id, &manifest_abs)?;
    Ok(Manifest::parse(&bytes))
}

fn write_manifest<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    manifest: &Manifest,
) -> Result<(), ApiError> {
    let manifest_abs = paths::join(&ctx.game_abs, source2::META_MANIFEST);
    host.upload(
        ctx.node_id,
        &manifest_abs,
        &manifest.to_bytes(),
        MANIFEST_FILE_PERMISSIONS,
    )?;
    Ok(())
}

/// The plugin's folder if it exists: (absolute path, currently enabled).
/// Enabled = directly under plugins/, disabled = under plugins/disabled/.
fn find_plugin_folder<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    name: &str,
) -> Result<Option<(String, bool)>, HostApiError> {
    let enabled_abs = paths::join(&css_plugins_abs(ctx), name);
    if host.stat(ctx.node_id, &enabled_abs)?.is_some_and(|s| s.is_dir) {
        return Ok(Some((enabled_abs, true)));
    }
    let disabled_abs = paths::join(&css_disabled_abs(ctx), name);
    if host
        .stat(ctx.node_id, &disabled_abs)?
        .is_some_and(|s| s.is_dir)
    {
        return Ok(Some((disabled_abs, false)));
    }
    Ok(None)
}

/// Moves a directory via the daemon's native nodefs move, creating the
/// destination parent when missing. NEVER via execute_command: the daemon
/// shellquote-splits and execs commands directly — there is no shell, so
/// `mkdir -p X && mv A B` becomes one mkdir call that creates junk
/// directories (including an empty one AT the move target) and exits 0.
fn move_dir<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    src_abs: &str,
    dst_abs: &str,
) -> Result<(), ApiError> {
    let dst_parent = match dst_abs.rfind('/') {
        Some(idx) => &dst_abs[..idx],
        None => return Err(ApiError::internal("move destination has no parent")),
    };
    if host.stat(ctx.node_id, dst_parent)?.is_none() {
        host.mk_dir(ctx.node_id, dst_parent)?;
    }
    host.move_path(ctx.node_id, src_abs, dst_abs)?;
    Ok(())
}
