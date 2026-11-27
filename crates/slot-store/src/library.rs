//! What Slot keeps about each game besides its saves: how long it has been played, when it was
//! last played, and whether it is a favorite. Config/library.txt, a line a game:
//! `GBA<tab>Pixel Racer<tab>seconds played<tab>last played (unix)<tab>favorite (0/1)`.
use std::collections::HashMap;
use std::path::Path;

use crate::atomic::atomic_write;
use crate::platform::Platform;
use crate::scan::Cart;

pub const LIBRARY_FILE: &str = "Config/library.txt";

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct GameRecord {
    pub played_secs: u64,
    /// UTC seconds when it was last started; 0 for never.
    pub last_played: i64,
    pub favorite: bool,
    /// UTC seconds when it was starred; 0 if unknown or it is not a favorite.
    pub favorited_at: i64,
}

/// How each shelf is ordered.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum LibrarySort {
    #[default]
    Az,
    /// The game played last first; games never played after, A–Z.
    Recent,
    /// The most played first; games never played after, A–Z.
    MostPlayed,
}

impl LibrarySort {
    pub const ALL: [LibrarySort; 3] = [
        LibrarySort::Az,
        LibrarySort::Recent,
        LibrarySort::MostPlayed,
    ];

    pub fn name(self) -> &'static str {
        match self {
            LibrarySort::Az => "az",
            LibrarySort::Recent => "recent",
            LibrarySort::MostPlayed => "most",
        }
    }

    pub fn parse(value: &str) -> Option<LibrarySort> {
        LibrarySort::ALL.into_iter().find(|s| s.name() == value)
    }
}

/// How the combined Favorites shelf is ordered.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum FavoritesSort {
    /// The game starred last first; untimed stars after, by last played, then A–Z.
    #[default]
    Added,
    /// The game played last first; games never played after, A–Z.
    Recent,
    /// The most played first; games never played after, A–Z.
    MostPlayed,
}

impl FavoritesSort {
    pub const ALL: [FavoritesSort; 3] = [FavoritesSort::Added, FavoritesSort::Recent, FavoritesSort::MostPlayed];

    pub fn name(self) -> &'static str {
        match self {
            FavoritesSort::Added => "added",
            FavoritesSort::Recent => "recent",
            FavoritesSort::MostPlayed => "most",
        }
    }

    /// What the shelf's filter pill and the slot say.
    pub fn label(self) -> &'static str {
        match self {
            FavoritesSort::Added => "ADDED",
            FavoritesSort::Recent => "LAST PLAYED",
            FavoritesSort::MostPlayed => "MOST PLAYED",
        }
    }

    pub fn parse(value: &str) -> Option<FavoritesSort> {
        FavoritesSort::ALL.into_iter().find(|s| s.name() == value)
    }
}

/// What a system's shelf shows, picked with SELECT and kept for each shelf: every game in an
/// order (A–Z, last played, most played), or one of the filters made from the offline rankings.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShelfView {
    Az,
    Recent,
    MostPlayed,
    /// The best rated hundred, best first (community user scores, weighted by how many rated).
    Top100,
    /// The hundred the most people rated, most first.
    Popular,
    /// The games on the system's best-selling list, best seller first.
    BestSelling,
    /// The games the platform holder published, A–Z.
    FirstParty,
}

/// The most games a ranked filter keeps.
pub const RANKED_MAX: usize = 100;

impl ShelfView {
    pub const ALL: [ShelfView; 7] = [
        ShelfView::Az,
        ShelfView::Recent,
        ShelfView::MostPlayed,
        ShelfView::Top100,
        ShelfView::Popular,
        ShelfView::BestSelling,
        ShelfView::FirstParty,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ShelfView::Az => "az",
            ShelfView::Recent => "recent",
            ShelfView::MostPlayed => "most",
            ShelfView::Top100 => "top100",
            ShelfView::Popular => "popular",
            ShelfView::BestSelling => "sales",
            ShelfView::FirstParty => "first",
        }
    }

    pub fn parse(value: &str) -> Option<ShelfView> {
        ShelfView::ALL.into_iter().find(|s| s.name() == value)
    }

    /// What the shelf's filter pill and the slot say.
    pub fn label(self) -> &'static str {
        match self {
            ShelfView::Az => "A–Z",
            ShelfView::Recent => "LAST PLAYED",
            ShelfView::MostPlayed => "MOST PLAYED",
            ShelfView::Top100 => "TOP 100",
            ShelfView::Popular => "POPULAR",
            ShelfView::BestSelling => "BEST SELLING",
            ShelfView::FirstParty => "FIRST PARTY",
        }
    }

    /// HOME → Library → Sort By, for a shelf that has not picked its own.
    pub fn from_sort(sort: LibrarySort) -> ShelfView {
        match sort {
            LibrarySort::Az => ShelfView::Az,
            LibrarySort::Recent => ShelfView::Recent,
            LibrarySort::MostPlayed => ShelfView::MostPlayed,
        }
    }

    /// Only some games, in the rankings' order: the shelf numbers them.
    pub fn ranked(self) -> bool {
        matches!(self, ShelfView::Top100 | ShelfView::Popular | ShelfView::BestSelling)
    }

    /// Played order: the game just played may move.
    pub fn by_play(self) -> bool {
        matches!(self, ShelfView::Recent | ShelfView::MostPlayed)
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Library {
    /// Nested so `get` can look up a stem without allocating a String key.
    games: HashMap<Platform, HashMap<String, GameRecord>>,
}

impl Library {
    pub fn read(root: &Path) -> Library {
        std::fs::read_to_string(root.join(LIBRARY_FILE))
            .map(|text| Library::parse(&text))
            .unwrap_or_default()
    }

    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        atomic_write(&root.join(LIBRARY_FILE), self.text().as_bytes())
    }

    pub fn parse(text: &str) -> Library {
        let mut games: HashMap<Platform, HashMap<String, GameRecord>> = HashMap::new();
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            // FAVADD\tPLATFORM\tSTEM\tunix: when the game was starred. Older builds skip
            // these lines (FAVADD is not a platform), so the five-field rows still load.
            if fields.len() == 4 && fields[0] == "FAVADD" {
                let Some(platform) = Platform::ALL
                    .into_iter()
                    .find(|p| p.dir_name() == fields[1])
                else {
                    continue;
                };
                let Ok(favorited_at) = fields[3].parse() else {
                    continue;
                };
                let record: &mut GameRecord = games
                    .entry(platform)
                    .or_default()
                    .entry(fields[2].to_string())
                    .or_default();
                record.favorite = true;
                record.favorited_at = favorited_at;
                continue;
            }
            let [platform, stem, secs, last, favorite] = fields[..] else {
                continue;
            };
            let Some(platform) = Platform::ALL.into_iter().find(|p| p.dir_name() == platform)
            else {
                continue;
            };
            let (Ok(played_secs), Ok(last_played)) = (secs.parse(), last.parse()) else {
                continue;
            };
            let record: &mut GameRecord = games
                .entry(platform)
                .or_default()
                .entry(stem.to_string())
                .or_default();
            record.played_secs = played_secs;
            record.last_played = last_played;
            record.favorite = favorite == "1";
        }
        Library { games }
    }

    /// One line a game that has anything to keep, in a steady order. When a star has a
    /// time, a FAVADD line follows the five-field rows, so older builds still read them.
    pub fn text(&self) -> String {
        let mut lines: Vec<String> = self
            .games
            .iter()
            .flat_map(|(platform, games)| {
                games
                    .iter()
                    .filter(|(_, r)| **r != GameRecord::default())
                    .map(move |(stem, r)| {
                        format!(
                            "{}\t{stem}\t{}\t{}\t{}",
                            platform.dir_name(),
                            r.played_secs,
                            r.last_played,
                            r.favorite as u8
                        )
                    })
            })
            .collect();
        lines.sort();
        let mut added: Vec<String> = self
            .games
            .iter()
            .flat_map(|(platform, games)| {
                games
                    .iter()
                    .filter(|(_, r)| r.favorite && r.favorited_at != 0)
                    .map(move |(stem, r)| {
                        format!(
                            "FAVADD\t{}\t{stem}\t{}",
                            platform.dir_name(),
                            r.favorited_at
                        )
                    })
            })
            .collect();
        added.sort();
        lines.extend(added);
        lines.into_iter().map(|l| l + "\n").collect()
    }

    pub fn get(&self, platform: Platform, stem: &str) -> GameRecord {
        self.games
            .get(&platform)
            .and_then(|games| games.get(stem))
            .copied()
            .unwrap_or_default()
    }

    pub fn get_mut(&mut self, platform: Platform, stem: &str) -> &mut GameRecord {
        self.games
            .entry(platform)
            .or_default()
            .entry(stem.to_string())
            .or_default()
    }

    pub fn any_favorite(&self, carts: &[Cart]) -> bool {
        carts.iter().any(|c| self.get(c.platform, &c.stem).favorite)
    }

    /// Every game the library remembers.
    pub fn records(&self) -> impl Iterator<Item = (Platform, &str, GameRecord)> + '_ {
        self.games.iter().flat_map(|(platform, games)| {
            games
                .iter()
                .map(move |(stem, record)| (*platform, stem.as_str(), *record))
        })
    }
}

/// `carts`, in A–Z order (as `scan` gives them), put in `sort`'s order, and only the favorites
/// kept when `favorites_only`.
pub fn arrange(carts: &mut Vec<Cart>, sort: LibrarySort, favorites_only: bool, library: &Library) {
    if favorites_only {
        carts.retain(|c| library.get(c.platform, &c.stem).favorite);
    }
    // Stable: games alike in the order keep their A–Z order.
    match sort {
        LibrarySort::Az => {}
        LibrarySort::Recent => {
            carts.sort_by_key(|c| std::cmp::Reverse(library.get(c.platform, &c.stem).last_played))
        }
        LibrarySort::MostPlayed => {
            carts.sort_by_key(|c| std::cmp::Reverse(library.get(c.platform, &c.stem).played_secs))
        }
    }
}

/// `carts`, in A–Z order, as `view` shows them; only the favorites when `favorites_only`.
/// A filter keeps one cart a game (clones and regions share a rank) and at most RANKED_MAX.
pub fn arrange_view(carts: &mut Vec<Cart>, view: ShelfView, favorites_only: bool, library: &Library) {
    let pick: fn(&crate::Rank) -> u32 = match view {
        ShelfView::Az => return arrange(carts, LibrarySort::Az, favorites_only, library),
        ShelfView::Recent => return arrange(carts, LibrarySort::Recent, favorites_only, library),
        ShelfView::MostPlayed => return arrange(carts, LibrarySort::MostPlayed, favorites_only, library),
        ShelfView::FirstParty => {
            arrange(carts, LibrarySort::Az, favorites_only, library);
            carts.retain(|c| crate::rank(c.platform, &c.stem).first);
            return;
        }
        ShelfView::Top100 => |r| r.top,
        ShelfView::Popular => |r| r.pop,
        ShelfView::BestSelling => |r| r.sales,
    };
    arrange(carts, LibrarySort::Az, favorites_only, library);
    let mut ranked: Vec<(u32, Cart)> = carts
        .drain(..)
        .filter_map(|c| {
            let at = pick(&crate::rank(c.platform, &c.stem));
            (at > 0).then_some((at, c))
        })
        .collect();
    ranked.sort_by_key(|(at, _)| *at);
    ranked.dedup_by_key(|(at, _)| *at);
    ranked.truncate(RANKED_MAX);
    carts.extend(ranked.into_iter().map(|(_, c)| c));
}

/// Whether SELECT stops at `view` on a shelf of `carts`: the orders always, a filter only when it
/// keeps some games, and FIRST PARTY only when it leaves some out.
pub fn view_shows(view: ShelfView, carts: &[Cart], favorites_only: bool, library: &Library) -> bool {
    if !matches!(view, ShelfView::Top100 | ShelfView::Popular | ShelfView::BestSelling | ShelfView::FirstParty) {
        return true;
    }
    let mut all = carts.to_vec();
    arrange(&mut all, LibrarySort::Az, favorites_only, library);
    let mut kept = all.clone();
    arrange_view(&mut kept, view, favorites_only, library);
    !kept.is_empty() && (view != ShelfView::FirstParty || kept.len() < all.len())
}

/// Favorites for the combined shelf, in `sort`'s order. Games already only the favorites.
pub fn arrange_favorites(carts: &mut Vec<Cart>, sort: FavoritesSort, library: &Library) {
    carts.retain(|c| library.get(c.platform, &c.stem).favorite);
    match sort {
        FavoritesSort::Added => carts.sort_by(|a, b| {
            added_order(
                library.get(a.platform, &a.stem),
                library.get(b.platform, &b.stem),
            )
        }),
        FavoritesSort::Recent => {
            carts.sort_by_key(|c| std::cmp::Reverse(library.get(c.platform, &c.stem).last_played))
        }
        FavoritesSort::MostPlayed => {
            carts.sort_by_key(|c| std::cmp::Reverse(library.get(c.platform, &c.stem).played_secs))
        }
    }
}

/// Newer stars first; stars with no added time come after, by last played (then A–Z, stable).
fn added_order(a: GameRecord, b: GameRecord) -> std::cmp::Ordering {
    match (a.favorited_at, b.favorited_at) {
        (0, 0) => b.last_played.cmp(&a.last_played),
        (0, _) => std::cmp::Ordering::Greater,
        (_, 0) => std::cmp::Ordering::Less,
        (ta, tb) => tb.cmp(&ta),
    }
}

/// Play time as the shelf prints it: in minutes ("12M", "200M"), or with `hours`, in hours and
/// minutes ("12M", "3H20M", "3H05M"). Nothing under a minute.
pub fn played_text(secs: u64, hours: bool) -> Option<String> {
    let mins = secs / 60;
    match (mins, hours) {
        (0, _) => None,
        (m, false) | (m @ 0..=59, true) => Some(format!("{m}M")),
        (m, true) => Some(format!("{}H{:02}M", m / 60, m % 60)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart(platform: Platform, stem: &str) -> Cart {
        Cart {
            platform,
            stem: stem.into(),
            rom: format!("{stem}.gba").into(),
            label: None,
            famicom_label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
        }
    }

    fn stems(carts: &[Cart]) -> Vec<&str> {
        carts.iter().map(|c| c.stem.as_str()).collect()
    }

    #[test]
    fn records_round_trip_and_bad_lines_are_skipped() {
        let mut lib = Library::default();
        *lib.get_mut(Platform::Gba, "Pixel Racer") = GameRecord {
            played_secs: 3700,
            last_played: 1_800_000_000,
            favorite: true,
            favorited_at: 0,
        };
        lib.get_mut(Platform::Gb, "Untouched");
        let text = lib.text();
        assert_eq!(text, "GBA\tPixel Racer\t3700\t1800000000\t1\n");
        let again = Library::parse(&format!(
            "{text}nonsense\nXYZ\ta\t1\t2\t1\nGB\tx\tlots\t2\t0\n"
        ));
        assert_eq!(
            again.get(Platform::Gba, "Pixel Racer"),
            lib.get(Platform::Gba, "Pixel Racer")
        );
        assert_eq!(again.games.len(), 1);
    }

    #[test]
    fn shelves_sort_by_last_played_or_most_played_with_the_rest_a_to_z() {
        let all = [
            cart(Platform::Gba, "Alpha"),
            cart(Platform::Gba, "Beta"),
            cart(Platform::Gba, "Gamma"),
            cart(Platform::Gba, "Delta"),
        ];
        let mut lib = Library::default();
        lib.get_mut(Platform::Gba, "Gamma").last_played = 20;
        lib.get_mut(Platform::Gba, "Gamma").played_secs = 10;
        lib.get_mut(Platform::Gba, "Beta").last_played = 10;
        lib.get_mut(Platform::Gba, "Beta").played_secs = 500;
        lib.get_mut(Platform::Gba, "Delta").favorite = true;
        let order = |sort, favorites| {
            let mut carts = all.to_vec();
            arrange(&mut carts, sort, favorites, &lib);
            stems(&carts).join(" ")
        };
        assert_eq!(order(LibrarySort::Az, false), "Alpha Beta Gamma Delta");
        assert_eq!(order(LibrarySort::Recent, false), "Gamma Beta Alpha Delta");
        assert_eq!(
            order(LibrarySort::MostPlayed, false),
            "Beta Gamma Alpha Delta"
        );
        assert_eq!(order(LibrarySort::Recent, true), "Delta");
        assert!(lib.any_favorite(&all) && !Library::default().any_favorite(&all));
    }

    #[test]
    fn play_time_reads_in_minutes_or_hours_and_minutes() {
        for hours in [false, true] {
            assert_eq!(played_text(59, hours), None);
            assert_eq!(played_text(12 * 60 + 5, hours).as_deref(), Some("12M"));
        }
        assert_eq!(
            played_text(3 * 3600 + 20 * 60, false).as_deref(),
            Some("200M")
        );
        assert_eq!(
            played_text(3 * 3600 + 20 * 60, true).as_deref(),
            Some("3H20M")
        );
        assert_eq!(
            played_text(3 * 3600 + 5 * 60, true).as_deref(),
            Some("3H05M")
        );
        assert_eq!(played_text(3 * 3600, true).as_deref(), Some("3H00M"));
    }

    #[test]
    fn sort_names_round_trip() {
        for sort in LibrarySort::ALL {
            assert_eq!(LibrarySort::parse(sort.name()), Some(sort));
        }
        for sort in FavoritesSort::ALL {
            assert_eq!(FavoritesSort::parse(sort.name()), Some(sort));
        }
    }

    #[test]
    fn favadd_lines_round_trip_and_old_five_field_rows_still_load() {
        let mut lib = Library::default();
        *lib.get_mut(Platform::Gba, "Pixel Racer") = GameRecord {
            played_secs: 3700,
            last_played: 1_800_000_000,
            favorite: true,
            favorited_at: 1_700_000_000,
        };
        let text = lib.text();
        assert!(text.starts_with("GBA\tPixel Racer\t3700\t1800000000\t1\n"));
        assert!(text.contains("FAVADD\tGBA\tPixel Racer\t1700000000\n"));
        let again = Library::parse(&text);
        assert_eq!(
            again.get(Platform::Gba, "Pixel Racer"),
            lib.get(Platform::Gba, "Pixel Racer")
        );
        // An older file with no FAVADD still loads the star.
        let old = Library::parse("GBA\tPixel Racer\t3700\t1800000000\t1\n");
        let rec = old.get(Platform::Gba, "Pixel Racer");
        assert!(rec.favorite && rec.favorited_at == 0 && rec.played_secs == 3700);
        // A FAVADD-only line still marks the game a favorite (and is how a just-starred,
        // never-played game is kept).
        let starred = Library::parse("FAVADD\tMD\tSonic\t42\n");
        let rec = starred.get(Platform::Md, "Sonic");
        assert!(rec.favorite && rec.favorited_at == 42);
        assert_eq!(
            starred.text(),
            "MD\tSonic\t0\t0\t1\nFAVADD\tMD\tSonic\t42\n"
        );
    }

    #[test]
    fn favorites_shelf_orders_by_when_starred_or_last_played() {
        let all = [
            cart(Platform::Gba, "Alpha"),
            cart(Platform::Md, "Sonic"),
            cart(Platform::Snes, "Zelda"),
            cart(Platform::Intv, "Astrosmash"),
        ];
        let mut lib = Library::default();
        lib.get_mut(Platform::Gba, "Alpha").favorite = true;
        lib.get_mut(Platform::Gba, "Alpha").favorited_at = 10;
        lib.get_mut(Platform::Md, "Sonic").favorite = true;
        lib.get_mut(Platform::Md, "Sonic").favorited_at = 30;
        lib.get_mut(Platform::Snes, "Zelda").favorite = true;
        lib.get_mut(Platform::Snes, "Zelda").favorited_at = 20;
        lib.get_mut(Platform::Snes, "Zelda").last_played = 100;
        lib.get_mut(Platform::Intv, "Astrosmash").favorite = true;
        lib.get_mut(Platform::Intv, "Astrosmash").last_played = 50; // no added time
        lib.get_mut(Platform::Md, "Sonic").played_secs = 90;
        lib.get_mut(Platform::Gba, "Alpha").played_secs = 30;
        let order = |sort| {
            let mut carts = all.to_vec();
            arrange_favorites(&mut carts, sort, &lib);
            stems(&carts).join(" ")
        };
        assert_eq!(order(FavoritesSort::Added), "Sonic Zelda Alpha Astrosmash");
        assert_eq!(order(FavoritesSort::Recent), "Zelda Astrosmash Alpha Sonic");
        assert_eq!(order(FavoritesSort::MostPlayed), "Sonic Alpha Zelda Astrosmash");
    }

    #[test]
    fn ranked_filters_keep_one_cart_a_game_best_first() {
        let snes = |s: &str| cart(Platform::Snes, s);
        let all = vec![
            snes("ActRaiser (USA)"),
            snes("Chrono Trigger (USA)"),
            snes("Legend of Zelda, The - A Link to the Past (Europe)"),
            snes("Legend of Zelda, The - A Link to the Past (USA)"),
            snes("Not A Real Game (USA)"),
            snes("Super Mario World (USA)"),
        ];
        let lib = Library::default();
        let order = |view| {
            let mut carts = all.clone();
            arrange_view(&mut carts, view, false, &lib);
            stems(&carts).iter().map(|s| s.split(" (").next().unwrap().to_string()).collect::<Vec<_>>()
        };
        let top = order(ShelfView::Top100);
        assert_eq!(top.len(), 4, "{top:?}");
        assert!(!top.contains(&"Not A Real Game".to_string()));
        assert!(top.iter().position(|s| s == "ActRaiser") > top.iter().position(|s| s == "Chrono Trigger"));
        assert_eq!(order(ShelfView::BestSelling)[0], "Super Mario World");
        assert_eq!(order(ShelfView::Az).len(), 6);
        let first = order(ShelfView::FirstParty);
        assert!(first.contains(&"Super Mario World".to_string()) && !first.contains(&"Chrono Trigger".to_string()));
        assert!(view_shows(ShelfView::Top100, &all, false, &lib));
        assert!(view_shows(ShelfView::FirstParty, &all, false, &lib));
        let unknown = vec![snes("Not A Real Game (USA)")];
        assert!(!view_shows(ShelfView::Top100, &unknown, false, &lib));
        assert!(!view_shows(ShelfView::FirstParty, &unknown, false, &lib));
        assert!(view_shows(ShelfView::MostPlayed, &unknown, false, &lib));
        let nintendo = vec![snes("Super Mario World (USA)")];
        assert!(!view_shows(ShelfView::FirstParty, &nintendo, false, &lib), "FIRST PARTY would be every game");
    }

    #[test]
    fn views_round_trip_by_name() {
        for view in ShelfView::ALL {
            assert_eq!(ShelfView::parse(view.name()), Some(view));
        }
        assert_eq!(ShelfView::parse("nope"), None);
    }
}
