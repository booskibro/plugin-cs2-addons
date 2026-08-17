//! Curated catalog of well-known CounterStrikeSharp plugins, installable
//! straight from their GitHub releases. Kept deliberately small and static:
//! every entry here is a promise that the install pipeline can handle its
//! release layout.

#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    /// Stable key used by the install route and the updates cache.
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// GitHub "owner/repo".
    pub repo: &'static str,
    /// CSS plugin folder the release creates (also matches update badges to
    /// installed rows).
    pub folder: &'static str,
    /// Substrings the release asset name must contain, lowercase. Every
    /// pattern must match; empty means "first .zip asset".
    pub asset_contains: &'static [&'static str],
}

pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        key: "matchzy",
        name: "MatchZy",
        description: "Match system for competitive CS2: practice mode, knife round, veto, demo recording and Get5 integration.",
        repo: "shobhit-pathak/MatchZy",
        folder: "MatchZy",
        asset_contains: &[".zip"],
    },
    CatalogEntry {
        key: "cs2-simpleadmin",
        name: "CS2-SimpleAdmin",
        description: "Admin essentials: bans, mutes, kicks, admin chat and a menu, backed by a database.",
        repo: "daffyyyy/CS2-SimpleAdmin",
        folder: "CS2-SimpleAdmin",
        asset_contains: &[".zip"],
    },
    CatalogEntry {
        key: "retakes",
        name: "Retakes",
        description: "Retakes game mode: spawn T's on a planted bomb site, CT's retake it.",
        repo: "B3none/cs2-retakes",
        folder: "RetakesPlugin",
        asset_contains: &[".zip"],
    },
    CatalogEntry {
        key: "weaponpaints",
        name: "WeaponPaints",
        description: "Skins, knives, gloves and agents for players, configured per SteamID.",
        repo: "Nereziel/cs2-WeaponPaints",
        folder: "WeaponPaints",
        asset_contains: &[".zip"],
    },
    CatalogEntry {
        key: "k4-system",
        name: "K4-System",
        description: "Ranks, stats and playtime tracking with in-chat placements.",
        repo: "KitsuneLab-Development/K4-System",
        folder: "K4-System",
        asset_contains: &[".zip"],
    },
];

pub fn find(key: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|entry| entry.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_unique_and_findable() {
        for entry in CATALOG {
            assert_eq!(find(entry.key).map(|e| e.repo), Some(entry.repo));
        }
        let mut keys: Vec<&str> = CATALOG.iter().map(|e| e.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), CATALOG.len());
    }
}
