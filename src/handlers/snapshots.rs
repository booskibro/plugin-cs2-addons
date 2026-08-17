//! Snapshots of the plugin setup (plugins/ + configs/plugins/) as tarballs in
//! addons/counterstrikesharp/backups/. Created before risky operations, kept
//! to a small retention cap, restorable in one call. The tar itself runs on
//! the node — the daemon only ships the file list, not every file's bytes.
//!
//! POST   /servers/{id}/snapshots           create
//! GET    /servers/{id}/snapshots           list (newest first)
//! POST   /servers/{id}/snapshots/restore   {name}
//! DELETE /servers/{id}/snapshots           {name}

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{
    SnapshotCreateResponse, SnapshotInfo, SnapshotListResponse, SnapshotNameRequest,
    SnapshotRestoreResponse,
};
use crate::source2::{self, paths};

const KEEP_SNAPSHOTS: usize = 5;
const SNAPSHOT_PREFIX: &str = "snap-";
const SNAPSHOT_EXT: &str = ".tar";
/// Snapshot members, relative to addons/counterstrikesharp.
const MEMBER_DIRS: &[&str] = &["plugins", "configs/plugins"];

fn css_abs(ctx: &ServerCtx) -> String {
    paths::join(&ctx.game_abs, source2::CSS_DIR)
}

fn backups_abs(ctx: &ServerCtx) -> String {
    paths::join(&ctx.game_abs, source2::BACKUPS_DIR)
}

fn require_linux(ctx: &ServerCtx) -> Result<(), ApiError> {
    if !ctx.node_os.is_empty() && !ctx.node_os.eq_ignore_ascii_case("linux") {
        return Err(ApiError::unprocessable(
            "LINUX_NODE_REQUIRED",
            "snapshots use tar on the node and support linux nodes only",
        ));
    }
    // Same rule as the platform installer: a path that would shellquote-split
    // wrong is refused rather than fed into a node-side command.
    if ctx
        .game_abs
        .chars()
        .any(|c| c.is_whitespace() || c == '"' || c == '\'')
    {
        return Err(ApiError::unprocessable(
            "UNSAFE_PATH",
            "server path contains whitespace or quotes; node-side tar cannot handle it safely",
        ));
    }
    Ok(())
}

/// "snap-1734567890.tar" → 1734567890.
fn parse_snapshot_ts(file_name: &str) -> Option<u64> {
    file_name
        .strip_prefix(SNAPSHOT_PREFIX)?
        .strip_suffix(SNAPSHOT_EXT)?
        .parse()
        .ok()
}

fn validate_name(name: &str) -> Result<(), ApiError> {
    if parse_snapshot_ts(&format!("{name}{SNAPSHOT_EXT}")).is_none() {
        return Err(ApiError::bad_request("invalid snapshot name"));
    }
    Ok(())
}

pub fn handle_create<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    require_linux(&ctx)?;
    super::require_css_installed(host, &ctx)?;

    let css = css_abs(&ctx);
    let members: Vec<&str> = {
        let mut present = Vec::new();
        for member in MEMBER_DIRS {
            let abs = paths::join(&css, member);
            if host.stat(ctx.node_id, &abs)?.is_some_and(|s| s.is_dir) {
                present.push(*member);
            }
        }
        present
    };
    if members.is_empty() {
        return Err(ApiError::unprocessable(
            "NOTHING_TO_SNAPSHOT",
            "neither plugins/ nor configs/plugins/ exists yet",
        ));
    }

    let backups = backups_abs(&ctx);
    if host.stat(ctx.node_id, &backups)?.is_none() {
        host.mk_dir(ctx.node_id, &backups)?;
    }

    let name = format!("{SNAPSHOT_PREFIX}{}", super::now_unix());
    let file_name = format!("{name}{SNAPSHOT_EXT}");
    let archive_abs = paths::join(&backups, &file_name);
    let command = format!("tar -cf {archive_abs} {}", members.join(" "));
    let result = host.exec(ctx.node_id, &command, Some(&css))?;
    if result.exit_code != 0 {
        return Err(ApiError::unprocessable(
            "SNAPSHOT_FAILED",
            format!("tar failed (exit {}): {}", result.exit_code, result.output),
        ));
    }

    let size = host
        .stat(ctx.node_id, &archive_abs)?
        .map(|s| s.size)
        .unwrap_or(0);

    // Retention: keep the newest KEEP_SNAPSHOTS, delete the rest.
    let mut existing = list_snapshots(host, &ctx)?;
    let mut pruned = Vec::new();
    while existing.len() > KEEP_SNAPSHOTS {
        let oldest = existing.pop().expect("len checked");
        let path_abs = paths::join(&backups, &format!("{}{SNAPSHOT_EXT}", oldest.name));
        if host.remove(ctx.node_id, &path_abs, false).is_ok() {
            pruned.push(oldest.name);
        }
    }

    super::audit::record(host, ctx.server_id, actor, "snapshot-create", &name);

    Ok(json_response(
        200,
        &SnapshotCreateResponse {
            snapshot: SnapshotInfo {
                created_at: parse_snapshot_ts(&file_name).unwrap_or_default(),
                path: ctx.rel(&paths::join(source2::BACKUPS_DIR, &file_name)),
                name,
                size,
            },
            pruned,
        },
    ))
}

pub fn handle_list<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let snapshots = list_snapshots(host, &ctx)?;
    Ok(json_response(200, &SnapshotListResponse { snapshots }))
}

/// Newest first.
fn list_snapshots<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
) -> Result<Vec<SnapshotInfo>, ApiError> {
    let backups = backups_abs(ctx);
    let Some(entries) = host.read_dir(ctx.node_id, &backups)? else {
        return Ok(Vec::new());
    };
    let mut snapshots = Vec::new();
    for entry in entries.into_iter().filter(|e| !e.is_dir) {
        let Some(ts) = parse_snapshot_ts(&entry.name) else {
            continue;
        };
        let abs = paths::join(&backups, &entry.name);
        let size = host.stat(ctx.node_id, &abs)?.map(|s| s.size).unwrap_or(0);
        snapshots.push(SnapshotInfo {
            name: entry.name.trim_end_matches(SNAPSHOT_EXT).to_string(),
            created_at: ts,
            size,
            path: ctx.rel(&paths::join(source2::BACKUPS_DIR, &entry.name)),
        });
    }
    snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(snapshots)
}

pub fn handle_restore<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    require_linux(&ctx)?;
    let request: SnapshotNameRequest = parse_json_body(body)?;
    validate_name(&request.name)?;

    let archive_abs = paths::join(
        &backups_abs(&ctx),
        &format!("{}{SNAPSHOT_EXT}", request.name),
    );
    if !host
        .stat(ctx.node_id, &archive_abs)?
        .is_some_and(|s| !s.is_dir)
    {
        return Err(ApiError::not_found("SNAPSHOT_NOT_FOUND", "snapshot not found"));
    }

    // Wipe the live dirs so files deleted since the snapshot do not survive it.
    let css = css_abs(&ctx);
    for member in MEMBER_DIRS {
        let abs = paths::join(&css, member);
        if host.stat(ctx.node_id, &abs)?.is_some() {
            host.remove(ctx.node_id, &abs, true)?;
        }
    }

    let command = format!("tar -xf {archive_abs}");
    let result = host.exec(ctx.node_id, &command, Some(&css))?;
    if result.exit_code != 0 {
        return Err(ApiError::unprocessable(
            "RESTORE_FAILED",
            format!("tar failed (exit {}): {}", result.exit_code, result.output),
        ));
    }

    super::audit::record(host, ctx.server_id, actor, "snapshot-restore", &request.name);

    Ok(json_response(
        200,
        &SnapshotRestoreResponse {
            name: request.name,
            restored: true,
        },
    ))
}

pub fn handle_delete<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: SnapshotNameRequest = parse_json_body(body)?;
    validate_name(&request.name)?;

    let archive_abs = paths::join(
        &backups_abs(&ctx),
        &format!("{}{SNAPSHOT_EXT}", request.name),
    );
    if host.stat(ctx.node_id, &archive_abs)?.is_none() {
        return Err(ApiError::not_found("SNAPSHOT_NOT_FOUND", "snapshot not found"));
    }
    host.remove(ctx.node_id, &archive_abs, false)?;

    super::audit::record(host, ctx.server_id, actor, "snapshot-delete", &request.name);

    Ok(json_response(
        200,
        &SnapshotRestoreResponse {
            name: request.name,
            restored: false,
        },
    ))
}
