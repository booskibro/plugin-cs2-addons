//! GET /servers/{id}/audit — who did what, newest first — plus the `record`
//! helper the mutating handlers call. Entries live in plugin storage (not in
//! plugins_meta.json: the in-game AddonsManager plugin shares that file and
//! must not choke on unknown keys).

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response};
use crate::model::{AuditEntry, AuditResponse};

const MAX_ENTRIES: usize = 100;

fn storage_key(server_id: u64) -> String {
    format!("audit:{server_id}")
}

/// Appends an audit entry, best-effort: an audit failure never fails the
/// operation it describes, it only leaves a log line.
pub fn record<H: HostApi>(
    host: &mut H,
    server_id: u64,
    actor: Option<&str>,
    action: &str,
    subject: &str,
) {
    let entry = AuditEntry {
        ts: super::now_unix(),
        user: actor.unwrap_or("unknown").to_string(),
        action: action.to_string(),
        subject: subject.to_string(),
    };
    let key = storage_key(server_id);
    let mut entries = load(host, &key);
    entries.insert(0, entry);
    entries.truncate(MAX_ENTRIES);
    match serde_json::to_vec(&entries) {
        Ok(bytes) => {
            if let Err(err) = host.storage_set(&key, &bytes) {
                host.log_error(&format!("audit write failed: {err:?}"));
            }
        }
        Err(err) => host.log_error(&format!("audit serialize failed: {err}")),
    }
}

fn load<H: HostApi>(host: &mut H, key: &str) -> Vec<AuditEntry> {
    match host.storage_get(key) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Ok(None) => Vec::new(),
        Err(err) => {
            host.log_error(&format!("audit read failed: {err:?}"));
            Vec::new()
        }
    }
}

pub fn handle<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let entries = load(host, &storage_key(ctx.server_id));
    Ok(json_response(200, &AuditResponse { entries }))
}
