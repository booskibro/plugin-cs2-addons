//! GET /servers/{id}/logs — tail of the newest CounterStrikeSharp log file.
//! The frontend filters per plugin; the backend just serves recent lines.
//!
//! The tail reader and the load-failure scanner are shared with the doctor
//! route, which reports plugins that threw while loading.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::{HostApi, HostApiError};
use crate::http::{ApiResult, json_response};
use crate::model::LogsResponse;
use crate::source2::{self, paths};

/// Only the tail of a big log is downloaded — the daemon has no ranged reads,
/// so the cap guards the wasm heap, not the transfer.
const MAX_LINES: usize = 400;
const MAX_TAIL_BYTES: usize = 256 * 1024;

pub(crate) struct LogTail {
    /// Server-relative path of the file the lines came from.
    pub file_rel: String,
    pub lines: Vec<String>,
}

/// Newest `logs/*.txt`, tail-read and split into lines. `None` when the server
/// has no log directory or no log file yet.
pub(crate) fn newest_log_tail<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
) -> Result<Option<LogTail>, HostApiError> {
    let logs_abs = paths::join(&ctx.game_abs, source2::CSS_LOGS_DIR);
    let Some(entries) = host.read_dir(ctx.node_id, &logs_abs)? else {
        return Ok(None);
    };

    // CSS logs are log_YYYYMMDD.txt — lexicographic order is date order.
    let newest = entries
        .into_iter()
        .filter(|e| !e.is_dir && e.name.to_ascii_lowercase().ends_with(".txt"))
        .map(|e| e.name)
        .max();
    let Some(file_name) = newest else {
        return Ok(None);
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

    Ok(Some(LogTail {
        file_rel: ctx.rel(&paths::join(source2::CSS_LOGS_DIR, &file_name)),
        lines,
    }))
}

pub fn handle<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let Some(tail) = newest_log_tail(host, &ctx)? else {
        return Ok(json_response(
            200,
            &LogsResponse {
                file: None,
                lines: Vec::new(),
            },
        ));
    };

    Ok(json_response(
        200,
        &LogsResponse {
            file: Some(tail.file_rel),
            lines: tail.lines,
        },
    ))
}

/// A plugin CounterStrikeSharp failed to load, as recorded in the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadFailure {
    /// Plugin folder name, taken from the dll it failed to load.
    pub plugin: String,
    /// Assembly the loader could not find, when the log names one — the usual
    /// cause is a contract assembly missing from shared/.
    pub missing: Option<String>,
}

const LOAD_FAILURE_MARKER: &str = "Failed to load plugin";
const MISSING_ASSEMBLY_MARKER: &str = "Could not load file or assembly '";
/// The assembly name sits a few lines below the marker, inside the stack trace.
const LOOKAHEAD_LINES: usize = 12;

/// Scans log lines for plugin load failures, newest state per plugin: a plugin
/// that failed and later loaded fine still reports here, so callers should
/// present this as log history rather than live state.
pub(crate) fn find_load_failures(lines: &[String]) -> Vec<LoadFailure> {
    let mut failures: Vec<LoadFailure> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !line.contains(LOAD_FAILURE_MARKER) {
            continue;
        }
        let Some(plugin) = plugin_name_from_line(line) else {
            continue;
        };
        let missing = lines
            .get(index + 1..)
            .unwrap_or_default()
            .iter()
            .take(LOOKAHEAD_LINES)
            .find_map(|following| assembly_name_from_line(following));
        // One entry per plugin — a later failure supersedes an earlier one.
        failures.retain(|existing| !existing.plugin.eq_ignore_ascii_case(&plugin));
        failures.push(LoadFailure { plugin, missing });
    }
    failures
}

/// "… Failed to load plugin from /…/plugins/PlayerSettings/PlayerSettings.dll"
/// → "PlayerSettings".
fn plugin_name_from_line(line: &str) -> Option<String> {
    let path = line.split_whitespace().next_back()?;
    // Only a line that actually ends in a dll path names a plugin; a bare
    // marker line would otherwise yield the last word as a plugin name.
    if !path.to_ascii_lowercase().ends_with(".dll") {
        return None;
    }
    let file = path.rsplit(['/', '\\']).next()?;
    let stem = &file[..file.len() - 4];
    (!stem.is_empty()).then(|| stem.to_string())
}

/// "Could not load file or assembly 'PlayerSettingsApi, Version=1.0.0.0, …"
/// → "PlayerSettingsApi".
fn assembly_name_from_line(line: &str) -> Option<String> {
    let rest = line.split_once(MISSING_ASSEMBLY_MARKER)?.1;
    let name = rest.split([',', '\'']).next()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(raw: &[&str]) -> Vec<String> {
        raw.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn finds_failure_and_missing_assembly() {
        let log = lines(&[
            "2026-08-18 23:34:11.000 -04:00 [INFO] (cssharp:PluginManager) Loading plugins",
            "2026-08-18 23:34:12.888 -04:00 [EROR] (cssharp:PluginManager) Failed to load plugin from /srv/gameap/servers/x/game/csgo/addons/counterstrikesharp/plugins/PlayerSettings/PlayerSettings.dll",
            "System.Reflection.ReflectionTypeLoadException: Unable to load one or more of the requested types.",
            "Could not load file or assembly 'PlayerSettingsApi, Version=1.0.0.0, Culture=neutral, PublicKeyToken=null'.",
            "   at System.Reflection.RuntimeModule.GetDefinedTypes()",
        ]);
        let found = find_load_failures(&log);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].plugin, "PlayerSettings");
        assert_eq!(found[0].missing.as_deref(), Some("PlayerSettingsApi"));
    }

    #[test]
    fn failure_without_a_named_assembly() {
        let log = lines(&[
            "[EROR] (cssharp:PluginManager) Failed to load plugin from /plugins/Broken/Broken.dll",
            "System.Exception: something else went wrong",
        ]);
        let found = find_load_failures(&log);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].plugin, "Broken");
        assert_eq!(found[0].missing, None);
    }

    #[test]
    fn one_entry_per_plugin_and_none_when_clean() {
        let log = lines(&[
            "[EROR] Failed to load plugin from /plugins/Same/Same.dll",
            "Could not load file or assembly 'FirstDep, Version=1.0.0.0'.",
            "[EROR] Failed to load plugin from /plugins/Same/Same.dll",
            "Could not load file or assembly 'SecondDep, Version=1.0.0.0'.",
        ]);
        let found = find_load_failures(&log);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].missing.as_deref(), Some("SecondDep"));

        assert!(find_load_failures(&lines(&["[INFO] All plugins loaded"])).is_empty());
    }

    #[test]
    fn ignores_a_marker_without_a_dll_path() {
        // The lookahead must not run off the end of the buffer either.
        let log = lines(&["[EROR] Failed to load plugin"]);
        assert!(find_load_failures(&log).is_empty());
    }
}
