//! Native handler tests against the MockHost fake node.

use std::collections::HashMap;

use serde_json::Value;

use crate::host_api::mock::MockHost;
use crate::http::ApiError;
use crate::router;

fn params(id: &str) -> HashMap<String, String> {
    HashMap::from([("id".to_string(), id.to_string())])
}

fn body_json(result: Result<gameap_plugin_sdk::proto::gameap::plugin::HttpResponse, ApiError>)
-> (i32, Value) {
    let resp = result.unwrap_or_else(|err| err.into_response());
    let value: Value = serde_json::from_slice(&resp.body).expect("json body");
    (resp.status_code, value)
}

const GAME: &str = MockHost::GAME_ABS;

fn with_css(host: &mut MockHost) {
    host.add_dir(&format!("{GAME}/addons/counterstrikesharp"));
    host.add_dir(&format!("{GAME}/addons/counterstrikesharp/plugins"));
}

fn add_plugin(host: &mut MockHost, name: &str) {
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/plugins/{name}/{name}.dll"),
        b"MZ",
    );
}

fn add_disabled_plugin(host: &mut MockHost, name: &str) {
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/plugins/disabled/{name}/{name}.dll"),
        b"MZ",
    );
}

const MANIFEST_ABS: &str = "/srv/gameap/servers/cs2/game/csgo/addons/counterstrikesharp/configs/plugins/AddonsManager/plugins_meta.json";

// ---------------------------------------------------------------- state

#[test]
fn state_assembles_platforms_and_plugins() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_dir(&format!("{GAME}/addons/metamod"));
    host.add_file(
        &format!("{GAME}/gameinfo.gi"),
        b"SearchPaths { Game csgo/addons/metamod\nGame csgo }",
    );
    add_plugin(&mut host, "MatchZy");
    add_plugin(&mut host, "WeaponPaints");
    add_disabled_plugin(&mut host, "RollTheDice");
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/configs/plugins/MatchZy/MatchZy.json"),
        b"{}",
    );
    host.add_file(
        MANIFEST_ABS,
        br#"{"MatchZy": {"Comment": "tournament", "Group": "match"},
             "GoneMod": {"Comment": "", "Group": ""}}"#,
    );

    let (status, body) = body_json(crate::handlers::state::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    assert_eq!(body["engine"], "source");
    assert_eq!(body["game_dir"], "game/csgo");
    assert_eq!(body["metamod"]["installed"], true);
    assert_eq!(body["metamod"]["dir_present"], true);
    assert_eq!(body["metamod"]["gameinfo_wired"], true);
    assert_eq!(body["css"]["installed"], true);

    let plugins = body["css"]["plugins"].as_array().expect("plugins array");
    assert_eq!(plugins.len(), 4);

    let by_name = |name: &str| {
        plugins
            .iter()
            .find(|p| p["name"] == name)
            .unwrap_or_else(|| panic!("{name} present"))
    };
    let matchzy = by_name("MatchZy");
    assert_eq!(matchzy["enabled"], true);
    assert_eq!(matchzy["missing"], false);
    assert_eq!(matchzy["comment"], "tournament");
    assert_eq!(matchzy["group"], "match");
    assert_eq!(matchzy["has_config"], true);
    assert_eq!(
        matchzy["config_path"],
        "game/csgo/addons/counterstrikesharp/configs/plugins/MatchZy/MatchZy.json"
    );

    let rtd = by_name("RollTheDice");
    assert_eq!(rtd["enabled"], false);
    assert_eq!(rtd["missing"], false);

    let gone = by_name("GoneMod");
    assert_eq!(gone["missing"], true);

    let paints = by_name("WeaponPaints");
    assert_eq!(paints["group_title"], Value::Null);
    assert_eq!(paints["group_index"], u32::MAX as u64);
}

#[test]
fn state_flags_broken_plugin_layout() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    // Folder present, dll name does not match the folder.
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/plugins/Broken/Other.dll"),
        b"MZ",
    );

    let (_, body) = body_json(crate::handlers::state::handle(&mut host, &params("3")));
    let plugins = body["css"]["plugins"].as_array().expect("plugins array");
    assert_eq!(plugins[0]["name"], "Broken");
    assert_eq!(plugins[0]["missing"], true);
}

#[test]
fn state_rejects_non_source2_servers() {
    let mut host = MockHost::cs2();
    if let Some(game) = host.games.get_mut("cs2") {
        game.engine = "goldsource".into();
        game.engine_version = "1".into();
    }
    let (status, body) = body_json(crate::handlers::state::handle(&mut host, &params("3")));
    assert_eq!(status, 422);
    assert_eq!(body["code"], "UNSUPPORTED_ENGINE");
}

// ---------------------------------------------------------------- toggle

#[test]
fn toggle_disable_moves_folder_to_disabled() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");

    let (status, body) = body_json(crate::handlers::toggle::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "enabled": false}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], true);
    assert_eq!(host.moves.len(), 1, "one native nodefs move, no shell commands");
    assert!(
        host.file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/disabled/MatchZy/MatchZy.dll"
        ))
        .is_some()
    );
    assert!(
        host.file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll"
        ))
        .is_none()
    );
}

#[test]
fn toggle_enable_moves_folder_back() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_disabled_plugin(&mut host, "RollTheDice");

    let (status, body) = body_json(crate::handlers::toggle::handle(
        &mut host,
        &params("3"),
        br#"{"name": "RollTheDice", "enabled": true}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], true);
    assert!(
        host.file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/RollTheDice/RollTheDice.dll"
        ))
        .is_some()
    );
}

#[test]
fn toggle_is_idempotent() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");

    let (status, body) = body_json(crate::handlers::toggle::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "enabled": true}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], false);
    assert!(host.moves.is_empty());
}

#[test]
fn toggle_unknown_plugin_404s() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    let (status, body) = body_json(crate::handlers::toggle::handle(
        &mut host,
        &params("3"),
        br#"{"name": "Nope", "enabled": false}"#,
    ));
    assert_eq!(status, 404);
    assert_eq!(body["code"], "PLUGIN_NOT_FOUND");
}

#[test]
fn toggle_rejects_path_escapes_and_disabled_dir() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    for name in ["../evil", "a/b", "disabled", ".."] {
        let body = format!(r#"{{"name": "{name}", "enabled": false}}"#);
        let (status, _) = body_json(crate::handlers::toggle::handle(
            &mut host,
            &params("3"),
            body.as_bytes(),
        ));
        assert_eq!(status, 400, "{name} must be rejected");
    }
}

// ---------------------------------------------------------------- attributes

#[test]
fn attributes_write_shared_manifest() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");

    let (status, body) = body_json(crate::handlers::attributes::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "comment": " tournament cfg ", "group": "match"}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], true);
    assert_eq!(body["comment"], "tournament cfg");

    let manifest = host.file(MANIFEST_ABS).expect("manifest written");
    let parsed: Value = serde_json::from_slice(manifest).expect("manifest json");
    assert_eq!(parsed["MatchZy"]["Comment"], "tournament cfg");
    assert_eq!(parsed["MatchZy"]["Group"], "match");
}

#[test]
fn attributes_noop_skips_write() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");
    host.add_file(MANIFEST_ABS, br#"{"MatchZy": {"Comment": "x", "Group": ""}}"#);
    let before = host.file(MANIFEST_ABS).map(<[u8]>::to_vec);

    let (status, body) = body_json(crate::handlers::attributes::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "comment": "x", "group": null}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], false);
    assert_eq!(host.file(MANIFEST_ABS).map(<[u8]>::to_vec), before);
}

#[test]
fn attributes_reject_control_characters() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");
    let (status, _) = body_json(crate::handlers::attributes::handle(
        &mut host,
        &params("3"),
        b"{\"name\": \"MatchZy\", \"comment\": \"evil\\nline\"}",
    ));
    assert_eq!(status, 400);
}

// ---------------------------------------------------------------- add

#[test]
fn add_registers_uploaded_plugin() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "CS2-Tags");

    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "CS2-Tags"}"#,
    ));
    assert_eq!(status, 201);
    assert_eq!(body["replaced"], false);

    let manifest = host.file(MANIFEST_ABS).expect("manifest written");
    let parsed: Value = serde_json::from_slice(manifest).expect("manifest json");
    assert!(parsed["CS2-Tags"].is_object());
}

#[test]
fn add_requires_uploaded_dll_and_force_for_replace() {
    let mut host = MockHost::cs2();
    with_css(&mut host);

    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "Ghost"}"#,
    ));
    assert_eq!(status, 422);
    assert_eq!(body["code"], "FILE_NOT_UPLOADED");

    add_plugin(&mut host, "MatchZy");
    host.add_file(MANIFEST_ABS, br#"{"MatchZy": {"Comment": "", "Group": ""}}"#);
    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy"}"#,
    ));
    assert_eq!(status, 409);
    assert_eq!(body["code"], "ALREADY_REGISTERED");

    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "force": true}"#,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["replaced"], true);
}

// ---------------------------------------------------------------- remove

#[test]
fn remove_deletes_folder_and_manifest_entry() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");
    host.add_file(
        MANIFEST_ABS,
        br#"{"MatchZy": {"Comment": "x", "Group": ""}, "Other": {"Comment": "", "Group": ""}}"#,
    );

    let (status, body) = body_json(crate::handlers::remove::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy"}"#,
        &HashMap::new(),
    ));
    assert_eq!(status, 200);
    assert_eq!(body["folder_deleted"], true);
    assert_eq!(body["entry_removed"], true);
    assert!(
        host.file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll"
        ))
        .is_none()
    );
    let parsed: Value =
        serde_json::from_slice(host.file(MANIFEST_ABS).expect("manifest")).expect("json");
    assert!(parsed.get("MatchZy").is_none());
    assert!(parsed.get("Other").is_some());
}

#[test]
fn remove_accepts_query_param_fallback() {
    use gameap_plugin_sdk::proto::gameap::plugin as pb;

    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_disabled_plugin(&mut host, "RollTheDice");

    let query = HashMap::from([(
        "name".to_string(),
        pb::QueryParamValues {
            values: vec!["RollTheDice".to_string()],
        },
    )]);
    let (status, body) = body_json(crate::handlers::remove::handle(
        &mut host,
        &params("3"),
        b"",
        &query,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["folder_deleted"], true);
}

#[test]
fn remove_unknown_plugin_404s() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    let (status, _) = body_json(crate::handlers::remove::handle(
        &mut host,
        &params("3"),
        br#"{"name": "Nope"}"#,
        &HashMap::new(),
    ));
    assert_eq!(status, 404);
}

// ---------------------------------------------------------------- dispatch

#[test]
fn dispatch_serves_state_route() {
    use gameap_plugin_sdk::proto::gameap::plugin as pb;

    let mut host = MockHost::cs2();
    with_css(&mut host);
    let resp = router::dispatch(
        &mut host,
        &pb::HttpRequest {
            method: "GET".into(),
            path: "/servers/3/state".into(),
            ..Default::default()
        },
    );
    assert_eq!(resp.status_code, 200);

    let resp = router::dispatch(
        &mut host,
        &pb::HttpRequest {
            method: "GET".into(),
            path: "/servers/3/nope".into(),
            ..Default::default()
        },
    );
    assert_eq!(resp.status_code, 404);
}
