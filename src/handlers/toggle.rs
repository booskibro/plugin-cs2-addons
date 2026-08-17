//! POST /servers/{id}/plugins/toggle — move a plugin folder in or out of
//! plugins/disabled/. CS2 has no plugins.ini: parking the folder where
//! CounterStrikeSharp does not scan is the persistent disable.

use std::collections::HashMap;

use crate::handlers::ctx::{self, ServerCtx};
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{ToggleRequest, ToggleResponse};
use crate::source2::paths;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let context = ServerCtx::resolve(host, params)?;
    let request: ToggleRequest = parse_json_body(body)?;
    ctx::sanitize_plugin_name(&request.name)?;

    super::require_css_installed(host, &context)?;

    let Some((current_abs, currently_enabled)) =
        super::find_plugin_folder(host, &context, &request.name)?
    else {
        return Err(ApiError::not_found(
            "PLUGIN_NOT_FOUND",
            format!("no plugin folder named {} on the server", request.name),
        ));
    };

    if currently_enabled == request.enabled {
        return Ok(json_response(
            200,
            &ToggleResponse {
                name: request.name,
                enabled: request.enabled,
                changed: false,
            },
        ));
    }

    let folder_name = paths::file_name(&current_abs).to_string();
    let target_abs = if request.enabled {
        paths::join(&super::css_plugins_abs(&context), &folder_name)
    } else {
        paths::join(&super::css_disabled_abs(&context), &folder_name)
    };

    if host.stat(context.node_id, &target_abs)?.is_some() {
        return Err(ApiError::conflict(
            "TARGET_EXISTS",
            format!("{target_abs} already exists; refusing to overwrite"),
        ));
    }

    super::move_dir(host, &context, &current_abs, &target_abs)?;

    super::audit::record(
        host,
        context.server_id,
        actor,
        if request.enabled { "plugin-enable" } else { "plugin-disable" },
        &request.name,
    );

    Ok(json_response(
        200,
        &ToggleResponse {
            name: request.name,
            enabled: request.enabled,
            changed: true,
        },
    ))
}
