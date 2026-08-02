pub mod add;
pub mod attributes;
pub mod ctx;
pub mod remove;
pub mod state;
pub mod toggle;

#[cfg(test)]
mod tests;

use crate::host_api::{HostApi, HostApiError};
use crate::http::ApiError;
use crate::source2::{self, manifest::Manifest, paths};

use ctx::ServerCtx;

const MANIFEST_FILE_PERMISSIONS: u32 = 0o644;

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

/// Moves a directory on the node via nodecmd (nodefs has no rename).
/// The destination parent is created when missing.
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
    let command = if ctx.node_os.to_ascii_lowercase().contains("windows") {
        format!(
            "cmd /C \"if not exist \"{}\" mkdir \"{}\" & move /Y \"{}\" \"{}\"\"",
            win_path(dst_parent),
            win_path(dst_parent),
            win_path(src_abs),
            win_path(dst_abs),
        )
    } else {
        format!(
            "mkdir -p '{}' && mv '{}' '{}'",
            sh_quote(dst_parent),
            sh_quote(src_abs),
            sh_quote(dst_abs),
        )
    };
    let result = host.execute_command(ctx.node_id, &command, None)?;
    if result.exit_code != 0 {
        return Err(ApiError::new(
            502,
            "NODE_OPERATION_FAILED",
            format!(
                "failed to move plugin folder (exit {}): {}",
                result.exit_code,
                result.output.trim()
            ),
        ));
    }
    Ok(())
}

/// Escapes a path for single-quoted POSIX shell interpolation.
fn sh_quote(path: &str) -> String {
    path.replace('\'', r"'\''")
}

fn win_path(path: &str) -> String {
    path.replace('/', "\\")
}
