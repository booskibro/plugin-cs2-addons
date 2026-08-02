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
