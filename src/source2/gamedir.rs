//! Locating the Source 2 game directory (the one holding gameinfo.gi)
//! inside a server directory — the analogue of the GoldSource mod dir.

/// Known game dir for a GameAP game code, tried before scanning.
pub fn known_game_dir(game_code: &str) -> Option<&'static str> {
    match game_code {
        "cs2" => Some("game/csgo"),
        _ => None,
    }
}

/// Parent directory scanned for `<dir>/gameinfo.gi` when the hint misses.
pub const GAME_PARENT: &str = "game";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_dirs() {
        assert_eq!(known_game_dir("cs2"), Some("game/csgo"));
        assert_eq!(known_game_dir("valve"), None);
    }
}
