//! Minimal gameinfo.gi inspection: is the Metamod search path wired in?
//!
//! Installing Metamod:Source on CS2 requires adding
//! `Game csgo/addons/metamod` to the SearchPaths block of gameinfo.gi.
//! Content is treated as lossy UTF-8, line-based; comments (`//`) are ignored.

/// Inserts the Metamod search path into the SearchPaths block, mimicking the
/// official install instructions: the line goes right before the first `Game`
/// entry, copying that entry's indentation. Returns `None` when the content is
/// already wired or has no SearchPaths block to patch (both cases mean
/// "nothing to write"; the caller distinguishes them via [`is_metamod_wired`]).
pub fn wire_metamod(content: &[u8]) -> Option<Vec<u8>> {
    if is_metamod_wired(content) {
        return None;
    }
    let text = String::from_utf8_lossy(content).into_owned();
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };

    let mut in_search_paths = false;
    let mut entered_block = false;
    let mut depth_into_block = 0i32;
    let mut insert_at: Option<(usize, String)> = None; // (byte offset of line start, indentation)
    let mut offset = 0usize;

    for line in text.split_inclusive('\n') {
        let code = match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        };
        let trimmed = code.trim();
        if !in_search_paths {
            if trimmed.to_ascii_lowercase().starts_with("searchpaths") {
                in_search_paths = true;
                // Tolerate "SearchPaths {" on one line.
                depth_into_block += code.matches('{').count() as i32;
                if depth_into_block > 0 {
                    entered_block = true;
                }
            }
        } else {
            depth_into_block += code.matches('{').count() as i32;
            if depth_into_block > 0 {
                entered_block = true;
                // First Game entry inside the block anchors the insertion.
                let first_token = trimmed.split_whitespace().next().unwrap_or("");
                if first_token.eq_ignore_ascii_case("game") {
                    let indent: String = line
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect();
                    insert_at = Some((offset, indent));
                    break;
                }
            }
            depth_into_block -= code.matches('}').count() as i32;
            if entered_block && depth_into_block <= 0 {
                break; // SearchPaths block closed without a Game line
            }
        }
        offset += line.len();
    }

    let (at, indent) = insert_at?;
    let mut patched = String::with_capacity(text.len() + 64);
    patched.push_str(&text[..at]);
    patched.push_str(&indent);
    patched.push_str("Game\tcsgo/addons/metamod");
    patched.push_str(newline);
    patched.push_str(&text[at..]);
    Some(patched.into_bytes())
}

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

    #[test]
    fn wires_before_first_game_entry() {
        let gi = b"\"GameInfo\"\n{\n\tFileSystem\n\t{\n\t\tSearchPaths\n\t\t{\n\t\t\tGame_LowViolence\tcsgo_lv\n\t\t\tGame\tcsgo\n\t\t\tGame\tcore\n\t\t}\n\t}\n}\n";
        let patched = wire_metamod(gi).expect("patches");
        let text = String::from_utf8(patched).expect("utf8");
        assert!(is_metamod_wired(text.as_bytes()));
        let mm_pos = text.find("csgo/addons/metamod").expect("inserted");
        let game_pos = text.find("Game\tcsgo\n").expect("original kept");
        assert!(mm_pos < game_pos, "metamod line must precede Game csgo");
        // Indentation copied from the anchor line.
        assert!(text.contains("\t\t\tGame\tcsgo/addons/metamod\n"));
        // Game_LowViolence must NOT anchor the insert.
        assert!(text.find("Game_LowViolence").expect("lv kept") < mm_pos);
    }

    #[test]
    fn wire_preserves_crlf() {
        let gi = b"SearchPaths\r\n{\r\n\tGame\tcsgo\r\n}\r\n";
        let patched = wire_metamod(gi).expect("patches");
        let text = String::from_utf8(patched).expect("utf8");
        assert!(text.contains("\tGame\tcsgo/addons/metamod\r\n\tGame\tcsgo\r\n"));
    }

    #[test]
    fn wire_noop_when_wired_or_unpatchable() {
        assert!(wire_metamod(b"SearchPaths\n{\n\tGame\tcsgo/addons/metamod\n}\n").is_none());
        assert!(wire_metamod(b"\"GameInfo\"\n{\n}\n").is_none());
        assert!(wire_metamod(b"SearchPaths\n{\n}\n").is_none());
    }
}
