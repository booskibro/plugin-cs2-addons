//! Hand-rolled wasm exports for the panel services the Rust SDK does not wrap
//! yet: the ProtocolService (our CS2-tolerant RCON) and the
//! ScheduledTaskHandler (auto-repair + update-check), plus the gameap-net and
//! gameap-scheduler host imports they need.
//!
//! Export names and the ABI envelope mirror what `register_plugin!` emits for
//! the core service; the panel discovers each optional service by probing for
//! its `<service>_api_version` export.

#![cfg(target_arch = "wasm32")]

use gameap_plugin_sdk::abi::{guest_call, host_call};
use gameap_plugin_sdk::proto::gameap::plugin::sdk::{net, protocol, scheduler};
use gameap_plugin_sdk::{HostError, host};

use crate::host_api::WasmHost;
use crate::rcon;

pub const RCON_PROTOCOL_ID: &str = "cs2-tolerant-source";

const AUTOREPAIR_TASK: &str = "cs2addons-gameinfo-autorepair";
const UPDATE_CHECK_TASK: &str = "cs2addons-update-check";
const AUTOREPAIR_INTERVAL_MS: i64 = 6 * 60 * 60 * 1000;
const UPDATE_CHECK_INTERVAL_MS: i64 = 24 * 60 * 60 * 1000;
const TASK_TIMEOUT_MS: i64 = 120 * 1000;

// ---------------------------------------------------------------------------
// Host imports
// ---------------------------------------------------------------------------

mod sys_net {
    #[link(wasm_import_module = "gameap-net")]
    unsafe extern "C" {
        pub fn send(ptr: u32, size: u32) -> u64;
        pub fn recv(ptr: u32, size: u32) -> u64;
    }
}

mod sys_scheduler {
    #[link(wasm_import_module = "gameap-scheduler")]
    unsafe extern "C" {
        pub fn add_task(ptr: u32, size: u32) -> u64;
    }
}

fn net_send(req: &net::NetSendRequest) -> Result<net::NetSendResponse, HostError> {
    unsafe { host_call(sys_net::send, req) }
}

fn net_recv(req: &net::NetRecvRequest) -> Result<net::NetRecvResponse, HostError> {
    unsafe { host_call(sys_net::recv, req) }
}

fn scheduler_add_task(
    req: &scheduler::AddTaskRequest,
) -> Result<scheduler::AddTaskResponse, HostError> {
    unsafe { host_call(sys_scheduler::add_task, req) }
}

/// Registers the recurring tasks; called from `initialize`. Failures are
/// logged, never fatal — the tab works fine without the background sweeps.
pub fn register_scheduled_tasks() {
    for (name, interval_ms) in [
        (AUTOREPAIR_TASK, AUTOREPAIR_INTERVAL_MS),
        (UPDATE_CHECK_TASK, UPDATE_CHECK_INTERVAL_MS),
    ] {
        let result = scheduler_add_task(&scheduler::AddTaskRequest {
            name: name.to_string(),
            interval_ms,
            error_policy: None,
            timeout_ms: TASK_TIMEOUT_MS,
        });
        match result {
            Ok(resp) if resp.success => host::log::info(format!("registered task {name}")),
            Ok(resp) => host::log::error(format!(
                "task {name} not registered: {}",
                resp.error.unwrap_or_default()
            )),
            Err(err) => host::log::error(format!("task {name} registration failed: {err}")),
        }
    }
}

// ---------------------------------------------------------------------------
// gameap-net backed Wire
// ---------------------------------------------------------------------------

struct NetWire {
    handle: u64,
}

impl rcon::Wire for NetWire {
    fn send(&mut self, data: &[u8]) -> Result<(), String> {
        let mut written = 0usize;
        while written < data.len() {
            let resp = net_send(&net::NetSendRequest {
                handle: self.handle,
                data: data[written..].to_vec(),
            })
            .map_err(|e| e.to_string())?;
            if let Some(err) = resp.error {
                return Err(err);
            }
            if resp.written <= 0 {
                return Err("connection wrote nothing".into());
            }
            written += resp.written as usize;
        }
        Ok(())
    }

    fn recv(&mut self, max_bytes: u32, timeout_ms: u32) -> Result<rcon::RecvChunk, String> {
        let resp = net_recv(&net::NetRecvRequest {
            handle: self.handle,
            max_bytes,
            timeout_ms,
        })
        .map_err(|e| e.to_string())?;
        if let Some(err) = resp.error {
            return Err(err);
        }
        Ok(rcon::RecvChunk {
            data: resp.data,
            timeout: resp.timeout,
        })
    }
}

// ---------------------------------------------------------------------------
// Per-connection request-id counter and reassembly state
// ---------------------------------------------------------------------------

// wasm runs single-threaded and the host serializes plugin calls, so plain
// thread_local cells are safe state.
thread_local! {
    static NEXT_ID: std::cell::Cell<i32> = const { std::cell::Cell::new(100) };
    static SESSIONS: std::cell::RefCell<std::collections::HashMap<u64, rcon::Session>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn take_id() -> i32 {
    NEXT_ID.with(|cell| {
        let id = cell.get();
        // Wrap far below the sign bit; -1 is the auth-failure sentinel.
        cell.set(if id > 0x0FFF_FFF0 { 100 } else { id + 1 });
        id
    })
}

fn with_session<R>(handle: u64, f: impl FnOnce(&mut rcon::Session) -> R) -> R {
    SESSIONS.with(|sessions| {
        let mut map = sessions.borrow_mut();
        f(map.entry(handle).or_default())
    })
}

fn drop_session(handle: u64) {
    SESSIONS.with(|sessions| {
        sessions.borrow_mut().remove(&handle);
    });
}

// ---------------------------------------------------------------------------
// ProtocolService exports
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_api_version() -> u64 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_get_rcon_protocols(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |_req: protocol::GetRconProtocolsRequest| {
        Ok(protocol::GetRconProtocolsResponse {
            protocols: vec![protocol::RconProtocol {
                id: RCON_PROTOCOL_ID.into(),
                name: "CS2-tolerant Source RCON".into(),
                game_codes: vec![crate::GAME_CODE.into()],
                engines: Vec::new(),
                transport: protocol::RconTransport::Plugin as i32,
                players: Some(protocol::PlayerCapability {
                    supported: true,
                    players_command: "status".into(),
                    kick_command: "kickid {id} \"{reason}\"".into(),
                    // CS2 has no persistent console ban — leave it unset.
                    ban_command: String::new(),
                    parse_via_plugin: false,
                }),
                builtin_protocol: String::new(),
            }],
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_get_query_protocols(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |_req: protocol::GetQueryProtocolsRequest| {
        Ok(protocol::GetQueryProtocolsResponse {
            protocols: Vec::new(),
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_rcon_open(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |req: protocol::RconOpenRequest| {
        let mut wire = NetWire {
            handle: req.conn_handle,
        };
        // A handle number can be reused after a teardown that skipped
        // RconClose — an open is always a fresh connection, so any stale
        // reassembly buffer under this handle must not leak into it.
        drop_session(req.conn_handle);
        let auth_id = take_id();
        let outcome = with_session(req.conn_handle, |session| {
            rcon::authenticate(&mut wire, session, &req.password, auth_id)
        });
        Ok(match outcome {
            Ok(rcon::AuthOutcome::Ok) => protocol::RconOpenResponse {
                ok: true,
                auth_failed: false,
                error: None,
            },
            Ok(rcon::AuthOutcome::BadPassword) => protocol::RconOpenResponse {
                ok: false,
                auth_failed: true,
                error: Some("rcon password rejected".into()),
            },
            Err(err) => {
                drop_session(req.conn_handle);
                protocol::RconOpenResponse {
                    ok: false,
                    auth_failed: false,
                    error: Some(err),
                }
            }
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_rcon_execute(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |req: protocol::RconExecuteRequest| {
        let mut wire = NetWire {
            handle: req.conn_handle,
        };
        let exec_id = take_id();
        let result = with_session(req.conn_handle, |session| {
            rcon::execute(&mut wire, session, &req.command, exec_id)
        });
        Ok(match result {
            Ok(outcome) => {
                // Quiet on healthy exchanges; loud enough to diagnose a server
                // that answers with foreign ids or nothing at all.
                if outcome.fallback_used || outcome.output.is_empty() {
                    host::log::info(format!(
                        "cs2-addons rcon: {:?} -> {} bytes (own packets {}, foreign packets {}, foreign-id fallback {})",
                        req.command,
                        outcome.output.len(),
                        outcome.own_packets,
                        outcome.foreign_packets,
                        outcome.fallback_used,
                    ));
                }
                protocol::RconExecuteResponse {
                    output: outcome.output,
                    error: None,
                }
            }
            Err(err) => protocol::RconExecuteResponse {
                output: String::new(),
                error: Some(err),
            },
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_rcon_close(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |req: protocol::RconCloseRequest| {
        drop_session(req.conn_handle);
        Ok(protocol::RconCloseResponse { error: None })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_query_server(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |_req: protocol::QueryServerRequest| {
        Ok(protocol::QueryServerResponse {
            result: None,
            error: Some("this plugin registers no query protocols".into()),
        })
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn protocol_service_parse_players(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |_req: protocol::ParsePlayersRequest| {
        Ok(protocol::ParsePlayersResponse {
            players: Vec::new(),
            error: Some("players are parsed by the panel's built-in valve parser".into()),
        })
    })
}

// ---------------------------------------------------------------------------
// ScheduledTaskHandler exports
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub extern "C" fn scheduled_task_handler_api_version() -> u64 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn scheduled_task_handler_handle_scheduled_task(ptr: u32, size: u32) -> u64 {
    guest_call(ptr, size, |req: scheduler::HandleScheduledTaskRequest| {
        let mut host_api = WasmHost;
        match req.task_name.as_str() {
            AUTOREPAIR_TASK => {
                let stats = crate::maintenance::autorepair_sweep(&mut host_api);
                host::log::info(format!(
                    "autorepair sweep: {} checked, {} repaired, {} failed",
                    stats.checked, stats.repaired, stats.failed
                ));
            }
            UPDATE_CHECK_TASK => match crate::handlers::updates::refresh_cache(&mut host_api) {
                Ok(_) => host::log::info("update check refreshed"),
                Err(err) => host::log::error(format!("update check failed: {err}")),
            },
            other => host::log::error(format!("unknown scheduled task {other}")),
        }
        Ok(scheduler::HandleScheduledTaskResponse {})
    })
}
