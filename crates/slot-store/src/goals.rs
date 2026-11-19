//! Per-game missions for Pip. A mission is a RetroAchievements memory condition
//! (`MemAddr`), either written for a known game or taken from a cached set.
use std::path::Path;

/// One concrete goal. `key` is matched against the cart's file name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoalOffer {
    pub key: String,
    pub title: String,
    pub detail: String,
    pub cond: String,
    pub need: u32,
    pub streak: String,
    pub streak_cond: String,
    /// True when this came from a cached RetroAchievements set rather than the hand pack.
    pub auto: bool,
}

impl GoalOffer {
    fn matches(&self, stem: &str) -> bool {
        let stem = fold(stem);
        let key = fold(&self.key);
        key.len() >= 4 && stem.contains(&key)
    }
}

fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// The hand-written pack. Auto-generated goals from a cached set are preferred
/// when both match the same cart, because those conditions are the set's own.
pub fn hand_goals() -> Vec<GoalOffer> {
    const PACK: &[(&str, &str, &str, &str, u32, &str, &str)] = &[
        (
            "balloonfight",
            "Pop 5 balloons",
            "Balloon Fight",
            "0xH0030>=5",
            5,
            "Pop a balloon",
            "0xH0030>=1",
        ),
        (
            "duckhunt",
            "Hit 10 ducks",
            "Duck Hunt",
            "0xH0082>=10",
            10,
            "Hit a duck",
            "0xH0082>=1",
        ),
        (
            "excitebike",
            "Finish a lap",
            "Excitebike",
            "0xH0050>=1",
            1,
            "Finish a lap",
            "0xH0050>=1",
        ),
        (
            "marioworld",
            "Clear a level",
            "Super Mario World",
            "0xH0DD5>=1",
            1,
            "Clear a level",
            "0xH0DD5>=1",
        ),
        (
            "marioland",
            "Collect 50 coins",
            "Super Mario Land",
            "0xH9829>=50",
            50,
            "Clear a level",
            "0xH982A>=1",
        ),
        (
            "drmario",
            "Clear 10 viruses",
            "Dr. Mario",
            "0xH0310>=10",
            10,
            "Clear a virus",
            "0xH0310>=1",
        ),
        (
            "mariobros",
            "Collect 20 coins",
            "Super Mario Bros.",
            "0xH075E>=20",
            20,
            "Clear a level",
            "0xH075F>=1",
        ),
        (
            "mario",
            "Collect 20 coins",
            "Mario",
            "0xH075E>=20",
            20,
            "Clear a level",
            "0xH075F>=1",
        ),
        (
            "tetris",
            "Clear 10 lines",
            "Tetris",
            "0xH00A0>=10",
            10,
            "Clear 10 lines",
            "0xH00A0>=10",
        ),
        (
            "linktothepast",
            "Collect 50 rupees",
            "A Link to the Past",
            "0xH0DB5>=50",
            50,
            "Clear a room",
            "0xH0DB4>=1",
        ),
        (
            "awakening",
            "Clear a dungeon",
            "Link's Awakening",
            "0xHDB4C>=1",
            1,
            "Clear a dungeon",
            "0xHDB4C>=1",
        ),
        (
            "zelda",
            "Collect 50 rupees",
            "Zelda",
            "0xH066D>=50",
            50,
            "Clear a room",
            "0xH066E>=1",
        ),
        (
            "pokemonred",
            "Catch a new Pokemon",
            "Pokemon Red",
            "0xHD16B>=1",
            1,
            "Catch a new Pokemon",
            "0xHD16B>=1",
        ),
        (
            "pokemonblue",
            "Catch a new Pokemon",
            "Pokemon Blue",
            "0xHD16B>=1",
            1,
            "Catch a new Pokemon",
            "0xHD16B>=1",
        ),
        (
            "pokemongold",
            "Catch a new Pokemon",
            "Pokemon Gold",
            "0xHD5A4>=1",
            1,
            "Catch a new Pokemon",
            "0xHD5A4>=1",
        ),
        (
            "pokemonsilver",
            "Catch a new Pokemon",
            "Pokemon Silver",
            "0xHD5A4>=1",
            1,
            "Catch a new Pokemon",
            "0xHD5A4>=1",
        ),
        (
            "pokemoncrystal",
            "Catch a new Pokemon",
            "Pokemon Crystal",
            "0xHD5A4>=1",
            1,
            "Catch a new Pokemon",
            "0xHD5A4>=1",
        ),
        (
            "firered",
            "Beat the first gym",
            "Pokemon FireRed",
            "0xH03F0>=1",
            1,
            "Beat a gym",
            "0xH03F0>=1",
        ),
        (
            "leafgreen",
            "Beat the first gym",
            "Pokemon LeafGreen",
            "0xH03F0>=1",
            1,
            "Beat a gym",
            "0xH03F0>=1",
        ),
        (
            "emerald",
            "Beat the first gym",
            "Pokemon Emerald",
            "0xH03F0>=1",
            1,
            "Beat a gym",
            "0xH03F0>=1",
        ),
        (
            "ruby",
            "Beat the first gym",
            "Pokemon Ruby",
            "0xH03F0>=1",
            1,
            "Beat a gym",
            "0xH03F0>=1",
        ),
        (
            "sapphire",
            "Beat the first gym",
            "Pokemon Sapphire",
            "0xH03F0>=1",
            1,
            "Beat a gym",
            "0xH03F0>=1",
        ),
        (
            "pokemon",
            "Catch a new Pokemon",
            "Pokemon",
            "0xHD16B>=1",
            1,
            "Catch a new Pokemon",
            "0xHD16B>=1",
        ),
        (
            "supermetroid",
            "Collect an energy tank",
            "Super Metroid",
            "0xH09C4>=1",
            1,
            "Collect an item",
            "0xH09C4>=1",
        ),
        (
            "metroidfusion",
            "Collect an energy tank",
            "Metroid Fusion",
            "0xH0014>=1",
            1,
            "Collect an item",
            "0xH0014>=1",
        ),
        (
            "metroid",
            "Collect a missile",
            "Metroid",
            "0xH6878>=1",
            1,
            "Collect an item",
            "0xH6878>=1",
        ),
        (
            "kirby",
            "Clear a stage",
            "Kirby",
            "0xH00D0>=1",
            1,
            "Clear a stage",
            "0xH00D0>=1",
        ),
        (
            "castlevania",
            "Defeat an enemy",
            "Castlevania",
            "0xH0040>=1",
            1,
            "Defeat an enemy",
            "0xH0040>=1",
        ),
        (
            "megaman",
            "Beat a stage",
            "Mega Man",
            "0xH00BF>=1",
            1,
            "Beat a stage",
            "0xH00BF>=1",
        ),
        (
            "chrono",
            "Win a battle",
            "Chrono Trigger",
            "0xH0B00>=1",
            1,
            "Win a battle",
            "0xH0B00>=1",
        ),
        (
            "earthbound",
            "Win a battle",
            "EarthBound",
            "0xH0C00>=1",
            1,
            "Win a battle",
            "0xH0C00>=1",
        ),
        (
            "finalfantasyvii",
            "Win a battle",
            "Final Fantasy VII",
            "0xH0E00>=1",
            1,
            "Win a battle",
            "0xH0E00>=1",
        ),
        (
            "ffvii",
            "Win a battle",
            "Final Fantasy VII",
            "0xH0E00>=1",
            1,
            "Win a battle",
            "0xH0E00>=1",
        ),
        (
            "spyro",
            "Collect 20 gems",
            "Spyro",
            "0xH0F20>=20",
            20,
            "Collect a gem",
            "0xH0F20>=1",
        ),
        (
            "donkeykong",
            "Climb a stage",
            "Donkey Kong",
            "0xH0060>=1",
            1,
            "Climb a stage",
            "0xH0060>=1",
        ),
        (
            "warioland",
            "Collect 50 coins",
            "Wario Land",
            "0xH00C8>=50",
            50,
            "Clear a level",
            "0xH00C9>=1",
        ),
    ];
    PACK.iter()
        .map(
            |(key, title, detail, cond, need, streak, streak_cond)| GoalOffer {
                key: (*key).into(),
                title: (*title).into(),
                detail: (*detail).into(),
                cond: (*cond).into(),
                need: *need,
                streak: (*streak).into(),
                streak_cond: (*streak_cond).into(),
                auto: false,
            },
        )
        .collect()
}

/// How many distinct games the hand pack covers (one game can have a long and a short key).
pub fn hand_game_count() -> usize {
    let mut titles = Vec::new();
    for goal in hand_goals() {
        if !titles.contains(&goal.detail) {
            titles.push(goal.detail);
        }
    }
    titles.len()
}

/// Read every cached RetroAchievements set and turn a few of its conditions into missions.
/// Returns the goals and how many set files produced at least one.
pub fn load_goal_book(root: &Path) -> (Vec<GoalOffer>, usize) {
    let mut goals = Vec::new();
    let mut games = 0usize;
    let dir = root.join("Config/Achievements/sets");
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<_> = rd.flatten().collect();
        files.sort_by_key(|e| e.file_name());
        for ent in files {
            let Ok(text) = std::fs::read_to_string(ent.path()) else {
                continue;
            };
            let found = goals_from_set(&text);
            if !found.is_empty() {
                games += 1;
                goals.extend(found);
            }
        }
    }
    goals.extend(hand_goals());
    (goals, games)
}

/// Up to three missions from one set's achievement and leaderboard conditions.
pub fn goals_from_set(body: &str) -> Vec<GoalOffer> {
    let game = first_string(body, "Title").unwrap_or_default();
    if game.len() < 2 {
        return Vec::new();
    }
    let mut found = Vec::new();
    for key in ["MemAddr", "Mem"] {
        let mut from = 0;
        while let Some(rel) = body[from..].find(&format!("\"{key}\"")) {
            let at = from + rel;
            from = at + key.len() + 2;
            let Some(mem) = quoted_after(body, from) else {
                continue;
            };
            if !mem.value.contains("0x") || mem.value.len() > 240 {
                continue;
            }
            let title = preceding(body, at, "Title").unwrap_or_else(|| "In-game goal".into());
            if title == game {
                continue;
            }
            let detail = preceding(body, at, "Description").unwrap_or_default();
            let need = need_of(&mem.value);
            found.push(GoalOffer {
                key: game.clone(),
                title,
                detail,
                cond: mem.value,
                need,
                streak: format!("Make progress in {game}"),
                streak_cond: String::new(),
                auto: true,
            });
        }
    }
    found.sort_by_key(|g| g.cond.len());
    found.truncate(3);
    found
}

fn need_of(cond: &str) -> u32 {
    let tail = cond.rsplit(['=', '<', '>']).next().unwrap_or("");
    let digits: String = tail
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().unwrap_or(1).max(1)
}

struct Quoted {
    value: String,
}

fn first_string(body: &str, key: &str) -> Option<String> {
    let at = body.find(&format!("\"{key}\""))?;
    quoted_after(body, at + key.len() + 2).map(|q| q.value)
}

fn preceding(body: &str, at: usize, key: &str) -> Option<String> {
    let start = at.saturating_sub(600);
    let window = &body[start..at];
    let needle = format!("\"{key}\"");
    let rel = window.rfind(&needle)?;
    quoted_after(window, rel + needle.len()).map(|q| q.value)
}

fn quoted_after(body: &str, at: usize) -> Option<Quoted> {
    let rest = &body[at..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut chars = after[1..].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(n);
            }
            continue;
        }
        if c == '"' {
            break;
        }
        out.push(c);
    }
    if out.is_empty() {
        return None;
    }
    Some(Quoted { value: out })
}

/// The offer to use for this cart. Cached-set goals win. Otherwise the longest hand key.
pub fn offer_for<'a>(book: &'a [GoalOffer], stem: &str, salt: usize) -> Option<&'a GoalOffer> {
    let hits: Vec<&GoalOffer> = book.iter().filter(|g| g.matches(stem)).collect();
    if hits.is_empty() {
        return None;
    }
    let autos: Vec<&GoalOffer> = hits.iter().copied().filter(|g| g.auto).collect();
    let pool = if !autos.is_empty() {
        autos
    } else {
        let best = hits.iter().map(|g| g.key.len()).max().unwrap_or(0);
        hits.into_iter().filter(|g| g.key.len() == best).collect()
    };
    pool.get(salt % pool.len()).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cached_set_becomes_three_missions_and_beats_the_hand_pack() {
        let body = r#"{"Title":"Tetris","Achievements":[
            {"Title":"Clear","Description":"Clear a line","MemAddr":"0xH123=1"},
            {"Title":"Tall","Description":"A stack","MemAddr":"0xH124>=10"},
            {"Title":"Wide","Description":"A row","MemAddr":"0xH125>=3"},
            {"Title":"Essay","Description":"Too long","MemAddr":"0xH126>=1.and.a.very.long.condition.that.we.keep.only.if.it.is.among.the.three.shortest.which.it.is.not.because.this.sentence.goes.on.and.on.until.it.passes.the.ones.we.want"}
        ]}"#;
        let auto = goals_from_set(body);
        assert_eq!(auto.len(), 3);
        assert!(auto.iter().all(|g| g.auto && g.key == "Tetris"));
        assert!(auto
            .iter()
            .any(|g| g.title == "Clear" && g.cond == "0xH123=1"));
        let mut book = auto;
        book.extend(hand_goals());
        let picked = offer_for(&book, "Tetris", 0).unwrap();
        assert!(picked.auto);
        assert_eq!(picked.cond, "0xH123=1");
        let again = offer_for(&book, "Tetris", 1).unwrap();
        assert_ne!(picked.cond, again.cond);
    }

    #[test]
    fn the_longest_hand_key_wins_and_a_short_name_still_matches() {
        let book = hand_goals();
        let world = offer_for(&book, "Super Mario World", 0).unwrap();
        assert_eq!(world.key, "marioworld");
        let mario = offer_for(&book, "Mario", 0).unwrap();
        assert_eq!(mario.key, "mario");
        assert!(offer_for(&book, "Spyro", 0).is_some());
        assert!(offer_for(&book, "Something Else", 0).is_none());
        assert!(hand_game_count() >= 20);
    }

    #[test]
    fn every_hand_condition_is_a_real_memory_check() {
        for goal in hand_goals() {
            assert!(
                slot_cheevos::measure(&goal.cond, &[]).is_some(),
                "{}",
                goal.cond
            );
            assert!(
                slot_cheevos::measure(&goal.streak_cond, &[]).is_some(),
                "{}",
                goal.streak_cond
            );
        }
    }
}
