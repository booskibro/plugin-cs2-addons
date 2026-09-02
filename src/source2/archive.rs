//! In-plugin zip extraction for catalog installs (small release archives that
//! fit through the panel's 10MB plugin-HTTP cap). Big archives — the CSS
//! with-runtime bundle — are downloaded and unpacked on the node instead.

use std::io::Read;

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    /// Forward-slash relative path, sanitized (no leading /, no dot segments).
    pub path: String,
    pub data: Vec<u8>,
    /// Unix permission bits, defaulted per file type when the zip has none.
    pub mode: u32,
}

/// Total uncompressed budget: a "small plugin" archive that inflates past this
/// is refused rather than ballooning the wasm heap.
const MAX_TOTAL_UNCOMPRESSED: u64 = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;

pub fn extract_zip(bytes: &[u8]) -> Result<Vec<ArchiveEntry>, String> {
    let cursor = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(cursor).map_err(|e| format!("not a zip archive: {e}"))?;
    if zip.len() > MAX_ENTRIES {
        return Err(format!("archive has too many entries ({})", zip.len()));
    }

    let mut total: u64 = 0;
    let mut entries = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip
            .by_index(index)
            .map_err(|e| format!("corrupt zip entry: {e}"))?;
        if file.is_dir() {
            continue;
        }
        let Some(path) = sanitize_zip_path(file.name()) else {
            return Err(format!("archive entry escapes its root: {}", file.name()));
        };
        total = total.saturating_add(file.size());
        if total > MAX_TOTAL_UNCOMPRESSED {
            return Err("archive is too large when uncompressed".into());
        }
        let mut data = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut data)
            .map_err(|e| format!("failed to inflate {}: {e}", file.name()))?;
        let mut mode = file
            .unix_mode()
            .filter(|m| *m & 0o777 != 0)
            .map(|m| m & 0o777)
            .unwrap_or(0o644);
        if needs_exec(&path) {
            mode |= 0o111;
        }
        entries.push(ArchiveEntry { path, data, mode });
    }
    Ok(entries)
}

/// Shared objects and the dotnet host must stay runnable even when the zip
/// was built without meaningful unix modes (Windows CI, python zipfile).
fn needs_exec(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".so") || lower.ends_with("/dotnet")
}

fn sanitize_zip_path(raw: &str) -> Option<String> {
    let normalized = raw.replace('\\', "/");
    let trimmed = normalized.trim_start_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    for segment in trimmed.split('/') {
        match segment {
            "" | "." => continue,
            ".." => return None,
            other => {
                if other.contains('\0') || other.contains(':') {
                    return None;
                }
                parts.push(other);
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}

/// Where a plugin release archive's content should land, judged by its layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallRoot {
    /// Entries already rooted at the game dir (addons/…, cfg/…): extract as-is.
    GameDir,
    /// Entries rooted at the CounterStrikeSharp dir (plugins/…, shared/…):
    /// extract into addons/counterstrikesharp. The shape shared-API releases
    /// ship, where a contract assembly under shared/ travels with the plugin —
    /// install the plugin without it and CSS fails to load it.
    ///
    /// Routed per entry, not wholesale: releases often ship the plugin as a
    /// bare `<Name>/` folder *beside* shared/, and those belong in plugins/.
    /// Extracting them relative to the CSS dir puts the dll one level too high,
    /// where the dotnet host cannot resolve it.
    CssDir,
    /// Entries rooted at a single `<Folder>/` that contains `<Folder>.dll`:
    /// extract into plugins/.
    PluginsDir,
    /// Loose files with a `<name>.dll` at the top: wrap into plugins/<name>/.
    WrapIntoFolder(String),
}

/// The game-dir prefixes a full-overlay release uses.
const GAME_DIR_ROOTS: &[&str] = &["addons", "cfg", "maps", "materials", "sound", "models", "cfg"];
/// The prefixes that sit directly inside addons/counterstrikesharp.
const CSS_DIR_ROOTS: &[&str] = &["plugins", "shared", "configs", "gamedata"];

/// Whether an entry belongs to one of the CounterStrikeSharp dir prefixes,
/// rather than being a plugin folder shipped beside them.
pub fn is_css_dir_entry(path: &str) -> bool {
    let top = path.split('/').next().unwrap_or("");
    CSS_DIR_ROOTS.contains(&top.to_ascii_lowercase().as_str())
}

pub fn detect_install_root(entries: &[ArchiveEntry]) -> Result<InstallRoot, String> {
    if entries.is_empty() {
        return Err("archive is empty".into());
    }
    let mut top_levels: Vec<&str> = Vec::new();
    for entry in entries {
        let top = entry.path.split('/').next().unwrap_or("");
        if !top_levels.contains(&top) {
            top_levels.push(top);
        }
    }

    if top_levels
        .iter()
        .any(|top| GAME_DIR_ROOTS.contains(&top.to_ascii_lowercase().as_str()))
    {
        return Ok(InstallRoot::GameDir);
    }

    // plugins/ and shared/ as siblings → rooted at the CounterStrikeSharp dir.
    // Checked before the single-folder rule so a lone plugins/ root is not
    // mistaken for a plugin folder that happens to be called "plugins".
    if top_levels
        .iter()
        .any(|top| CSS_DIR_ROOTS.contains(&top.to_ascii_lowercase().as_str()))
    {
        return Ok(InstallRoot::CssDir);
    }

    // Single folder holding its own <Folder>.dll → a plugins/-shaped release.
    if top_levels.len() == 1 && !top_levels[0].is_empty() {
        let folder = top_levels[0];
        let dll = format!("{folder}/{folder}.dll").to_ascii_lowercase();
        if entries
            .iter()
            .any(|e| e.path.to_ascii_lowercase() == dll)
        {
            return Ok(InstallRoot::PluginsDir);
        }
    }

    // Loose top-level dll → wrap. Pick the dll whose name we can use as folder.
    let top_dll = entries.iter().find_map(|e| {
        (!e.path.contains('/') && e.path.to_ascii_lowercase().ends_with(".dll"))
            .then(|| e.path[..e.path.len() - 4].to_string())
    });
    if let Some(name) = top_dll {
        return Ok(InstallRoot::WrapIntoFolder(name));
    }

    Err(
        "unrecognized archive layout: expected addons/…, plugins/…, <Name>/<Name>.dll or a top-level .dll"
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn build_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            for (name, data) in files {
                writer
                    .start_file(*name, SimpleFileOptions::default())
                    .expect("start file");
                writer.write_all(data).expect("write");
            }
            writer.finish().expect("finish");
        }
        cursor.into_inner()
    }

    #[test]
    fn extracts_and_sanitizes() {
        let bytes = build_zip(&[
            ("MatchZy/MatchZy.dll", b"dll"),
            ("MatchZy/lang/en.json", b"{}"),
        ]);
        let entries = extract_zip(&bytes).expect("extracts");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "MatchZy/MatchZy.dll");
        assert_eq!(entries[0].mode, 0o644);
    }

    #[test]
    fn rejects_traversal() {
        let bytes = build_zip(&[("../evil.dll", b"x")]);
        assert!(extract_zip(&bytes).is_err());
    }

    #[test]
    fn detects_roots() {
        let game = extract_zip(&build_zip(&[(
            "addons/counterstrikesharp/plugins/MatchZy/MatchZy.dll",
            b"x",
        )]))
        .expect("zip");
        assert_eq!(detect_install_root(&game).expect("root"), InstallRoot::GameDir);

        let plugins = extract_zip(&build_zip(&[("Retakes/Retakes.dll", b"x")])).expect("zip");
        assert_eq!(
            detect_install_root(&plugins).expect("root"),
            InstallRoot::PluginsDir
        );

        let loose = extract_zip(&build_zip(&[("K4-System.dll", b"x")])).expect("zip");
        assert_eq!(
            detect_install_root(&loose).expect("root"),
            InstallRoot::WrapIntoFolder("K4-System".into())
        );

        let junk = extract_zip(&build_zip(&[("readme.txt", b"x")])).expect("zip");
        assert!(detect_install_root(&junk).is_err());
    }

    /// The shape a shared-API release ships: the plugin and the contract
    /// assembly it loads types from, as siblings under the CSS dir.
    #[test]
    fn detects_css_dir_root() {
        let shared = extract_zip(&build_zip(&[
            ("plugins/PlayerSettings/PlayerSettings.dll", b"x"),
            ("shared/PlayerSettingsApi/PlayerSettingsApi.dll", b"x"),
        ]))
        .expect("zip");
        assert_eq!(
            detect_install_root(&shared).expect("root"),
            InstallRoot::CssDir
        );

        // A lone plugins/ root counts too - it would otherwise be read as a
        // plugin folder named "plugins" and fail for want of plugins.dll.
        let only_plugins =
            extract_zip(&build_zip(&[("plugins/Retakes/Retakes.dll", b"x")])).expect("zip");
        assert_eq!(
            detect_install_root(&only_plugins).expect("root"),
            InstallRoot::CssDir
        );
    }

    #[test]
    fn so_files_get_exec_bit() {
        let bytes = build_zip(&[("addons/metamod/bin/linuxsteamrt64/metamod.2.cs2.so", b"x")]);
        let entries = extract_zip(&bytes).expect("extracts");
        assert_eq!(entries[0].mode, 0o755);
    }
}
