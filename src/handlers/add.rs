//! POST /servers/{id}/plugins — register an uploaded plugin folder.
//!
//! The frontend uploads `plugins/<Name>/<Name>.dll` (and any sibling files)
//! through the panel file manager first; this route validates the layout and
//! records the plugin in plugins_meta.json so it survives folder deletion as
//! a "Missing" entry, mirroring the goldsrc plugins.ini registration.

use std::collections::HashMap;

use crate::handlers::ctx::{self, ServerCtx};
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{AddPluginRequest, AddPluginResponse};
use crate::source2::paths;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let context = ServerCtx::resolve(host, params)?;
    let request: AddPluginRequest = parse_json_body(body)?;
    ctx::sanitize_plugin_name(&request.name)?;

    super::require_css_installed(host, &context)?;

    let folder_abs = paths::join(&super::css_plugins_abs(&context), &request.name);
    let dll_abs = paths::join(&folder_abs, &format!("{}.dll", request.name));
    if !host.stat(context.node_id, &dll_abs)?.is_some_and(|s| !s.is_dir) {
        return Err(ApiError::unprocessable(
            "FILE_NOT_UPLOADED",
            format!(
                "{}/{}.dll was not found; upload the plugin first",
                request.name, request.name
            ),
        ));
    }

    let mut manifest = super::read_manifest(host, &context)?;
    let replaced = manifest.contains(&request.name);
    if replaced && !request.force {
        return Err(ApiError::conflict(
            "ALREADY_REGISTERED",
            format!("{} is already registered", request.name),
        ));
    }
    manifest.ensure(&request.name);
    super::write_manifest(host, &context, &manifest)?;

    super::audit::record(
        host,
        context.server_id,
        actor,
        if replaced { "plugin-update" } else { "plugin-install" },
        &request.name,
    );

    Ok(json_response(
        if replaced { 200 } else { 201 },
        &AddPluginResponse {
            name: request.name,
            replaced,
        },
    ))
}
