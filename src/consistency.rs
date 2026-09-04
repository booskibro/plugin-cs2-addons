//! Tests that the facts duplicated between the Rust and TypeScript halves
//! still agree.
//!
//! Four things are necessarily written twice — the version, the plugin id, the
//! ability name and the game code — and none of them fails loudly when only
//! one copy is updated: a stale frontend version misreports itself in the
//! panel, a mismatched id breaks every route and the tab's permission gate,
//! and a game code the frontend gates on but the backend does not look for
//! either shows a tab the sweep ignores or sweeps a game with no tab. So they
//! are checked here rather than trusted.
//!
//! Ported from the plugin-source-addons sibling, which has had this guard
//! since its first release. This repo is the reason it is worth having: the
//! frontend lockfile silently sat at 0.1.0 until 0.3.4, because `npm ci` does
//! not check the root version field.

#![cfg(test)]

use std::fs;
use std::path::PathBuf;

fn repo_file(relative: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), relative].iter().collect();
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

/// Values of every `"<key>": "<value>"` / `<key>: '<value>'` occurrence.
fn json_string_values(text: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    for quote in ['"', '\''] {
        let needle = format!("\"{key}\":");
        let alt = format!("{key}:");
        for pattern in [needle.as_str(), alt.as_str()] {
            let mut rest = text;
            while let Some(idx) = rest.find(pattern) {
                rest = &rest[idx + pattern.len()..];
                let Some(start) = rest.find(quote) else {
                    continue;
                };
                // Only a value on the same line; a later line's quote is not ours.
                if rest[..start].contains('\n') {
                    continue;
                }
                let Some(end) = rest[start + 1..].find(quote) else {
                    continue;
                };
                out.push(rest[start + 1..start + 1 + end].to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn version_is_the_same_everywhere() {
    let version = env!("CARGO_PKG_VERSION");

    let package_json = repo_file("frontend/package.json");
    assert!(
        json_string_values(&package_json, "version").contains(&version.to_string()),
        "frontend/package.json version does not match Cargo.toml ({version})"
    );

    // npm does not check the root version field on `npm ci`, so the lockfile
    // is the copy most likely to be left behind. It carries two.
    let lock = repo_file("frontend/package-lock.json");
    let lock_versions: Vec<String> = json_string_values(&lock, "version")
        .into_iter()
        .filter(|value| value.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .collect();
    let root_count = lock
        .match_indices(&format!("\"version\": \"{version}\""))
        .count();
    assert!(
        root_count >= 2,
        "frontend/package-lock.json should carry {version} twice (root package and \"\" entry), found {root_count} - {} distinct versions present",
        lock_versions.len()
    );

    let index_ts = repo_file("frontend/src/index.ts");
    assert!(
        json_string_values(&index_ts, "version").contains(&version.to_string()),
        "frontend/src/index.ts version does not match Cargo.toml ({version})"
    );
}

#[test]
fn plugin_id_is_the_same_everywhere() {
    let index_ts = repo_file("frontend/src/index.ts");
    assert!(
        json_string_values(&index_ts, "id").contains(&crate::PLUGIN_ID.to_string()),
        "frontend/src/index.ts id does not match PLUGIN_ID ({})",
        crate::PLUGIN_ID
    );
    // The tab's permission gate embeds the id in a literal string.
    assert!(
        index_ts.contains(&format!("plugin:{}:manage", crate::PLUGIN_ID)),
        "the checkPermission ability does not embed PLUGIN_ID"
    );
}

/// The panel's CompactPluginID rewrites an id that is not round-trip-stable
/// base32 into an FNV hash, which silently breaks the routes and the ability.
#[test]
fn plugin_id_survives_panel_normalization() {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let id = crate::PLUGIN_ID;
    assert!(
        id.bytes().all(|b| ALPHABET.contains(&b)),
        "PLUGIN_ID must be lowercase base32"
    );
    assert!(
        matches!(id.len(), 2 | 4 | 5 | 7 | 8 | 10 | 12 | 13),
        "PLUGIN_ID length {} is not a valid unpadded base32 length",
        id.len()
    );

    // Decode to bits, then re-encode, and require the same string back.
    let mut bits = String::new();
    for byte in id.bytes() {
        let index = ALPHABET
            .iter()
            .position(|candidate| *candidate == byte)
            .expect("checked above");
        bits.push_str(&format!("{index:05b}"));
    }
    let whole_bytes = bits.len() / 8;
    assert!(
        bits[whole_bytes * 8..].bytes().all(|bit| bit == b'0'),
        "PLUGIN_ID has non-zero padding bits and will not re-encode to itself"
    );
    let mut reencoded = String::new();
    let payload = &bits[..whole_bytes * 8];
    let mut index = 0;
    while index < payload.len() {
        let end = (index + 5).min(payload.len());
        let mut chunk = payload[index..end].to_string();
        while chunk.len() < 5 {
            chunk.push('0');
        }
        let value = usize::from_str_radix(&chunk, 2).expect("binary digits");
        reencoded.push(ALPHABET[value] as char);
        index += 5;
    }
    assert_eq!(reencoded, id, "PLUGIN_ID does not round-trip through base32");
}

/// The frontend gates the tab on `checkGame.codes`; the backend looks servers
/// up by `GAME_CODE` and advertises the RCON protocol for it. The sibling
/// plugin has to compare two lists here — this one only has to check that the
/// frontend's single code is still the one the backend uses.
#[test]
fn game_code_matches_the_frontend_tab_gate() {
    let index_ts = repo_file("frontend/src/index.ts");
    let gate = index_ts
        .find("checkGame")
        .expect("frontend declares a checkGame block");
    let key = index_ts[gate..]
        .find("codes:")
        .expect("checkGame gates on codes")
        + gate;
    let block_start = index_ts[key..].find('[').expect("array literal") + key;
    let block_end = index_ts[block_start..].find(']').expect("array end") + block_start;
    let frontend: Vec<String> = index_ts[block_start + 1..block_end]
        .split(',')
        .filter_map(|token| {
            let trimmed = token.trim().trim_matches(['\'', '"', '\n']);
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .collect();

    assert_eq!(
        frontend,
        vec![crate::GAME_CODE.to_string()],
        "frontend/src/index.ts checkGame.codes and GAME_CODE have drifted"
    );
}
