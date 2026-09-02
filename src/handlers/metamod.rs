//! POST /servers/{id}/metamod/toggle — enable/disable a binary Metamod plugin
//! by renaming its addons/metamod/<name>.vdf alias file.

use std::collections::HashMap;

use crate::handlers::ctx::{self, ServerCtx};
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{MetamodToggleRequest, MetamodToggleResponse};
use crate::source2::{self, paths, vdf};

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let context = ServerCtx::resolve(host, params)?;
    let request: MetamodToggleRequest = parse_json_body(body)?;
    ctx::sanitize_plugin_name(&request.name)?;

    // Disabling this one alias unloads CounterStrikeSharp itself - every CSS
    // plugin stops and `css_plugins` becomes an unknown console command. It is
    // still allowed, but never as an unremarkable row switch.
    if !request.enabled && !request.force && vdf::is_platform(&request.name) {
        return Err(ApiError::conflict(
            "PLATFORM_VDF",
            "counterstrikesharp.vdf registers CounterStrikeSharp itself; disabling it unloads every CSS plugin and the console commands this tab relies on",
        ));
    }

    let metamod_abs = paths::join(&context.game_abs, source2::METAMOD_DIR);
    let live_abs = paths::join(&metamod_abs, &format!("{}{}", request.name, vdf::VDF_EXT));
    let parked_abs = paths::join(
        &metamod_abs,
        &format!("{}{}", request.name, vdf::DISABLED_SUFFIX),
    );

    let live = host.stat(context.node_id, &live_abs)?.is_some_and(|s| !s.is_dir);
    let parked = host
        .stat(context.node_id, &parked_abs)?
        .is_some_and(|s| !s.is_dir);

    if !live && !parked {
        return Err(ApiError::not_found(
            "VDF_NOT_FOUND",
            format!("no {}.vdf in addons/metamod", request.name),
        ));
    }
    if live && parked {
        return Err(ApiError::conflict(
            "VDF_AMBIGUOUS",
            format!(
                "both {0}.vdf and {0}.vdf.disabled exist; delete one in the file manager",
                request.name
            ),
        ));
    }

    let changed = live != request.enabled;
    if changed {
        let (from, to) = if request.enabled {
            (&parked_abs, &live_abs)
        } else {
            (&live_abs, &parked_abs)
        };
        host.move_path(context.node_id, from, to)?;
        super::audit::record(
            host,
            context.server_id,
            actor,
            if request.enabled {
                "metamod-plugin-enable"
            } else {
                "metamod-plugin-disable"
            },
            &request.name,
        );
    }

    Ok(json_response(
        200,
        &MetamodToggleResponse {
            name: request.name,
            enabled: request.enabled,
            changed,
        },
    ))
}
