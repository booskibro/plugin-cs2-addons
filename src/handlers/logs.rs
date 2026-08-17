//! GET /servers/{id}/logs — tail of the newest CounterStrikeSharp log file.
//! The frontend filters per plugin; the backend just serves recent lines.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response};
use crate::model::LogsResponse;
use crate::source2::{self, paths};

/// Only the tail of a big log is downloaded — the daemon has no ranged reads,
/// so the cap guards the wasm heap, not the transfer.
const MAX_LINES: usize = 400;
const MAX_TAIL_BYTES: usize = 256 * 1024;

pub fn handle<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;

    let logs_abs = paths::join(&ctx.game_abs, source2::CSS_LOGS_DIR);
    let Some(entries) = host.read_dir(ctx.node_id, &logs_abs)? else {
        return Ok(json_response(
            200,
            &LogsResponse {
                file: None,
                lines: Vec::new(),
            },
        ));
    };

    // CSS logs are log_YYYYMMDD.txt — lexicographic order is date order.
    let newest = entries
        .into_iter()
        .filter(|e| !e.is_dir && e.name.to_ascii_lowercase().ends_with(".txt"))
        .map(|e| e.name)
        .max();
    let Some(file_name) = newest else {
        return Ok(json_response(
            200,
            &LogsResponse {
                file: None,
                lines: Vec::new(),
            },
        ));
    };

    let file_abs = paths::join(&logs_abs, &file_name);
    let bytes = host.download(ctx.node_id, &file_abs)?;
    let tail_start = bytes.len().saturating_sub(MAX_TAIL_BYTES);
    let text = String::from_utf8_lossy(&bytes[tail_start..]);
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    // A mid-line cut leaves a garbage first line; drop it when we truncated.
    if tail_start > 0 && !lines.is_empty() {
        lines.remove(0);
    }
    let skip = lines.len().saturating_sub(MAX_LINES);
    let lines = lines.split_off(skip);

    Ok(json_response(
        200,
        &LogsResponse {
            file: Some(ctx.rel(&paths::join(source2::CSS_LOGS_DIR, &file_name))),
            lines,
        },
    ))
}
