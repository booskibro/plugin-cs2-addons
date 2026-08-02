pub mod gamedir;
pub mod gameinfo;
pub mod manifest;
pub mod paths;

/// CounterStrikeSharp layout relative to the game dir (e.g. game/csgo).
pub const CSS_DIR: &str = "addons/counterstrikesharp";
pub const CSS_PLUGINS_DIR: &str = "addons/counterstrikesharp/plugins";
pub const CSS_CONFIGS_DIR: &str = "addons/counterstrikesharp/configs/plugins";
/// Folder inside the plugins dir where disabled plugins are parked. CSS only
/// loads `plugins/<Name>/<Name>.dll`, so a nested folder is skipped — the CS2
/// stand-in for commenting a plugins.ini line out.
pub const DISABLED_DIR_NAME: &str = "disabled";
/// Shared per-plugin annotations manifest. The same file the AddonsManager
/// CounterStrikeSharp plugin maintains in game, so comments and groups set in
/// the panel and in the server console stay in sync.
pub const META_MANIFEST: &str =
    "addons/counterstrikesharp/configs/plugins/AddonsManager/plugins_meta.json";
pub const METAMOD_DIR: &str = "addons/metamod";
pub const GAMEINFO_FILE: &str = "gameinfo.gi";
