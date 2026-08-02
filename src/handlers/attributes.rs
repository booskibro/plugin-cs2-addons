//! POST /servers/{id}/plugins/attributes — set a plugin's comment and group
//! in plugins_meta.json (the shared AddonsManager manifest).

use std::collections::HashMap;

use crate::handlers::ctx::{self, ServerCtx};
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{SetAttributesRequest, SetAttributesResponse};

const MAX_COMMENT_LEN: usize = 200;
const MAX_GROUP_LEN: usize = 64;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
) -> ApiResult {
    let context = ServerCtx::resolve(host, params)?;
    let request: SetAttributesRequest = parse_json_body(body)?;
    ctx::sanitize_plugin_name(&request.name)?;

    super::require_css_installed(host, &context)?;

    let comment = normalize_text(request.comment, MAX_COMMENT_LEN, "comment")?;
    let group = normalize_text(request.group, MAX_GROUP_LEN, "group")?;

    // The plugin must exist somewhere the panel knows about: as a folder
    // (enabled or disabled) or as an existing manifest entry.
    let mut manifest = super::read_manifest(host, &context)?;
    let known = manifest.contains(&request.name)
        || super::find_plugin_folder(host, &context, &request.name)?.is_some();
    if !known {
        return Err(ApiError::not_found(
            "PLUGIN_NOT_FOUND",
            format!("no plugin named {} on the server", request.name),
        ));
    }

    let changed = manifest.comment(&request.name) != comment
        || manifest.group(&request.name) != group;
    if changed {
        manifest.set_comment(&request.name, comment.as_deref());
        manifest.set_group(&request.name, group.as_deref());
        super::write_manifest(host, &context, &manifest)?;
    }

    Ok(json_response(
        200,
        &SetAttributesResponse {
            name: request.name,
            comment,
            group,
            changed,
        },
    ))
}

/// Validates and normalizes incoming free text: rejects control characters,
/// trims, and enforces a length cap. Empty result becomes `None`.
fn normalize_text(
    value: Option<String>,
    max_len: usize,
    what: &str,
) -> Result<Option<String>, ApiError> {
    let Some(raw) = value else {
        return Ok(None);
    };
    if raw.chars().any(char::is_control) {
        return Err(ApiError::bad_request(format!(
            "{what} must not contain control characters"
        )));
    }
    let text = raw.trim();
    if text.chars().count() > max_len {
        return Err(ApiError::bad_request(format!("{what} is too long")));
    }
    Ok((!text.is_empty()).then(|| text.to_string()))
}
