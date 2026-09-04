//! GameAP plugin: manage Metamod:Source and CounterStrikeSharp on CS2 servers.
//!
//! Backend of the "Plugins" server tab: assembles the Metamod:Source and
//! CounterStrikeSharp picture of a Counter-Strike 2 server and performs plugin
//! folder moves and `plugins_meta.json` mutations through the nodefs/nodecmd
//! host libraries. The Vue frontend (embedded bundle) talks to these routes and
//! to existing panel endpoints (RCON for live statuses, file-manager for
//! uploads and configs).
//!
//! A CS2 re-imagining of gameap/plugin-goldsrc-addons.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

pub mod consistency;
pub mod handlers;
pub mod host_api;
pub mod http;
pub mod maintenance;
pub mod model;
pub mod rcon;
pub mod router;
pub mod source2;
pub mod wasm_ext;

use gameap_plugin_sdk::proto::gameap::plugin as pb;
use gameap_plugin_sdk::{Plugin, PluginError, register_plugin};

use crate::host_api::HostApi;

// The panel normalizes plugin ids (CompactPluginID): the id must decode as
// base32 (alphabet a-z2-7, no padding — so 2/4/5/7/8/10/12/13 chars) and
// re-encode to itself, or it is rewritten to an FNV hash, which breaks route
// paths and the plugin:<id>:manage ability the tab is gated on.
// "mnzteylemrxw4" is base32("cs2addon") and is round-trip stable.
pub const PLUGIN_ID: &str = "mnzteylemrxw4";

/// The GameAP game code this plugin is for.
///
/// Written in three places that cannot import one another: the auto-repair
/// sweep's server lookup, the RCON protocol's `game_codes`, and the frontend
/// tab's `checkGame.codes`. Both Rust copies read this const; the frontend one
/// is pinned to it by `consistency::game_code_matches_the_frontend_tab_gate`.
pub const GAME_CODE: &str = "cs2";

/// The grants this plugin needs, as GameAP 4.5 names them.
///
/// The panel derives what a plugin *uses* from its wasm import section
/// (internal/plugin/permissions.go, against hostlibrary's policy table) and
/// flags anything used-but-not-declared as a missing grant in the admin UI and
/// in the upload dry-run. Each entry below is the narrowest grant covering a
/// host call this plugin actually makes:
///
/// - `node_commands` - nodecmd.execute_command
/// - `files` - nodefs mk_dir, move, remove, upload, chmod. It also covers `files_read`, which is all the
///   reads (read_dir, download, get_file_info) would need on their own:
///   the panel treats a broader grant as satisfying a narrower one, and its own
///   derivation drops the narrower one, so declaring both would disagree with
///   what the admin UI reports as used.
/// - `manage_servers` - servercontrol.restart_server
///
/// Deliberately absent: gameap-nodes get_node and gameap-servers
/// find_servers/get_server are read-only and ungated; gameap-http, gameap-storage,
/// gameap-scheduler, gameap-net, gameap-games and gameap-log are not gated at
/// all; and `listen_events` is for a plugin that subscribes to events, which
/// this one does not.
///
/// Nothing here is enforced yet - PLUGINS_PERMISSIONS_ENFORCE defaults to false
/// in 4.5, and a panel older than 4.5 ignores the field entirely - but a later
/// release turns it on, and an undeclared plugin is refused then.
pub const REQUIRED_PERMISSIONS: [&str; 3] = ["node_commands", "files", "manage_servers"];

const FRONTEND_JS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.js"));
const FRONTEND_CSS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/plugin.css"));

pub struct Cs2Addons<H> {
    host: H,
}

impl<H> Cs2Addons<H> {
    pub fn new(host: H) -> Self {
        Self { host }
    }
}

impl<H: HostApi> Plugin for Cs2Addons<H> {
    fn get_info(&mut self, _req: pb::GetInfoRequest) -> Result<pb::PluginInfo, PluginError> {
        Ok(pb::PluginInfo {
            id: PLUGIN_ID.into(),
            name: "CS2 Addons".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Manage Metamod:Source and CounterStrikeSharp plugins on Counter-Strike 2 servers"
                .into(),
            author: "SilverSasquatchGameAPDev".into(),
            license: "MIT".into(),
            api_version: "1".into(),
            required_permissions: REQUIRED_PERMISSIONS
                .iter()
                .map(|permission| (*permission).to_string())
                .collect(),
            ..Default::default()
        })
    }

    fn initialize(
        &mut self,
        _req: pb::InitializeRequest,
    ) -> Result<pb::InitializeResponse, PluginError> {
        self.host.log_info("cs2-addons plugin initialized");
        #[cfg(target_arch = "wasm32")]
        crate::wasm_ext::register_scheduled_tasks();
        Ok(pb::InitializeResponse {
            result: Some(gameap_plugin_sdk::ok_result()),
        })
    }

    fn get_http_routes(
        &mut self,
        _req: pb::GetHttpRoutesRequest,
    ) -> Result<pb::GetHttpRoutesResponse, PluginError> {
        Ok(pb::GetHttpRoutesResponse {
            routes: router::http_routes(),
        })
    }

    fn handle_http_request(
        &mut self,
        req: pb::HttpRequest,
    ) -> Result<pb::HttpResponse, PluginError> {
        // Total dispatch: every failure becomes a JSON error response. An Err
        // here would surface as a plain-text host 500.
        Ok(router::dispatch(&mut self.host, &req))
    }

    fn get_server_abilities(
        &mut self,
        _req: pb::GetServerAbilitiesRequest,
    ) -> Result<pb::GetServerAbilitiesResponse, PluginError> {
        // Admins get plugin abilities automatically; the frontend tab is
        // gated on plugin:mnzteylemrxw4:manage.
        Ok(pb::GetServerAbilitiesResponse {
            abilities: vec![pb::ServerAbility {
                name: "manage".into(),
                title: "Manage CS2 addons (Metamod:Source / CounterStrikeSharp)".into(),
            }],
        })
    }

    fn get_frontend_bundle(
        &mut self,
        _req: pb::GetFrontendBundleRequest,
    ) -> Result<pb::GetFrontendBundleResponse, PluginError> {
        Ok(pb::GetFrontendBundleResponse {
            bundle: FRONTEND_JS.to_vec(),
            has_bundle: !FRONTEND_JS.is_empty(),
            styles: FRONTEND_CSS.to_vec(),
            has_styles: !FRONTEND_CSS.is_empty(),
        })
    }
}

register_plugin!(
    Cs2Addons<host_api::WasmHost>,
    Cs2Addons::new(host_api::WasmHost)
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_api::mock::MockHost;

    /// Every permission GameAP 4.5 understands (internal/domain/plugin.go,
    /// PluginPermissions).
    const PANEL_PERMISSIONS: [&str; 12] = [
        "manage_servers",
        "manage_nodes",
        "manage_games",
        "manage_game_mods",
        "manage_users",
        "manage_rbac",
        "files",
        "files_read",
        "listen_events",
        "secrets",
        "node_commands",
        "ssh",
    ];

    #[test]
    fn get_info_reports_the_declared_permissions() {
        let mut plugin = Cs2Addons::new(MockHost::default());
        let info = plugin
            .get_info(pb::GetInfoRequest::default())
            .expect("get_info");

        let declared: Vec<&str> = info
            .required_permissions
            .iter()
            .map(String::as_str)
            .collect();

        assert_eq!(declared, REQUIRED_PERMISSIONS);
    }

    #[test]
    fn every_declared_permission_is_one_the_panel_knows() {
        // ParsePluginPermissions silently DROPS names it does not recognize, so
        // a typo would not fail the install - it would quietly grant less than
        // intended, and only show up as a denied host call once enforcement is
        // on. This is the check that turns that into a build failure.
        for permission in REQUIRED_PERMISSIONS {
            assert!(
                PANEL_PERMISSIONS.contains(&permission),
                "{permission} is not a permission GameAP understands"
            );
        }
    }

    #[test]
    fn declares_nothing_twice_and_nothing_another_grant_covers() {
        let mut seen = REQUIRED_PERMISSIONS.to_vec();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            REQUIRED_PERMISSIONS.len(),
            "the same permission is declared more than once"
        );

        assert!(
            !(REQUIRED_PERMISSIONS.contains(&"files")
                && REQUIRED_PERMISSIONS.contains(&"files_read")),
            "files already covers files_read; declaring both disagrees with the panel"
        );
    }
}
