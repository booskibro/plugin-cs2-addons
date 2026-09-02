//! POST /servers/{id}/plugins/install-archive — install a plugin from a zip
//! the user uploaded via the panel file manager.
//!
//! The zip goes through the file manager (plugin HTTP bodies are capped at
//! 1MB, file-manager uploads are not); this route then pulls it off the node,
//! unpacks it in-wasm with the same layout detection the catalog uses, writes
//! the files, registers the folders, and deletes the archive.

use std::collections::HashMap;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::HostApi;
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{InstallArchiveRequest, InstallArchiveResponse};
use crate::source2::{archive, paths};

/// Archive size cap: the whole archive is inflated in wasm memory, and the
/// download that fetches it is a single nodefs call, so the panel's inline
/// limit binds it too. Deliberately the same number rather than a second
/// opinion - this gate stats the file first and refuses with a message saying
/// what to do instead, where the panel's refusal is a generic "file too large".
const MAX_ARCHIVE_BYTES: u64 = super::PANEL_MAX_INLINE_BYTES;

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: InstallArchiveRequest = parse_json_body(body)?;
    paths::sanitize_rel_path(&request.path).map_err(ApiError::bad_request)?;
    if !request.path.to_ascii_lowercase().ends_with(".zip") {
        return Err(ApiError::bad_request("a .zip archive is required"));
    }

    super::require_css_installed(host, &ctx)?;

    let archive_abs = paths::join(&ctx.root_abs, &request.path);
    let stat = host
        .stat(ctx.node_id, &archive_abs)?
        .filter(|s| !s.is_dir)
        .ok_or_else(|| {
            ApiError::not_found("ARCHIVE_NOT_FOUND", "uploaded archive not found on the server")
        })?;
    if stat.size > MAX_ARCHIVE_BYTES {
        let _ = host.remove(ctx.node_id, &archive_abs, false);
        return Err(ApiError::unprocessable(
            "ARCHIVE_TOO_LARGE",
            "archives over 32MB are not supported here; unpack via the file manager instead",
        ));
    }

    let bytes = host.download(ctx.node_id, &archive_abs)?;
    let result = install_bytes(host, &ctx, &bytes, request.force, actor);
    // The uploaded zip is scratch — except on a conflict, where the frontend
    // asks the user and retries with force against the same upload.
    if !matches!(&result, Err(err) if err.status == 409) {
        let _ = host.remove(ctx.node_id, &archive_abs, false);
    }
    let (folders, files_written) = result?;

    super::audit::record(
        host,
        ctx.server_id,
        actor,
        "plugin-install-zip",
        &folders.join(", "),
    );

    Ok(json_response(200, &InstallArchiveResponse { folders, files_written }))
}

fn install_bytes<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    bytes: &[u8],
    force: bool,
    actor: Option<&str>,
) -> Result<(Vec<String>, u32), ApiError> {
    let entries =
        archive::extract_zip(bytes).map_err(|err| ApiError::unprocessable("BAD_ARCHIVE", err))?;
    let root = archive::detect_install_root(&entries)
        .map_err(|err| ApiError::unprocessable("BAD_ARCHIVE", err))?;
    let folders = super::archive_plugin_folders(&entries, &root);

    if !force {
        let manifest = super::read_manifest(host, ctx)?;
        for folder in &folders {
            let exists = super::find_plugin_folder(host, ctx, folder)?.is_some()
                || manifest.contains(folder);
            if exists {
                return Err(ApiError::conflict(
                    "ALREADY_REGISTERED",
                    format!("{folder} is already installed"),
                ));
            }
        }
    }

    // Overwrites are reversible: snapshot first, best-effort.
    super::snapshots::try_auto_snapshot(host, ctx, actor, None);

    let files_written = super::write_archive_entries(host, ctx, &entries, &root)?;

    if !folders.is_empty() {
        let mut manifest = super::read_manifest(host, ctx)?;
        for folder in &folders {
            manifest.ensure(folder);
        }
        super::write_manifest(host, ctx, &manifest)?;
    }

    Ok((folders, files_written))
}
