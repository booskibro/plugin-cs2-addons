//! DELETE /servers/{id}/plugins — drop the plugin folder and its manifest entry.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;

use crate::handlers::ctx::{self, ServerCtx};
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{RemovePluginRequest, RemovePluginResponse};

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    query_params: &HashMap<String, pb::QueryParamValues>,
    actor: Option<&str>,
) -> ApiResult {
    let context = ServerCtx::resolve(host, params)?;

    // DELETE bodies are dropped by some proxies — accept ?name= as a fallback.
    let name = if body.is_empty() {
        query_params
            .get("name")
            .and_then(|values| values.values.first())
            .cloned()
            .ok_or_else(|| ApiError::bad_request("name is required (body or ?name=)"))?
    } else {
        parse_json_body::<RemovePluginRequest>(body)?.name
    };
    ctx::sanitize_plugin_name(&name)?;

    super::require_css_installed(host, &context)?;

    let folder = super::find_plugin_folder(host, &context, &name)?;
    let mut manifest = super::read_manifest(host, &context)?;
    if folder.is_none() && !manifest.contains(&name) {
        return Err(ApiError::not_found(
            "PLUGIN_NOT_FOUND",
            format!("no plugin named {name} on the server"),
        ));
    }

    // Keep configs (configs/plugins/<name>); a failed folder delete degrades
    // to folder_deleted=false rather than blocking the manifest cleanup.
    let folder_deleted = match &folder {
        Some((folder_abs, _)) => match host.remove(context.node_id, folder_abs, true) {
            Ok(()) => true,
            Err(err) => {
                host.log_error(&format!("cs2-addons: failed to delete {folder_abs}: {err:?}"));
                false
            }
        },
        None => false,
    };

    let entry_removed = manifest.remove(&name);
    if entry_removed {
        super::write_manifest(host, &context, &manifest)?;
    }

    super::audit::record(host, context.server_id, actor, "plugin-delete", &name);

    Ok(json_response(
        200,
        &RemovePluginResponse {
            name,
            folder_deleted,
            entry_removed,
        },
    ))
}
