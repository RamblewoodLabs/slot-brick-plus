//! Offline rankings for the shelf filters (SELECT on a shelf): TOP 100, POPULAR, BEST SELLING
//! and FIRST PARTY. The tables are made by brick/make-rankings.py from the LaunchBox Games
//! Database's community ratings, libretro-database's publishers and Wikipedia's best-selling
//! lists, and built into Slot, so nothing is looked up on the network. A system's table is read
//! the first time one of its shelves asks.
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::platform::Platform;

/// What the tables know about one game. A rank of 0 means no data.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Rank {
    /// By weighted user score: 1 is the best rated on the system.
    pub top: u32,
    /// By how many people rated it: 1 is the most rated.
    pub pop: u32,
    /// Place on Wikipedia's best-selling list for the system.
    pub sales: u32,
    /// Published by whoever made the system.
    pub first: bool,
}

fn table(platform: Platform) -> &'static str {
    match platform {
        Platform::Gba => include_str!("../rankings/GBA.tsv"),
        Platform::Gb => include_str!("../rankings/GB.tsv"),
        Platform::Gbc => include_str!("../rankings/GBC.tsv"),
        Platform::Nes => include_str!("../rankings/NES.tsv"),
        Platform::Snes => include_str!("../rankings/SNES.tsv"),
        Platform::Ps1 => include_str!("../rankings/PS1.tsv"),
        Platform::Md => include_str!("../rankings/MD.tsv"),
        Platform::Sms => include_str!("../rankings/SMS.tsv"),
        Platform::Gg => include_str!("../rankings/GG.tsv"),
        Platform::Arcade => include_str!("../rankings/ARCADE.tsv"),
        Platform::Pce => include_str!("../rankings/PCE.tsv"),
        Platform::A2600 => include_str!("../rankings/A2600.tsv"),
        Platform::Intv => include_str!("../rankings/INTV.tsv"),
        Platform::Lynx => include_str!("../rankings/LYNX.tsv"),
        Platform::A7800 => include_str!("../rankings/A7800.tsv"),
        Platform::Gw => include_str!("../rankings/GW.tsv"),
    }
}

/// The words in a file name's tags that mean the platform holder made it, for collections that
/// name the maker there ("Snoopy (Nintendo, Table Top)", "Astrosmash (1981) (Mattel)").
fn holder_words(platform: Platform) -> &'static [&'static str] {
    match platform {
        Platform::Gba | Platform::Gb | Platform::Gbc | Platform::Nes | Platform::Snes | Platform::Gw => &["nintendo"],
        Platform::Md | Platform::Sms | Platform::Gg => &["sega"],
        Platform::Ps1 => &["sony"],
        Platform::Pce => &["nec", "hudson"],
        Platform::A2600 | Platform::A7800 | Platform::Lynx => &["atari"],
        Platform::Intv => &["mattel"],
        Platform::Arcade => &[],
    }
}

pub fn parse(text: &str) -> HashMap<String, Rank> {
    let mut out = HashMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('\t').collect();
        let [key, top, pop, sales, first] = f[..] else {
            continue;
        };
        let n = |s: &str| s.parse::<u32>().unwrap_or(0);
        out.insert(key.to_string(), Rank { top: n(top), pop: n(pop), sales: n(sales), first: first == "1" });
    }
    out
}

fn loaded(platform: Platform) -> &'static HashMap<String, Rank> {
    static TABLES: [OnceLock<HashMap<String, Rank>>; Platform::ALL.len()] = [const { OnceLock::new() }; Platform::ALL.len()];
    TABLES[platform.index()].get_or_init(|| parse(table(platform)))
}

/// What the tables say about the game in `stem` (its file name without the extension).
pub fn rank(platform: Platform, stem: &str) -> Rank {
    let key = match platform {
        Platform::Arcade => stem.to_ascii_lowercase(),
        _ => rank_key(stem),
    };
    let mut rank = loaded(platform).get(&key).copied().unwrap_or_default();
    if !rank.first {
        rank.first = holder_in_tags(platform, stem);
    }
    rank
}

fn holder_in_tags(platform: Platform, stem: &str) -> bool {
    let Some(at) = stem.find(['(', '[']) else {
        return false;
    };
    let tags = stem[at..].to_ascii_lowercase();
    let words: Vec<&str> = tags.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    holder_words(platform).iter().any(|w| words.contains(w))
}

fn unaccent(c: char) -> char {
    match c {
        'à'..='å' => 'a',
        'è'..='ë' => 'e',
        'ì'..='ï' => 'i',
        'ò'..='ö' | 'ø' => 'o',
        'ù'..='ü' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        'ý' | 'ÿ' => 'y',
        c => c,
    }
}

/// The game's name with its tags, case, accents, punctuation and "the" taken off, so "Legend
/// of Zelda, The - A Link to the Past (USA)" and "The Legend of Zelda: A Link to the Past" meet.
/// make-rankings.py's rank_key does exactly the same.
pub fn rank_key(name: &str) -> String {
    let base = match name.find(|c| c == '(' || c == '[') {
        Some(at) => name[..at].trim_end(),
        None => name,
    };
    let base: String = base
        .to_lowercase()
        .chars()
        .map(unaccent)
        .filter(|c| *c != '\'')
        .collect::<String>()
        .replace('&', " and ");
    let mut words: Vec<&str> = base
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|w| !w.is_empty() && *w != "the")
        .collect();
    if words.len() > 1 && words.last() == Some(&"version") {
        words.pop();
    }
    words
        .into_iter()
        .map(|w| match w {
            "ii" => "2",
            "iii" => "3",
            "iv" => "4",
            "vi" => "6",
            "vii" => "7",
            "viii" => "8",
            w => w,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_match_how_the_lists_name_games() {
        assert_eq!(rank_key("Legend of Zelda, The - A Link to the Past (USA)"), "legendofzeldaalinktopast");
        assert_eq!(rank_key("The Legend of Zelda: A Link to the Past"), "legendofzeldaalinktopast");
        assert_eq!(rank_key("Pokemon - Ruby Version (USA, Europe) (Rev 2)"), "pokemonruby");
        assert_eq!(rank_key("Pokémon Ruby"), "pokemonruby");
        assert_eq!(rank_key("Street Fighter II Turbo (USA)"), "streetfighter2turbo");
        assert_eq!(rank_key("Donkey Kong Country 2 - Diddy's Kong Quest (USA) (En,Fr) (Rev 1)"), "donkeykongcountry2diddyskongquest");
        assert_eq!(rank_key("Mario & Luigi - Superstar Saga (USA)"), "marioandluigisuperstarsaga");
        assert_eq!(rank_key("Mega Man X (USA)"), "megamanx");
    }

    #[test]
    fn the_built_in_tables_rank_the_classics() {
        let zelda = rank(Platform::Snes, "Legend of Zelda, The - A Link to the Past (USA)");
        assert!(zelda.top > 0 && zelda.top <= 20, "{zelda:?}");
        assert!(zelda.pop > 0 && zelda.sales > 0 && zelda.first);
        let smb = rank(Platform::Nes, "Super Mario Bros. (World)");
        assert_eq!(smb.sales, 1);
        assert!(rank(Platform::Arcade, "galaga").pop > 0);
        assert!(!rank(Platform::Arcade, "galaga").first);
        assert_eq!(rank(Platform::Gba, "Not A Real Game (USA)"), Rank::default());
    }

    #[test]
    fn makers_named_in_the_tags_are_first_party() {
        assert!(rank(Platform::Gw, "Snoopy (Nintendo, Table Top)").first);
        assert!(!rank(Platform::Gw, "Banana (VTech, Time & Fun)").first);
        assert!(rank(Platform::Intv, "Snafu (1981) (Mattel)").first);
        assert!(!holder_in_tags(Platform::Intv, "Mattel Football"));
    }

    #[test]
    fn rows_parse_and_bad_ones_are_skipped() {
        let t = parse("# header\nabc\t1\t2\t0\t1\nbad\tline\nxyz\t0\t0\t3\t0\n");
        assert_eq!(t["abc"], Rank { top: 1, pop: 2, sales: 0, first: true });
        assert_eq!(t["xyz"].sales, 3);
        assert_eq!(t.len(), 2);
    }
}
