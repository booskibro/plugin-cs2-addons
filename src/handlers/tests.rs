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
        None,
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
        None,
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
        None,
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
        None,
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
            None,
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
        None,
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
        None,
    ));
    assert_eq!(status, 422);
    assert_eq!(body["code"], "FILE_NOT_UPLOADED");

    add_plugin(&mut host, "MatchZy");
    host.add_file(MANIFEST_ABS, br#"{"MatchZy": {"Comment": "", "Group": ""}}"#);
    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy"}"#,
        None,
    ));
    assert_eq!(status, 409);
    assert_eq!(body["code"], "ALREADY_REGISTERED");

    let (status, body) = body_json(crate::handlers::add::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "force": true}"#,
        None,
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
        None,
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
        None,
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
        None,
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

// ---------------------------------------------------------------- gameinfo repair

const UNWIRED_GI: &[u8] = b"\"GameInfo\"\n{\n\tFileSystem\n\t{\n\t\tSearchPaths\n\t\t{\n\t\t\tGame_LowViolence\tcsgo_lv\n\t\t\tGame\tcsgo\n\t\t}\n\t}\n}\n";

#[test]
fn repair_wires_gameinfo_and_audits() {
    let mut host = MockHost::cs2();
    host.add_file(&format!("{GAME}/gameinfo.gi"), UNWIRED_GI);

    let (status, body) = body_json(crate::handlers::repair::handle(
        &mut host,
        &params("3"),
        Some("john"),
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], true);
    assert!(crate::source2::gameinfo::is_metamod_wired(
        host.file(&format!("{GAME}/gameinfo.gi")).expect("gameinfo")
    ));

    // Audit entry landed in storage.
    let audit = host.storage.get("audit:3").expect("audit stored");
    let entries: Value = serde_json::from_slice(audit).expect("audit json");
    assert_eq!(entries[0]["action"], "gameinfo-repair");
    assert_eq!(entries[0]["user"], "john");

    // Second call is a no-op and adds no audit entry.
    let (_, body) = body_json(crate::handlers::repair::handle(&mut host, &params("3"), None));
    assert_eq!(body["changed"], false);
    let entries: Value =
        serde_json::from_slice(host.storage.get("audit:3").expect("audit")).expect("json");
    assert_eq!(entries.as_array().expect("array").len(), 1);
}

// ---------------------------------------------------------------- metamod vdf

/// The CounterStrikeSharp alias is a platform switch, not a plugin row.
#[test]
fn metamod_toggle_guards_the_platform_vdf() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file(&format!("{GAME}/addons/metamod/counterstrikesharp.vdf"), b"vdf");

    let (status, body) = body_json(crate::handlers::metamod::handle(
        &mut host,
        &params("3"),
        br#"{"name": "counterstrikesharp", "enabled": false}"#,
        Some("john"),
    ));
    assert_eq!(status, 409, "{body}");
    assert_eq!(body["code"], "PLATFORM_VDF");
    assert!(
        host.file(&format!("{GAME}/addons/metamod/counterstrikesharp.vdf")).is_some(),
        "the alias must still be live after a refused toggle"
    );

    // Explicit force still goes through - the guard is a speed bump, not a wall.
    let (status, body) = body_json(crate::handlers::metamod::handle(
        &mut host,
        &params("3"),
        br#"{"name": "counterstrikesharp", "enabled": false, "force": true}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    assert!(host
        .file(&format!("{GAME}/addons/metamod/counterstrikesharp.vdf.disabled"))
        .is_some());

    // Re-enabling never needs force.
    let (status, body) = body_json(crate::handlers::metamod::handle(
        &mut host,
        &params("3"),
        br#"{"name": "counterstrikesharp", "enabled": true}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
}

#[test]
fn metamod_toggle_renames_vdf() {
    let mut host = MockHost::cs2();
    host.add_file(&format!("{GAME}/addons/metamod/cs2fixes.vdf"), b"vdf");

    let (status, body) = body_json(crate::handlers::metamod::handle(
        &mut host,
        &params("3"),
        br#"{"name": "cs2fixes", "enabled": false}"#,
        None,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["changed"], true);
    assert!(host
        .file(&format!("{GAME}/addons/metamod/cs2fixes.vdf.disabled"))
        .is_some());
    assert!(host.file(&format!("{GAME}/addons/metamod/cs2fixes.vdf")).is_none());

    // And back on.
    let (_, body) = body_json(crate::handlers::metamod::handle(
        &mut host,
        &params("3"),
        br#"{"name": "cs2fixes", "enabled": true}"#,
        None,
    ));
    assert_eq!(body["changed"], true);
    assert!(host.file(&format!("{GAME}/addons/metamod/cs2fixes.vdf")).is_some());
}

#[test]
fn state_lists_metamod_vdf_plugins() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file(&format!("{GAME}/addons/metamod/counterstrikesharp.vdf"), b"v");
    host.add_file(&format!("{GAME}/addons/metamod/cs2fixes.vdf.disabled"), b"v");
    host.add_file(&format!("{GAME}/addons/metamod/metaplugins.ini"), b";");

    let (_, body) = body_json(crate::handlers::state::handle(&mut host, &params("3")));
    let mm_plugins = body["metamod"]["plugins"].as_array().expect("vdf list");
    assert_eq!(mm_plugins.len(), 2);
    assert_eq!(mm_plugins[0]["name"], "counterstrikesharp");
    assert_eq!(mm_plugins[0]["enabled"], true);
    assert_eq!(mm_plugins[1]["name"], "cs2fixes");
    assert_eq!(mm_plugins[1]["enabled"], false);
}

// ---------------------------------------------------------------- restart

#[test]
fn restart_calls_servercontrol() {
    let mut host = MockHost::cs2();
    let (status, body) = body_json(crate::handlers::restart::handle(
        &mut host,
        &params("3"),
        Some("john"),
    ));
    assert_eq!(status, 200);
    assert_eq!(body["restarted"], true);
    assert_eq!(host.restarts, vec![3]);
}

// ---------------------------------------------------------------- logs

#[test]
fn logs_tail_newest_file() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/logs/log_20260101.txt"),
        b"old line\n",
    );
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/logs/log_20260817.txt"),
        b"line one\nline two\n",
    );

    let (status, body) = body_json(crate::handlers::logs::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    assert_eq!(
        body["file"],
        "game/csgo/addons/counterstrikesharp/logs/log_20260817.txt"
    );
    let lines = body["lines"].as_array().expect("lines");
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1], "line two");
}

#[test]
fn logs_empty_without_logs_dir() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    let (status, body) = body_json(crate::handlers::logs::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    assert_eq!(body["file"], Value::Null);
}

// ---------------------------------------------------------------- snapshots

#[test]
fn snapshot_create_list_restore_delete() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");

    let (status, body) = body_json(crate::handlers::snapshots::handle_create(
        &mut host,
        &params("3"),
        Some("john"),
    ));
    assert_eq!(status, 200);
    let name = body["snapshot"]["name"].as_str().expect("name").to_string();
    assert!(name.starts_with("snap-"));

    // tar ran on the node, from the css dir, with only present members.
    let (tar_cmd, work_dir) = host.execs.first().expect("tar ran").clone();
    assert!(tar_cmd.starts_with("tar -cf"));
    assert!(tar_cmd.ends_with(" plugins"), "configs/plugins absent: {tar_cmd}");
    assert_eq!(
        work_dir.as_deref(),
        Some(&format!("{GAME}/addons/counterstrikesharp")[..])
    );

    // The tar file must exist for list/restore; MockHost exec does not create
    // it, so fake the node side effect.
    let tar_abs = format!("{GAME}/addons/counterstrikesharp/backups/{name}.tar");
    host.add_file(&tar_abs, b"TAR");

    let (status, body) = body_json(crate::handlers::snapshots::handle_list(
        &mut host,
        &params("3"),
    ));
    assert_eq!(status, 200);
    assert_eq!(body["snapshots"][0]["name"], name.as_str());

    let restore_body = format!(r#"{{"name": "{name}"}}"#);
    let (status, body) = body_json(crate::handlers::snapshots::handle_restore(
        &mut host,
        &params("3"),
        restore_body.as_bytes(),
        None,
    ));
    assert_eq!(status, 200);
    assert_eq!(body["restored"], true);
    // Live plugins dir was wiped before extraction.
    assert!(host
        .file(&format!("{GAME}/addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll"))
        .is_none());
    assert!(host.execs.iter().any(|(cmd, _)| cmd.starts_with("tar -xf")));

    let delete_body = format!(r#"{{"name": "{name}"}}"#);
    let (status, _) = body_json(crate::handlers::snapshots::handle_delete(
        &mut host,
        &params("3"),
        delete_body.as_bytes(),
        None,
    ));
    assert_eq!(status, 200);
    assert!(host.file(&tar_abs).is_none());
}

#[test]
fn snapshot_restore_rejects_bad_names() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    for bad in ["../../etc", "snap-abc", "x", "snap-1; rm -rf /"] {
        let body = format!(r#"{{"name": "{}"}}"#, bad.replace('\\', ""));
        let (status, _) = body_json(crate::handlers::snapshots::handle_restore(
            &mut host,
            &params("3"),
            body.as_bytes(),
            None,
        ));
        assert_eq!(status, 400, "{bad} must be rejected");
    }
}

// ---------------------------------------------------------------- catalog

/// A shared-API release: the plugin plus the contract assembly it needs.
fn shared_api_zip() -> Vec<u8> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        for name in [
            "plugins/PlayerSettings/PlayerSettings.dll",
            "shared/PlayerSettingsApi/PlayerSettingsApi.dll",
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .expect("start");
            writer.write_all(b"MZ").expect("write");
        }
        writer.finish().expect("finish");
    }
    cursor.into_inner()
}

/// The layout that used to be rejected outright, leaving the contract assembly
/// uninstalled and the plugin failing to load.
#[test]
fn archive_install_places_shared_assemblies() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file("/srv/gameap/servers/cs2/upload/PlayerSettings.zip", &shared_api_zip());

    let (status, body) = body_json(crate::handlers::archive_install::handle(
        &mut host,
        &params("3"),
        br#"{"path": "upload/PlayerSettings.zip"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["files_written"], 2);
    assert!(host
        .file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/PlayerSettings/PlayerSettings.dll"
        ))
        .is_some());
    assert!(host
        .file(&format!(
            "{GAME}/addons/counterstrikesharp/shared/PlayerSettingsApi/PlayerSettingsApi.dll"
        ))
        .is_some());
    // Only the plugin is registered - shared/ is not a plugin folder.
    assert_eq!(body["folders"].as_array().expect("folders").len(), 1);
    assert_eq!(body["folders"][0], "PlayerSettings");
}

/// A plugin folder shipped BESIDE shared/, rather than under plugins/ - the
/// shape that used to land the plugin one level too high, directly in
/// addons/counterstrikesharp, where the dotnet host cannot resolve it.
fn bare_folder_with_shared_zip() -> Vec<u8> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        for name in [
            "MenuManagerCore/MenuManagerCore.dll",
            "shared/MenuManagerApi/MenuManagerApi.dll",
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .expect("start");
            writer.write_all(b"MZ").expect("write");
        }
        writer.finish().expect("finish");
    }
    cursor.into_inner()
}

#[test]
fn archive_install_puts_bare_folders_under_plugins() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file(
        "/srv/gameap/servers/cs2/upload/MenuManager.zip",
        &bare_folder_with_shared_zip(),
    );

    let (status, body) = body_json(crate::handlers::archive_install::handle(
        &mut host,
        &params("3"),
        br#"{"path": "upload/MenuManager.zip"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    // The plugin belongs under plugins/, the contract assembly under shared/.
    assert!(host
        .file(&format!(
            "{GAME}/addons/counterstrikesharp/plugins/MenuManagerCore/MenuManagerCore.dll"
        ))
        .is_some());
    assert!(host
        .file(&format!(
            "{GAME}/addons/counterstrikesharp/shared/MenuManagerApi/MenuManagerApi.dll"
        ))
        .is_some());
    // Never directly in the CounterStrikeSharp dir.
    assert!(host
        .file(&format!(
            "{GAME}/addons/counterstrikesharp/MenuManagerCore/MenuManagerCore.dll"
        ))
        .is_none());
    assert_eq!(body["folders"].as_array().expect("folders").len(), 1);
    assert_eq!(body["folders"][0], "MenuManagerCore");
}

fn catalog_zip() -> Vec<u8> {
    use std::io::Write;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        writer
            .start_file("MatchZy/MatchZy.dll", zip::write::SimpleFileOptions::default())
            .expect("start");
        writer.write_all(b"MZ").expect("write");
        writer.finish().expect("finish");
    }
    cursor.into_inner()
}

#[test]
fn catalog_install_downloads_and_registers() {
    let mut host = MockHost::cs2();
    with_css(&mut host);

    let release = serde_json::json!({
        "tag_name": "v0.9.1",
        "html_url": "https://github.com/shobhit-pathak/MatchZy/releases/tag/v0.9.1",
        "assets": [
            {"name": "MatchZy-0.9.1.zip",
             "browser_download_url": "https://example.com/MatchZy-0.9.1.zip"}
        ]
    });
    host.http_responses.insert(
        "https://api.github.com/repos/shobhit-pathak/MatchZy/releases/latest".into(),
        (200, serde_json::to_vec(&release).expect("json")),
    );
    host.http_responses.insert(
        "https://example.com/MatchZy-0.9.1.zip".into(),
        (200, catalog_zip()),
    );

    let (status, body) = body_json(crate::handlers::catalog_routes::handle_install(
        &mut host,
        &params("3"),
        br#"{"key": "matchzy"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["folder"], "MatchZy");
    assert_eq!(body["version"], "0.9.1");
    assert_eq!(body["files_written"], 1);
    assert!(host
        .file(&format!("{GAME}/addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll"))
        .is_some());

    // Registered in the shared manifest.
    let manifest: Value =
        serde_json::from_slice(host.file(MANIFEST_ABS).expect("manifest")).expect("json");
    assert!(manifest["MatchZy"].is_object());
}

#[test]
fn catalog_list_serves_entries() {
    let mut host = MockHost::cs2();
    let (status, body) = body_json(crate::handlers::catalog_routes::handle_list(
        &mut host,
        &params("3"),
    ));
    assert_eq!(status, 200);
    assert!(body["entries"].as_array().expect("entries").len() >= 4);
}

// ---------------------------------------------------------------- updates

#[test]
fn updates_caches_and_serves_versions() {
    let mut host = MockHost::cs2();
    host.http_responses.insert(
        "https://mms.alliedmods.net/mmsdrop/2.0/mmsource-latest-linux".into(),
        (200, b"mmsource-2.0.0-git1359-linux.tar.gz".to_vec()),
    );
    let css_release = serde_json::json!({
        "tag_name": "v1.0.305",
        "html_url": "https://github.com/roflmuffin/CounterStrikeSharp/releases/tag/v1.0.305",
        "assets": [
            {"name": "counterstrikesharp-with-runtime-linux-1.0.305.zip",
             "browser_download_url": "https://example.com/css.zip"}
        ]
    });
    host.http_responses.insert(
        "https://api.github.com/repos/roflmuffin/CounterStrikeSharp/releases/latest".into(),
        (200, serde_json::to_vec(&css_release).expect("json")),
    );
    // Catalog repos are not scripted — their failures must not fail the route.

    let (status, body) = body_json(crate::handlers::updates::handle(
        &mut host,
        &params("3"),
        &HashMap::new(),
    ));
    assert_eq!(status, 200);
    assert_eq!(body["metamod"]["version"], "2.0.0-git1359");
    assert_eq!(
        body["metamod"]["download_url"],
        "https://mms.alliedmods.net/mmsdrop/2.0/mmsource-2.0.0-git1359-linux.tar.gz"
    );
    assert_eq!(body["css"]["version"], "1.0.305");
    assert_eq!(body["stale"], false);

    // Second call hits the cache: no new http traffic.
    let calls_before = host.http_calls.len();
    let (status, _) = body_json(crate::handlers::updates::handle(
        &mut host,
        &params("3"),
        &HashMap::new(),
    ));
    assert_eq!(status, 200);
    assert_eq!(host.http_calls.len(), calls_before);
}

// ---------------------------------------------------------------- platform install

#[test]
fn platform_install_metamod_downloads_on_node() {
    let mut host = MockHost::cs2();
    host.add_file(&format!("{GAME}/gameinfo.gi"), UNWIRED_GI);
    host.http_responses.insert(
        "https://mms.alliedmods.net/mmsdrop/2.0/mmsource-latest-linux".into(),
        (200, b"mmsource-2.0.0-git1359-linux.tar.gz".to_vec()),
    );
    // curl "succeeds" but MockHost exec creates no file — fake it via a
    // scripted side effect: pre-create the archive path the handler checks.
    host.add_file(
        "/srv/gameap/servers/cs2/.cs2addons/mmsource.tar.gz",
        b"TARGZ",
    );

    let (status, body) = body_json(crate::handlers::platform::handle(
        &mut host,
        &params("3"),
        br#"{"kind": "metamod"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["version"], "2.0.0-git1359");
    assert_eq!(body["gameinfo_patched"], true);

    let commands: Vec<&str> = host.execs.iter().map(|(cmd, _)| cmd.as_str()).collect();
    assert!(commands.iter().any(|cmd| cmd.starts_with("curl ")));
    assert!(commands.iter().any(|cmd| cmd.starts_with("tar -xzf")));
    assert!(crate::source2::gameinfo::is_metamod_wired(
        host.file(&format!("{GAME}/gameinfo.gi")).expect("gameinfo")
    ));
}

#[test]
fn platform_install_refuses_paths_that_would_split_wrong() {
    let mut host = MockHost::cs2();
    if let Some(server) = host.servers.get_mut(&3) {
        server.dir = "servers/cs 2".into();
    }
    host.add_file(
        "/srv/gameap/servers/cs 2/game/csgo/gameinfo.gi",
        b"SearchPaths\n{\n\tGame\tcsgo\n}\n",
    );
    host.http_responses.insert(
        "https://mms.alliedmods.net/mmsdrop/2.0/mmsource-latest-linux".into(),
        (200, b"mmsource-2.0.0-git1359-linux.tar.gz".to_vec()),
    );

    let (status, body) = body_json(crate::handlers::platform::handle(
        &mut host,
        &params("3"),
        br#"{"kind": "metamod"}"#,
        None,
    ));
    assert_eq!(status, 422);
    assert_eq!(body["code"], "UNSAFE_PATH");
    assert!(host.execs.is_empty(), "no node command may run on an unsafe path");
}

#[test]
fn platform_install_rejects_unknown_kind() {
    let mut host = MockHost::cs2();
    let (status, _) = body_json(crate::handlers::platform::handle(
        &mut host,
        &params("3"),
        br#"{"kind": "sourcemod"}"#,
        None,
    ));
    assert_eq!(status, 400);
}

// ---------------------------------------------------------------- zip install

#[test]
fn archive_install_extracts_registers_and_cleans_up() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_file("/srv/gameap/servers/cs2/upload/MatchZy.zip", &catalog_zip());

    let (status, body) = body_json(crate::handlers::archive_install::handle(
        &mut host,
        &params("3"),
        br#"{"path": "upload/MatchZy.zip"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["folders"][0], "MatchZy");
    assert_eq!(body["files_written"], 1);
    assert!(host
        .file(&format!("{GAME}/addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll"))
        .is_some());
    // Archive removed, manifest updated.
    assert!(host.file("/srv/gameap/servers/cs2/upload/MatchZy.zip").is_none());
    let manifest: Value =
        serde_json::from_slice(host.file(MANIFEST_ABS).expect("manifest")).expect("json");
    assert!(manifest["MatchZy"].is_object());
}

#[test]
fn archive_install_conflicts_without_force() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");
    host.add_file("/srv/gameap/servers/cs2/up.zip", &catalog_zip());

    let (status, body) = body_json(crate::handlers::archive_install::handle(
        &mut host,
        &params("3"),
        br#"{"path": "up.zip"}"#,
        None,
    ));
    assert_eq!(status, 409);
    assert_eq!(body["code"], "ALREADY_REGISTERED");

    host.add_file("/srv/gameap/servers/cs2/up.zip", &catalog_zip());
    let (status, _) = body_json(crate::handlers::archive_install::handle(
        &mut host,
        &params("3"),
        br#"{"path": "up.zip", "force": true}"#,
        None,
    ));
    assert_eq!(status, 200);
}

#[test]
fn archive_install_rejects_escaping_paths() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    for path in ["../evil.zip", "/abs.zip", "a/../b.zip", "not-a-zip.rar"] {
        let body = format!(r#"{{"path": "{path}"}}"#);
        let (status, _) = body_json(crate::handlers::archive_install::handle(
            &mut host,
            &params("3"),
            body.as_bytes(),
            None,
        ));
        assert_eq!(status, 400, "{path} must be rejected");
    }
}

// ---------------------------------------------------------------- auto snapshot

#[test]
fn catalog_install_snapshots_first() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "Existing");

    let release = serde_json::json!({
        "tag_name": "v1.0.0",
        "html_url": "https://github.com/shobhit-pathak/MatchZy/releases/tag/v1.0.0",
        "assets": [
            {"name": "MatchZy-1.0.0.zip",
             "browser_download_url": "https://example.com/mz.zip"}
        ]
    });
    host.http_responses.insert(
        "https://api.github.com/repos/shobhit-pathak/MatchZy/releases/latest".into(),
        (200, serde_json::to_vec(&release).expect("json")),
    );
    host.http_responses
        .insert("https://example.com/mz.zip".into(), (200, catalog_zip()));

    let (status, _) = body_json(crate::handlers::catalog_routes::handle_install(
        &mut host,
        &params("3"),
        br#"{"key": "matchzy"}"#,
        Some("john"),
    ));
    assert_eq!(status, 200);
    assert!(
        host.execs.iter().any(|(cmd, _)| cmd.starts_with("tar -cf")),
        "an automatic snapshot must run before the install"
    );
    let audit: Value =
        serde_json::from_slice(host.storage.get("audit:3").expect("audit")).expect("json");
    let actions: Vec<&str> = audit
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|entry| entry["action"].as_str())
        .collect();
    assert!(actions.contains(&"snapshot-auto"));
    assert!(actions.contains(&"catalog-install"));
}

// ---------------------------------------------------------------- doctor

/// The two placement faults that make a plugin silently never load: a folder
/// unpacked above plugins/, and a shared folder whose name does not match the
/// assembly inside it.
#[test]
fn doctor_flags_misplaced_folders_and_names_the_shared_dll() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");
    // Unpacked one level too high.
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/MenuManagerCore/MenuManagerCore.dll"),
        b"MZ",
    );
    // Folder name does not match the assembly it holds.
    host.add_file(
        &format!("{GAME}/addons/counterstrikesharp/shared/GoldKingZ/GoldKingZ.Api.dll"),
        b"MZ",
    );

    let (status, body) = body_json(crate::handlers::doctor::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    let checks = body["checks"].as_array().expect("checks");
    let by_id = |id: &str| {
        checks
            .iter()
            .find(|c| c["id"] == id)
            .unwrap_or_else(|| panic!("{id} check present"))
    };

    assert_eq!(by_id("stray")["status"], "fail");
    let stray_detail = by_id("stray")["detail"].as_str().expect("detail");
    assert!(stray_detail.contains("MenuManagerCore"), "{stray_detail}");

    assert_eq!(by_id("shared")["status"], "warn");
    let shared_detail = by_id("shared")["detail"].as_str().expect("detail");
    // The point of the check: say which dll is there and what to rename to.
    assert!(shared_detail.contains("GoldKingZ.Api.dll"), "{shared_detail}");
    assert!(
        shared_detail.contains("rename the folder to GoldKingZ.Api"),
        "{shared_detail}"
    );
}

#[test]
fn doctor_flags_duplicates_and_unwired_gameinfo() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_dir(&format!("{GAME}/addons/metamod"));
    host.add_file(&format!("{GAME}/gameinfo.gi"), UNWIRED_GI);
    add_plugin(&mut host, "MatchZy");
    add_disabled_plugin(&mut host, "MatchZy");

    let (status, body) = body_json(crate::handlers::doctor::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    let checks = body["checks"].as_array().expect("checks");
    let by_id = |id: &str| {
        checks
            .iter()
            .find(|c| c["id"] == id)
            .unwrap_or_else(|| panic!("{id} check present"))
    };
    assert_eq!(by_id("gameinfo")["status"], "fail");
    assert_eq!(by_id("duplicates")["status"], "fail");
    assert_eq!(by_id("metamod")["status"], "ok");
    assert_eq!(by_id("css")["status"], "ok");
    assert_eq!(by_id("layout")["status"], "ok");
}

#[test]
fn doctor_all_green_on_healthy_server() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    host.add_dir(&format!("{GAME}/addons/metamod"));
    host.add_file(
        &format!("{GAME}/gameinfo.gi"),
        b"SearchPaths\n{\n\tGame\tcsgo/addons/metamod\n\tGame\tcsgo\n}\n",
    );
    add_plugin(&mut host, "MatchZy");

    let (_, body) = body_json(crate::handlers::doctor::handle(&mut host, &params("3")));
    let checks = body["checks"].as_array().expect("checks");
    assert!(
        checks.iter().all(|c| c["status"] == "ok"),
        "expected all ok, got {body}"
    );
}

// ---------------------------------------------------------------- audit route

#[test]
fn audit_route_returns_recorded_entries() {
    let mut host = MockHost::cs2();
    with_css(&mut host);
    add_plugin(&mut host, "MatchZy");

    let (_, _) = body_json(crate::handlers::toggle::handle(
        &mut host,
        &params("3"),
        br#"{"name": "MatchZy", "enabled": false}"#,
        Some("john"),
    ));

    let (status, body) = body_json(crate::handlers::audit::handle(&mut host, &params("3")));
    assert_eq!(status, 200);
    assert_eq!(body["entries"][0]["action"], "plugin-disable");
    assert_eq!(body["entries"][0]["subject"], "MatchZy");
    assert_eq!(body["entries"][0]["user"], "john");
}

// ------------------------------------------- archive entry size guard

/// An entry at exactly the panel's inline limit, and one byte over it.
fn sized_entry(path: &str, len: u64) -> crate::source2::archive::ArchiveEntry {
    crate::source2::archive::ArchiveEntry {
        path: path.to_string(),
        data: vec![0u8; len as usize],
        mode: 0o644,
    }
}

#[test]
fn an_entry_over_the_panel_inline_limit_is_refused_before_anything_is_written() {
    // Extraction bounds an archive's TOTAL uncompressed size at twice the
    // panel's per-call upload limit and bounds no single member, so one
    // oversized file is reachable from an archive that is itself acceptable.
    // Uploading entry by entry, the panel would refuse that one partway
    // through and leave a half-installed plugin behind; the guard turns it
    // into a refusal with nothing written.
    let mut host = MockHost::cs2();
    with_css(&mut host);
    let ctx = crate::handlers::ctx::ServerCtx::resolve(&mut host, &params("3")).expect("ctx");
    let before = host.files.len();

    let entries = vec![
        sized_entry("ok.txt", 8),
        sized_entry("huge.bin", crate::handlers::PANEL_MAX_INLINE_BYTES + 1),
    ];

    let err = crate::handlers::write_archive_entries(&mut host, &ctx, &entries, &crate::source2::archive::InstallRoot::GameDir)
        .expect_err("an entry over the inline limit must be refused");

    assert_eq!(err.status, 422);
    // Not even the small entry ahead of it — the check runs before the loop.
    assert_eq!(host.files.len(), before, "nothing may be written");
}

#[test]
fn an_entry_exactly_at_the_panel_inline_limit_is_written() {
    // The panel's own comparison is strict — its tests pin 4096 bytes against
    // a 4096 cap as an accepted upload — so the boundary value has to pass
    // here too, or the two disagree at exactly one size and the guard refuses
    // a file the panel would have taken.
    let mut host = MockHost::cs2();
    with_css(&mut host);
    let ctx = crate::handlers::ctx::ServerCtx::resolve(&mut host, &params("3")).expect("ctx");

    let entries = vec![sized_entry("exact.bin", crate::handlers::PANEL_MAX_INLINE_BYTES)];

    let written = crate::handlers::write_archive_entries(&mut host, &ctx, &entries, &crate::source2::archive::InstallRoot::GameDir)
        .expect("the boundary value is not over the limit");

    assert_eq!(written, 1);
}
