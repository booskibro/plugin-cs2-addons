//! Minimal gameinfo.gi inspection: is the Metamod search path wired in?
//!
//! Installing Metamod:Source on CS2 requires adding
//! `Game csgo/addons/metamod` to the SearchPaths block of gameinfo.gi.
//! Content is treated as lossy UTF-8, line-based; comments (`//`) are ignored.

/// True when a non-comment line references the addons/metamod search path.
pub fn is_metamod_wired(content: &[u8]) -> bool {
    let text = String::from_utf8_lossy(content);
    for line in text.lines() {
        let line = match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        };
        if line.to_ascii_lowercase().contains("addons/metamod") {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_wired_metamod() {
        let gi = br#"
"GameInfo"
{
    FileSystem
    {
        SearchPaths
        {
            Game_LowViolence	csgo_lv
            Game	csgo/addons/metamod
            Game	csgo
        }
    }
}
"#;
        assert!(is_metamod_wired(gi));
    }

    #[test]
    fn ignores_comments_and_absence() {
        assert!(!is_metamod_wired(b"Game csgo\n"));
        assert!(!is_metamod_wired(b"// Game csgo/addons/metamod\n"));
        assert!(is_metamod_wired(b"Game CSGO/Addons/Metamod // required\n"));
    }
}
