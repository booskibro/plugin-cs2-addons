//! JSON request/response DTOs of the plugin API.

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct ToggleRequest {
    /// Plugin folder name inside addons/counterstrikesharp/plugins.
    pub name: String,
    pub enabled: bool,
}

#[derive(Deserialize, Debug)]
pub struct AddPluginRequest {
    /// Plugin folder name; `<name>/<name>.dll` must already be uploaded.
    pub name: String,
    /// Re-register an already known plugin instead of failing with 409.
    #[serde(default)]
    pub force: bool,
}

#[derive(Deserialize, Debug)]
pub struct RemovePluginRequest {
    pub name: String,
}

#[derive(Serialize, Debug)]
pub struct StateResponse {
    pub server_id: u64,
    pub game_code: String,
    pub engine: String,
    pub engine_version: String,
    /// Server-dir-relative game directory, e.g. "game/csgo".
    pub game_dir: String,
    pub paths: StatePaths,
    pub metamod: MetamodState,
    pub css: CssState,
}

/// All paths are relative to the server directory, ready to be passed to the
/// panel file-manager API as-is.
#[derive(Serialize, Debug)]
pub struct StatePaths {
    pub gameinfo: String,
    pub metamod_dir: String,
    pub css_dir: String,
    pub css_plugins_dir: String,
    pub css_disabled_dir: String,
    pub css_configs_dir: String,
    pub meta_manifest: String,
}

#[derive(Serialize, Debug)]
pub struct MetamodState {
    /// addons/metamod exists and gameinfo.gi loads it.
    pub installed: bool,
    /// The addons/metamod directory exists.
    pub dir_present: bool,
    /// gameinfo.gi contains the csgo/addons/metamod search path.
    pub gameinfo_wired: bool,
    /// Binary Metamod plugins registered via addons/metamod/*.vdf.
    pub plugins: Vec<MetamodPluginEntry>,
}

#[derive(Serialize, Debug)]
pub struct MetamodPluginEntry {
    /// VDF file stem, e.g. "counterstrikesharp" for counterstrikesharp.vdf.
    pub name: String,
    /// The .vdf is live (not renamed to .vdf.disabled).
    pub enabled: bool,
    /// This alias is CounterStrikeSharp itself, not a plugin: switching it off
    /// unloads the platform. The frontend confirms before touching it.
    pub platform: bool,
}

#[derive(Serialize, Debug)]
pub struct CssState {
    /// addons/counterstrikesharp exists.
    pub installed: bool,
    pub plugins: Vec<CssPluginEntry>,
}

#[derive(Serialize, Debug)]
pub struct CssPluginEntry {
    /// Plugin folder name — the stable identity of a CounterStrikeSharp plugin.
    pub name: String,
    /// The folder is in plugins/ (not parked in plugins/disabled/).
    pub enabled: bool,
    /// `<name>/<name>.dll` is missing (folder broken), or — for manifest-only
    /// entries — the whole folder is gone.
    pub missing: bool,
    /// Free-text note from plugins_meta.json.
    pub comment: Option<String>,
    /// Group name from plugins_meta.json.
    pub group: Option<String>,
    pub has_config: bool,
    /// Server-dir-relative path of `configs/plugins/<name>/<name>.json`, when present.
    pub config_path: Option<String>,
    /// Index of the display group; ungrouped entries share one trailing "Other".
    pub group_index: u32,
    /// Header of the display group, `None` for the common "Other" group.
    pub group_title: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ToggleResponse {
    pub name: String,
    pub enabled: bool,
    pub changed: bool,
}

#[derive(Serialize, Debug)]
pub struct AddPluginResponse {
    pub name: String,
    /// True when the plugin was already known (force re-register).
    pub replaced: bool,
}

#[derive(Serialize, Debug)]
pub struct RemovePluginResponse {
    pub name: String,
    pub folder_deleted: bool,
    pub entry_removed: bool,
}

#[derive(Deserialize, Debug)]
pub struct SetAttributesRequest {
    pub name: String,
    /// Full desired comment; `null`/absent clears it.
    #[serde(default)]
    pub comment: Option<String>,
    /// Full desired group; `null`/absent clears it.
    #[serde(default)]
    pub group: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct SetAttributesResponse {
    pub name: String,
    pub comment: Option<String>,
    pub group: Option<String>,
    pub changed: bool,
}

#[derive(Serialize, Debug)]
pub struct RepairGameinfoResponse {
    /// False when gameinfo.gi was already wired and nothing was written.
    pub changed: bool,
}

#[derive(Deserialize, Debug)]
pub struct MetamodToggleRequest {
    /// VDF stem inside addons/metamod (no extension).
    pub name: String,
    pub enabled: bool,
    /// Required to disable the CounterStrikeSharp alias, which is a
    /// platform-wide off switch rather than a per-plugin one.
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, Debug)]
pub struct MetamodToggleResponse {
    pub name: String,
    pub enabled: bool,
    pub changed: bool,
}

#[derive(Serialize, Debug)]
pub struct LogsResponse {
    /// Server-dir-relative path of the log file that was read, if any.
    pub file: Option<String>,
    pub lines: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct RestartResponse {
    pub restarted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct UpdatesCache {
    /// Unix seconds of the last successful refresh.
    pub fetched_at: u64,
    pub metamod: Option<PlatformRelease>,
    pub css: Option<PlatformRelease>,
    /// Catalog key → latest known release.
    pub plugins: std::collections::BTreeMap<String, PluginRelease>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlatformRelease {
    pub version: String,
    pub download_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginRelease {
    pub version: String,
    pub release_url: String,
    pub download_url: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct UpdatesResponse {
    pub fetched_at: u64,
    pub stale: bool,
    pub metamod: Option<PlatformRelease>,
    pub css: Option<PlatformRelease>,
    /// Catalog entries with their latest release, keyed for row matching:
    /// entry.folder is the CSS plugin folder name the release belongs to.
    pub plugins: Vec<PluginUpdateInfo>,
}

#[derive(Serialize, Debug)]
pub struct PluginUpdateInfo {
    pub key: String,
    pub folder: String,
    pub version: String,
    pub release_url: String,
}

#[derive(Serialize, Debug)]
pub struct CatalogResponse {
    pub entries: Vec<CatalogEntryInfo>,
}

#[derive(Serialize, Debug)]
pub struct CatalogEntryInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub homepage: String,
    /// CSS plugin folder the install creates (row identity after install).
    pub folder: String,
}

#[derive(Deserialize, Debug)]
pub struct CatalogInstallRequest {
    pub key: String,
}

#[derive(Serialize, Debug)]
pub struct CatalogInstallResponse {
    pub key: String,
    pub folder: String,
    pub version: String,
    pub files_written: u32,
}

#[derive(Deserialize, Debug)]
pub struct PlatformInstallRequest {
    /// "metamod" or "css".
    pub kind: String,
}

#[derive(Serialize, Debug)]
pub struct PlatformInstallResponse {
    pub kind: String,
    pub version: String,
    /// True when gameinfo.gi was patched as part of the install.
    pub gameinfo_patched: bool,
}

#[derive(Serialize, Debug)]
pub struct SnapshotInfo {
    pub name: String,
    /// Unix seconds parsed from the snapshot name.
    pub created_at: u64,
    pub size: u64,
    /// Server-dir-relative path (for file-manager download).
    pub path: String,
}

#[derive(Serialize, Debug)]
pub struct SnapshotListResponse {
    pub snapshots: Vec<SnapshotInfo>,
}

#[derive(Deserialize, Debug)]
pub struct SnapshotNameRequest {
    pub name: String,
}

#[derive(Serialize, Debug)]
pub struct SnapshotCreateResponse {
    pub snapshot: SnapshotInfo,
    /// Older snapshots deleted to honor the retention cap.
    pub pruned: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct SnapshotRestoreResponse {
    pub name: String,
    pub restored: bool,
}

#[derive(Deserialize, Debug)]
pub struct InstallArchiveRequest {
    /// Server-dir-relative path of the uploaded .zip.
    pub path: String,
    /// Overwrite already-installed plugins instead of failing with 409.
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, Debug)]
pub struct InstallArchiveResponse {
    /// CSS plugin folders the archive created/updated.
    pub folders: Vec<String>,
    pub files_written: u32,
}

#[derive(Serialize, Debug)]
pub struct DoctorCheck {
    pub id: String,
    /// "ok" | "warn" | "fail".
    pub status: String,
    pub detail: String,
}

#[derive(Serialize, Debug)]
pub struct DoctorResponse {
    pub checks: Vec<DoctorCheck>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditEntry {
    /// Unix seconds.
    pub ts: u64,
    pub user: String,
    pub action: String,
    pub subject: String,
}

#[derive(Serialize, Debug)]
pub struct AuditResponse {
    pub entries: Vec<AuditEntry>,
}
