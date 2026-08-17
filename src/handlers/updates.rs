//! GET /servers/{id}/updates — latest upstream versions of Metamod:Source,
//! CounterStrikeSharp and the catalog plugins, cached in plugin storage.
//! `?refresh=1` forces a re-fetch; the nightly scheduled task does the same.

use std::collections::HashMap;

use gameap_plugin_sdk::proto::gameap::plugin as pb;

use crate::handlers::ctx::ServerCtx;
use crate::host_api::{HostApi, HttpFetchParams};
use crate::http::{ApiResult, json_response};
use crate::model::{PlatformRelease, PluginRelease, PluginUpdateInfo, UpdatesCache, UpdatesResponse};
use crate::source2::catalog;

const CACHE_KEY: &str = "updates:v1";
const CACHE_TTL_SECONDS: u64 = 6 * 60 * 60;
const FETCH_TIMEOUT_SECONDS: i32 = 20;

const MMSOURCE_BASE: &str = "https://mms.alliedmods.net/mmsdrop/2.0/";
const MMSOURCE_LATEST: &str = "https://mms.alliedmods.net/mmsdrop/2.0/mmsource-latest-linux";
pub const CSS_REPO: &str = "roflmuffin/CounterStrikeSharp";
pub const CSS_ASSET_PATTERNS: &[&str] = &["with-runtime", "linux"];

pub fn handle<H: HostApi>(
    host: &mut H,
    params: &HashMap<String, String>,
    query: &HashMap<String, pb::QueryParamValues>,
) -> ApiResult {
    let _ctx = ServerCtx::resolve(host, params)?;

    let force = query
        .get("refresh")
        .and_then(|v| v.values.first())
        .is_some_and(|v| v == "1" || v == "true");

    let cached = load_cache(host);
    let now = super::now_unix();
    let cache = match cached {
        Some(cache) if !force && now.saturating_sub(cache.fetched_at) < CACHE_TTL_SECONDS => cache,
        stale => match refresh_cache(host) {
            Ok(fresh) => fresh,
            Err(err) => {
                host.log_error(&format!("updates refresh failed: {err}"));
                stale.unwrap_or_default()
            }
        },
    };

    let plugins = cache
        .plugins
        .iter()
        .filter_map(|(key, release)| {
            catalog::find(key).map(|entry| PluginUpdateInfo {
                key: entry.key.to_string(),
                folder: entry.folder.to_string(),
                version: release.version.clone(),
                release_url: release.release_url.clone(),
            })
        })
        .collect();

    Ok(json_response(
        200,
        &UpdatesResponse {
            fetched_at: cache.fetched_at,
            stale: super::now_unix().saturating_sub(cache.fetched_at) >= CACHE_TTL_SECONDS,
            metamod: cache.metamod,
            css: cache.css,
            plugins,
        },
    ))
}

fn load_cache<H: HostApi>(host: &mut H) -> Option<UpdatesCache> {
    match host.storage_get(CACHE_KEY) {
        Ok(Some(bytes)) => serde_json::from_slice(&bytes).ok(),
        _ => None,
    }
}

/// Fetches everything and stores the cache. Individual sources may fail
/// without failing the sweep — a missing entry simply stays unknown.
pub fn refresh_cache<H: HostApi>(host: &mut H) -> Result<UpdatesCache, String> {
    let mut cache = UpdatesCache {
        fetched_at: super::now_unix(),
        metamod: None,
        css: None,
        plugins: Default::default(),
    };

    match fetch_metamod_latest(host) {
        Ok(release) => cache.metamod = Some(release),
        Err(err) => host.log_error(&format!("metamod version check failed: {err}")),
    }

    match fetch_github_latest(host, CSS_REPO, CSS_ASSET_PATTERNS) {
        Ok(release) => {
            cache.css = Some(PlatformRelease {
                version: release.version.clone(),
                download_url: release.download_url.clone().unwrap_or_default(),
            });
        }
        Err(err) => host.log_error(&format!("counterstrikesharp version check failed: {err}")),
    }

    for entry in catalog::CATALOG {
        match fetch_github_latest(host, entry.repo, entry.asset_contains) {
            Ok(release) => {
                cache.plugins.insert(entry.key.to_string(), release);
            }
            Err(err) => host.log_error(&format!("{} version check failed: {err}", entry.key)),
        }
    }

    let bytes = serde_json::to_vec(&cache).map_err(|e| e.to_string())?;
    host.storage_set(CACHE_KEY, &bytes)
        .map_err(|e| format!("{e:?}"))?;
    Ok(cache)
}

/// mmsource-latest-linux is a text file holding the current build's file name.
pub fn fetch_metamod_latest<H: HostApi>(host: &mut H) -> Result<PlatformRelease, String> {
    let resp = host
        .http_fetch(&HttpFetchParams {
            method: "GET".into(),
            url: MMSOURCE_LATEST.into(),
            headers: default_headers(),
            timeout_seconds: FETCH_TIMEOUT_SECONDS,
        })
        .map_err(|e| format!("{e:?}"))?;
    if resp.status != 200 {
        return Err(format!("mmsource-latest-linux returned HTTP {}", resp.status));
    }
    let file_name = String::from_utf8_lossy(&resp.body).trim().to_string();
    if !file_name.starts_with("mmsource-") || file_name.contains('/') {
        return Err(format!("unexpected latest-build answer: {file_name:?}"));
    }
    // mmsource-2.0.0-git1359-linux.tar.gz → "2.0.0-git1359"
    let version = file_name
        .trim_start_matches("mmsource-")
        .trim_end_matches(".tar.gz")
        .trim_end_matches("-linux")
        .to_string();
    Ok(PlatformRelease {
        version,
        download_url: format!("{MMSOURCE_BASE}{file_name}"),
    })
}

/// GitHub "latest release" lookup, picking the first asset whose lowercase
/// name contains every pattern.
pub fn fetch_github_latest<H: HostApi>(
    host: &mut H,
    repo: &str,
    asset_contains: &[&str],
) -> Result<PluginRelease, String> {
    let resp = host
        .http_fetch(&HttpFetchParams {
            method: "GET".into(),
            url: format!("https://api.github.com/repos/{repo}/releases/latest"),
            headers: default_headers(),
            timeout_seconds: FETCH_TIMEOUT_SECONDS,
        })
        .map_err(|e| format!("{e:?}"))?;
    if resp.status != 200 {
        return Err(format!("github api returned HTTP {}", resp.status));
    }
    let release: serde_json::Value =
        serde_json::from_slice(&resp.body).map_err(|e| format!("bad github response: {e}"))?;
    let version = release["tag_name"]
        .as_str()
        .ok_or("release has no tag_name")?
        .trim_start_matches('v')
        .to_string();
    let release_url = release["html_url"].as_str().unwrap_or_default().to_string();
    let download_url = release["assets"].as_array().and_then(|assets| {
        assets.iter().find_map(|asset| {
            let name = asset["name"].as_str()?.to_ascii_lowercase();
            asset_contains
                .iter()
                .all(|pattern| name.contains(pattern))
                .then(|| asset["browser_download_url"].as_str().map(str::to_string))
                .flatten()
        })
    });
    Ok(PluginRelease {
        version,
        release_url,
        download_url,
    })
}

fn default_headers() -> Vec<(String, String)> {
    vec![
        // GitHub rejects requests without a User-Agent.
        ("User-Agent".into(), "gameap-cs2-addons".into()),
        ("Accept".into(), "application/vnd.github+json".into()),
    ]
}
