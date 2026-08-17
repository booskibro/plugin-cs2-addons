//! POST /servers/{id}/restart — restart the game server via the panel's
//! server-control host service, so pending plugin toggles take effect.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiResult, json_response};
use crate::model::RestartResponse;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    host.restart_server(ctx.server_id)?;
    super::audit::record(host, ctx.server_id, actor, "server-restart", "server");
    Ok(json_response(200, &RestartResponse { restarted: true }))
}
