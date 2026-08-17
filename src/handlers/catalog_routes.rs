//! GET  /servers/{id}/catalog          — the curated plugin catalog
//! POST /servers/{id}/catalog/install  — download a release and install it
//!
//! Small release zips travel through the panel's plugin-HTTP (10MB cap) and
//! are unpacked in-plugin, so nothing beyond the daemon is required on the
//! node. Layout quirks are handled by `archive::detect_install_root`.

use std::collections::{BTreeSet, HashMap};

use crate::handlers::ctx::ServerCtx;
use crate::host_api::{HostApi, HostApiError, HttpFetchParams};
use crate::http::{ApiError, ApiResult, json_response, parse_json_body};
use crate::model::{
    CatalogEntryInfo, CatalogInstallRequest, CatalogInstallResponse, CatalogResponse,
};
use crate::source2::archive::{self, InstallRoot};
use crate::source2::{catalog, paths};

const DOWNLOAD_TIMEOUT_SECONDS: i32 = 25;

pub fn handle_list<H: HostApi>(host: &mut H, params: &HashMap<String, String>) -> ApiResult {
    let _ctx = ServerCtx::resolve(host, params)?;
    let entries = catalog::CATALOG
        .iter()
        .map(|entry| CatalogEntryInfo {
            key: entry.key.to_string(),
            name: entry.name.to_string(),
            description: entry.description.to_string(),
            homepage: format!("https://github.com/{}", entry.repo),
            folder: entry.folder.to_string(),
        })
        .collect();
    Ok(json_response(200, &CatalogResponse { entries }))
}

pub fn handle_install<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    body: &[u8],
    actor: Option<&str>,
) -> ApiResult {
    let ctx = ServerCtx::resolve(host, params)?;
    let request: CatalogInstallRequest = parse_json_body(body)?;
    let entry = catalog::find(&request.key)
        .ok_or_else(|| ApiError::not_found("CATALOG_KEY_UNKNOWN", "unknown catalog entry"))?;

    super::require_css_installed(host, &ctx)?;

    let release = super::updates::fetch_github_latest(host, entry.repo, entry.asset_contains)
        .map_err(|err| ApiError::unprocessable("RELEASE_LOOKUP_FAILED", err))?;
    let download_url = release.download_url.clone().ok_or_else(|| {
        ApiError::unprocessable(
            "NO_MATCHING_ASSET",
            format!("latest {} release has no matching zip asset", entry.name),
        )
    })?;

    let resp = host
        .http_fetch(&HttpFetchParams {
            method: "GET".into(),
            url: download_url,
            headers: vec![("User-Agent".into(), "gameap-cs2-addons".into())],
            timeout_seconds: DOWNLOAD_TIMEOUT_SECONDS,
        })
        .map_err(|err| {
            ApiError::unprocessable(
                "DOWNLOAD_FAILED",
                format!(
                    "release download failed ({err:?}); note the panel caps plugin downloads at 10MB"
                ),
            )
        })?;
    if resp.status != 200 {
        return Err(ApiError::unprocessable(
            "DOWNLOAD_FAILED",
            format!("release download returned HTTP {}", resp.status),
        ));
    }

    let entries = archive::extract_zip(&resp.body)
        .map_err(|err| ApiError::unprocessable("BAD_ARCHIVE", err))?;
    let root = archive::detect_install_root(&entries)
        .map_err(|err| ApiError::unprocessable("BAD_ARCHIVE", err))?;

    let files_written = write_entries(host, &ctx, &entries, &root)?;

    // Register like a manual upload would, so the row appears with metadata.
    let mut manifest = super::read_manifest(host, &ctx)?;
    manifest.ensure(entry.folder);
    super::write_manifest(host, &ctx, &manifest)?;

    super::audit::record(
        host,
        ctx.server_id,
        actor,
        "catalog-install",
        &format!("{} {}", entry.name, release.version),
    );

    Ok(json_response(
        200,
        &CatalogInstallResponse {
            key: entry.key.to_string(),
            folder: entry.folder.to_string(),
            version: release.version,
            files_written,
        },
    ))
}

fn write_entries<H: HostApi>(
    host: &mut H,
    ctx: &ServerCtx,
    entries: &[archive::ArchiveEntry],
    root: &InstallRoot,
) -> Result<u32, ApiError> {
    let plugins_abs = super::css_plugins_abs(ctx);
    let mut written = 0u32;
    let mut ensured_dirs: BTreeSet<String> = BTreeSet::new();
    for entry in entries {
        let target_abs = match root {
            InstallRoot::GameDir => paths::join(&ctx.game_abs, &entry.path),
            InstallRoot::PluginsDir => paths::join(&plugins_abs, &entry.path),
            InstallRoot::WrapIntoFolder(folder) => {
                paths::join(&paths::join(&plugins_abs, folder), &entry.path)
            }
        };
        ensure_parent_dirs(host, ctx.node_id, &target_abs, &mut ensured_dirs)?;
        host.upload(ctx.node_id, &target_abs, &entry.data, entry.mode)?;
        written += 1;
    }
    Ok(written)
}

/// Creates every missing ancestor of `file_abs` (the daemon's mk_dir is not
/// guaranteed to be recursive). Already-known dirs are skipped via the set.
fn ensure_parent_dirs<H: HostApi>(
    host: &mut H,
    node_id: u64,
    file_abs: &str,
    ensured: &mut BTreeSet<String>,
) -> Result<(), HostApiError> {
    let Some(parent_end) = file_abs.rfind('/') else {
        return Ok(());
    };
    let parent = &file_abs[..parent_end];
    if ensured.contains(parent) {
        return Ok(());
    }
    // Walk down from the shortest missing ancestor.
    let mut prefixes: Vec<&str> = Vec::new();
    let mut idx = parent.len();
    loop {
        let candidate = &parent[..idx];
        if ensured.contains(candidate) || host.stat(node_id, candidate)?.is_some() {
            break;
        }
        prefixes.push(candidate);
        match candidate.rfind('/') {
            Some(next) if next > 0 => idx = next,
            _ => break,
        }
    }
    for candidate in prefixes.into_iter().rev() {
        host.mk_dir(node_id, candidate)?;
        ensured.insert(candidate.to_string());
    }
    ensured.insert(parent.to_string());
    Ok(())
}
