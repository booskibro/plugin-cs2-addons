//! Shared request context: server → game (engine gate) → node → game dir.

use std::collections::HashMap;

use crate::host_api::{HostApi, HostApiError};
use crate::http::ApiError;
use crate::source2::{gamedir, paths};

const SOURCE_ENGINE: &str = "source";
/// Upper bound of subdirectories probed for gameinfo.gi.
const GAME_DIR_SCAN_CAP: usize = 16;

pub struct ServerCtx {
    pub server_id: u64,
    pub node_id: u64,
    /// Node operating system as reported by the panel ("linux", "windows").
    pub node_os: String,
    pub game_code: String,
    pub engine: String,
    pub engine_version: String,
    /// Absolute server root on the node.
    pub root_abs: String,
    /// Game folder path relative to the server root, e.g. "game/csgo".
    pub game_dir: String,
    /// Absolute game folder path.
    pub game_abs: String,
}

impl ServerCtx {
    pub fn resolve<H: HostApi>(
        host: &mut H,
        params: &HashMap<String, String>,
    ) -> Result<ServerCtx, ApiError> {
        let server_id: u64 = params
            .get("id")
            .and_then(|raw| raw.parse().ok())
            .ok_or_else(|| ApiError::bad_request("invalid server id"))?;

        let server = host
            .get_server(server_id)?
            .ok_or_else(|| ApiError::not_found("SERVER_NOT_FOUND", "server not found"))?;

        let game = host
            .get_game(&server.game_code)?
            .ok_or_else(|| ApiError::not_found("GAME_NOT_FOUND", "game not found"))?;
        let source2 = game.engine.eq_ignore_ascii_case(SOURCE_ENGINE)
            && game.engine_version.trim().starts_with('2');
        if !source2 {
            return Err(ApiError::unprocessable(
                "UNSUPPORTED_ENGINE",
                format!(
                    "server engine is {:?} v{:?}, expected Source 2",
                    game.engine, game.engine_version
                ),
            ));
        }

        let node = host
            .get_node(server.node_id)?
            .ok_or_else(|| ApiError::not_found("NODE_NOT_FOUND", "node not found"))?;

        let root_abs = paths::join(&node.work_path, &server.dir);
        let game_dir = resolve_game_dir(host, node.id, &root_abs, &server.game_code)?
            .ok_or_else(|| {
                ApiError::unprocessable(
                    "GAME_DIR_NOT_FOUND",
                    "could not locate the game directory (gameinfo.gi) inside the server directory",
                )
            })?;
        let game_abs = paths::join(&root_abs, &game_dir);

        Ok(ServerCtx {
            server_id,
            node_id: node.id,
            node_os: node.os,
            game_code: server.game_code,
            engine: game.engine,
            engine_version: game.engine_version,
            root_abs,
            game_dir,
            game_abs,
        })
    }

    /// Path relative to the server dir, for the frontend file-manager API.
    pub fn rel(&self, game_relative: &str) -> String {
        paths::join(&self.game_dir, game_relative)
    }
}

fn resolve_game_dir<H: HostApi>(
    host: &mut H,
    node_id: u64,
    root_abs: &str,
    game_code: &str,
) -> Result<Option<String>, HostApiError> {
    let hint = gamedir::known_game_dir(game_code);
    if let Some(dir) = hint
        && has_gameinfo(host, node_id, root_abs, dir)?
    {
        return Ok(Some(dir.to_string()));
    }

    let parent_abs = paths::join(root_abs, gamedir::GAME_PARENT);
    let Some(entries) = host.read_dir(node_id, &parent_abs)? else {
        return Ok(None);
    };
    for entry in entries.iter().filter(|e| e.is_dir).take(GAME_DIR_SCAN_CAP) {
        let candidate = paths::join(gamedir::GAME_PARENT, &entry.name);
        if Some(candidate.as_str()) == hint {
            continue;
        }
        if has_gameinfo(host, node_id, root_abs, &candidate)? {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn has_gameinfo<H: HostApi>(
    host: &mut H,
    node_id: u64,
    root_abs: &str,
    dir: &str,
) -> Result<bool, HostApiError> {
    let gameinfo = paths::join(&paths::join(root_abs, dir), crate::source2::GAMEINFO_FILE);
    Ok(host.stat(node_id, &gameinfo)?.is_some_and(|s| !s.is_dir))
}

/// Validates a plugin folder name coming from a request.
pub fn sanitize_plugin_name(name: &str) -> Result<(), ApiError> {
    paths::sanitize_file_name(name).map_err(ApiError::bad_request)?;
    if name.eq_ignore_ascii_case(crate::source2::DISABLED_DIR_NAME) {
        return Err(ApiError::bad_request(
            "the disabled folder itself is not a plugin",
        ));
    }
    Ok(())
}
