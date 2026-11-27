//! Arcade games' titles. An arcade zip has to keep its MAME/FBNeo short name ("mslug.zip"): the
//! core finds the game, and a clone its parent, by that name, so renaming the file breaks it.
//! The shelf shows the title instead, from a table made from the FBNeo, MAME 2003-Plus and MAME
//! DATs (`data/arcade-names.tsv`), so any set dropped in `Games/ARCADE` is named with no work.

use std::collections::HashMap;
use std::sync::OnceLock;

const TABLE: &str = include_str!("../data/arcade-names.tsv");

fn table() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| {
        TABLE
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once('\t'))
            .collect()
    })
}

/// The title of the arcade set `stem` ("mslug" gives "Metal Slug - Super Vehicle-001"), or
/// None for a name no DAT has, such as a zip already renamed by hand.
pub fn arcade_title(stem: &str) -> Option<&'static str> {
    let map = table();
    map.get(stem).or_else(|| map.get(stem.to_ascii_lowercase().as_str())).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_give_titles() {
        assert_eq!(arcade_title("mslug"), Some("Metal Slug - Super Vehicle-001"));
        assert_eq!(arcade_title("sf2"), Some("Street Fighter II - The World Warrior"));
        assert_eq!(arcade_title("3wonders"), Some("Three Wonders"));
        assert_eq!(arcade_title("spacefb"), Some("Space Firebird"));
        assert_eq!(arcade_title("MSLUG"), Some("Metal Slug - Super Vehicle-001"));
    }

    #[test]
    fn old_set_names_are_hand_fixed() {
        assert_eq!(arcade_title("shangonb"), Some("Super Hang-On"));
        assert_eq!(arcade_title("chplftb"), Some("Choplifter"));
    }

    #[test]
    fn unknown_or_renamed_files_have_none() {
        assert_eq!(arcade_title("Metal Slug"), None);
        assert_eq!(arcade_title(""), None);
    }

    #[test]
    fn every_row_is_a_name_and_a_title() {
        for line in TABLE.lines().filter(|l| !l.starts_with('#')) {
            let (name, title) = line.split_once('\t').expect(line);
            assert!(!name.is_empty() && !title.trim().is_empty(), "{line}");
            assert_eq!(name, name.to_ascii_lowercase(), "{line}");
        }
    }
}
