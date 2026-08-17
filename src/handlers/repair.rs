//! POST /servers/{id}/gameinfo/repair — re-add the Metamod search path that
//! CS2 updates keep reverting out of gameinfo.gi.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response};
use crate::model::RepairGameinfoResponse;
use crate::source2::{self, gameinfo, paths};

const GAMEINFO_PERMISSIONS: u32 = 0o644;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let changed = repair(host, &ctx)?;
    if changed {
        super::audit::record(host, ctx.server_id, actor, "gameinfo-repair", "gameinfo.gi");
    }
    Ok(json_response(200, &RepairGameinfoResponse { changed }))
}

/// Wires gameinfo.gi if needed; returns whether a write happened. Shared with
/// the platform installer and the scheduled auto-repair sweep.
pub fn repair<H: HostApi>(host: &mut H, ctx: &ServerCtx) -> Result<bool, ApiError> {
    let gameinfo_abs = paths::join(&ctx.game_abs, source2::GAMEINFO_FILE);
    if !host
        .stat(ctx.node_id, &gameinfo_abs)?
        .is_some_and(|s| !s.is_dir)
    {
        return Err(ApiError::unprocessable(
            "GAMEINFO_NOT_FOUND",
            "gameinfo.gi not found in the game directory",
        ));
    }
    let content = host.download(ctx.node_id, &gameinfo_abs)?;
    if gameinfo::is_metamod_wired(&content) {
        return Ok(false);
    }
    let Some(patched) = gameinfo::wire_metamod(&content) else {
        return Err(ApiError::unprocessable(
            "GAMEINFO_UNPATCHABLE",
            "gameinfo.gi has no SearchPaths Game entry to anchor the Metamod line; edit it manually",
        ));
    };
    host.upload(ctx.node_id, &gameinfo_abs, &patched, GAMEINFO_PERMISSIONS)?;
    Ok(true)
}
