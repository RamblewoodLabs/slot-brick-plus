mod common;

use common::{app_booting_at, tmp_root_with_carts, write_gb_cart};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{read_slot_state, write_slot_state, Library, LibrarySort, Platform, ShelfKind, SlotState, LIBRARY_FILE};
use slot_ui::QuickRow;

fn order(a: &App) -> String {
    a.system_carts().map(|c| c.stem.as_str()).collect::<Vec<_>>().join(" ")
}

fn fav_order(a: &App) -> String {
    a.favorite_carts()
        .map(|c| format!("{}:{}", c.platform.dir_name(), c.stem))
        .collect::<Vec<_>>()
        .join(" ")
}

fn write_stub(d: &tempfile::TempDir, platform: Platform, stem: &str) {
    let dir = d.path().join("Games").join(platform.dir_name());
    std::fs::create_dir_all(&dir).unwrap();
    let ext = platform.extensions()[0];
    std::fs::write(dir.join(format!("{stem}.{ext}")), b"stand-in").unwrap();
}

fn library_app(library: &str, state: SlotState) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Alpha", "Beta", "Gamma"]);
    write_slot_state(d.path(), &SlotState { clock_set: true, ..state }).unwrap();
    std::fs::write(d.path().join(LIBRARY_FILE), library).unwrap();
    let (a, _) = app_booting_at(d.path(), 1_800_000_000);
    assert!(matches!(a.phase(), Phase::Shelf), "{:?}", a.phase());
    (d, a)
}

const PLAYED: &str = "GBA\tGamma\t4000\t20\t1\nGBA\tBeta\t9000\t10\t0\n";

#[test]
fn select_steps_the_shelf_through_its_orders_and_names_each() {
    let (d, mut a) = library_app(PLAYED, SlotState::default());
    assert_eq!(order(&a), "Alpha Beta Gamma");
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(order(&a), "Gamma Beta Alpha");
    assert_eq!(a.slot_text().as_deref(), Some("LAST PLAYED"));
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(order(&a), "Beta Gamma Alpha");
    assert_eq!(a.slot_text().as_deref(), Some("MOST PLAYED"));
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(order(&a), "Alpha Beta Gamma");
    a.settle_saves();
    assert_eq!(slot_store::read_slot_state(d.path()).library_sort, LibrarySort::Az);
}

#[test]
fn the_order_chosen_is_kept_and_the_selected_game_stays_selected() {
    let (_d, mut a) = library_app(PLAYED, SlotState { library_sort: LibrarySort::MostPlayed, ..SlotState::default() });
    assert_eq!(order(&a), "Beta Gamma Alpha");
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    let picked = a.selected_stem().map(str::to_string);
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.selected_stem().map(str::to_string), picked);
}

#[test]
fn y_stars_a_favorite_and_favorites_only_shows_just_those() {
    let (d, mut a) = library_app("", SlotState::default());
    assert_eq!(a.selected_stem(), Some("Alpha"));
    a.apply(Action::GbaDown(Btn::Y));
    a.settle_saves();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(Library::read(d.path()).get(slot_store::Platform::Gba, "Alpha").favorite);
    let (_d2, b) = library_app("GBA\tGamma\t0\t0\t1\n", SlotState { favorites_only: true, ..SlotState::default() });
    assert_eq!(order(&b), "Gamma");
    // With no favorites at all, Favorites still shows every game rather than none.
    let (_d3, c) = library_app("", SlotState { favorites_only: true, ..SlotState::default() });
    assert_eq!(order(&c), "Alpha Beta Gamma");
}

#[test]
fn the_shelf_prints_how_long_the_selected_game_was_played() {
    let (_d, mut a) = library_app(PLAYED, SlotState { library_sort: LibrarySort::MostPlayed, ..SlotState::default() });
    assert_eq!(a.played_text().as_deref(), Some("150M"));
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    assert_eq!(a.selected_stem(), Some("Gamma"));
    assert_eq!(a.played_text().as_deref(), Some("66M"));
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    assert_eq!(a.played_text(), None, "Alpha was never played");
    let (_d2, b) = library_app(PLAYED, SlotState { library_sort: LibrarySort::MostPlayed, play_time_hours: true, ..SlotState::default() });
    assert_eq!(b.played_text().as_deref(), Some("2H30M"));
}

#[test]
fn the_title_card_names_the_selected_game_unless_turned_off() {
    let (_d, mut a) = library_app("", SlotState::default());
    assert_eq!(a.title_card_text().as_deref(), Some("Alpha"));
    a.apply(Action::GbaDown(Btn::Right));
    a.apply(Action::GbaUp(Btn::Right));
    assert_eq!(a.title_card_text().as_deref(), Some("Beta"));
    let (_d2, b) = library_app("", SlotState { title_card: false, ..SlotState::default() });
    assert_eq!(b.title_card_text(), None);
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
    a.apply(Action::GbaUp(btn));
}

fn go_favorites(a: &mut App) {
    if a.current_shelf_kind() != ShelfKind::Favorites {
        press(a, Btn::L1);
    }
    assert_eq!(a.current_shelf_kind(), ShelfKind::Favorites);
}

/// HOME → Library → Systems, with the bar on `row`.
fn open_systems_at(a: &mut App, row: QuickRow) {
    while a.quick_menu().is_some() {
        press(a, Btn::B);
    }
    a.apply(Action::QuickMenu);
    for _ in 0..QuickRow::Library.position() {
        press(a, Btn::Down);
    }
    press(a, Btn::A);
    for _ in 0..QuickRow::LibrarySystems.position() {
        press(a, Btn::Down);
    }
    press(a, Btn::A);
    for _ in 0..row.position() {
        press(a, Btn::Down);
    }
    assert_eq!(a.quick_menu(), Some(row));
}

fn two_systems(state: SlotState) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Alpha", "Beta"]);
    write_gb_cart(&d, "Pocket", "POCKET");
    write_slot_state(d.path(), &SlotState { clock_set: true, ..state }).unwrap();
    let (a, _) = app_booting_at(d.path(), 1_800_000_000);
    (d, a)
}

#[test]
fn a_hidden_system_has_no_shelf_and_stays_hidden() {
    let (d, mut a) = two_systems(SlotState::default());
    assert_eq!(order(&a), "Alpha Beta Pocket");
    open_systems_at(&mut a, QuickRow::ShowGba);
    assert_eq!(a.quick_value(QuickRow::ShowGba), Some(slot_ui::QuickValue::On));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::ShowGba), Some(slot_ui::QuickValue::Off));
    assert_eq!(order(&a), "Pocket");
    assert!(!read_slot_state(d.path()).shows(Platform::Gba));
    // Back on the shelf, the Game Boy's is the one there is.
    for _ in 0..3 {
        press(&mut a, Btn::B);
    }
    assert!(matches!(a.phase(), Phase::Shelf));
    assert_eq!(a.selected_stem(), Some("Pocket"));
    let (_d2, b) = two_systems(read_slot_state(d.path()));
    assert_eq!(order(&b), "Pocket");
}

#[test]
fn the_last_system_with_games_cannot_be_hidden() {
    let mut state = SlotState::default();
    state.hidden_systems[Platform::Gba.index()] = true;
    let (d, mut a) = two_systems(state);
    open_systems_at(&mut a, QuickRow::ShowGb);
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::ShowGb), Some(slot_ui::QuickValue::On));
    assert_eq!(order(&a), "Pocket");
    assert!(read_slot_state(d.path()).shows(Platform::Gb));
    // A system with no games can be hidden all the same.
    open_systems_at(&mut a, QuickRow::ShowNes);
    press(&mut a, Btn::Right);
    assert!(!read_slot_state(d.path()).shows(Platform::Nes));
}

#[test]
fn with_every_system_that_has_games_hidden_all_are_shown() {
    let mut state = SlotState::default();
    state.hidden_systems[Platform::Gba.index()] = true;
    state.hidden_systems[Platform::Gb.index()] = true;
    let (d, mut a) = two_systems(state);
    assert_eq!(order(&a), "Alpha Beta Pocket");
    // Showing one again is never refused.
    open_systems_at(&mut a, QuickRow::ShowGb);
    press(&mut a, Btn::Right);
    assert!(read_slot_state(d.path()).shows(Platform::Gb));
    assert_eq!(order(&a), "Pocket");
}

#[test]
fn favorites_only_counts_the_favorites_on_shown_systems() {
    let mut state = SlotState { favorites_only: true, ..SlotState::default() };
    state.hidden_systems[Platform::Gb.index()] = true;
    let d = tmp_root_with_carts(&["Alpha", "Beta"]);
    write_gb_cart(&d, "Pocket", "POCKET");
    write_slot_state(d.path(), &SlotState { clock_set: true, ..state }).unwrap();
    std::fs::write(d.path().join(LIBRARY_FILE), "GB\tPocket\t0\t0\t1\n").unwrap();
    let (a, _) = app_booting_at(d.path(), 1_800_000_000);
    assert_eq!(order(&a), "Alpha Beta", "the only favorite is on a hidden shelf");
    // The combined Favorites shelf still holds it, even though its system is hidden.
    assert_eq!(fav_order(&a), "GB:Pocket");
}

fn mixed_favorites_app(library: &str, state: SlotState) -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Alpha"]);
    write_gb_cart(&d, "Pocket", "POCKET");
    write_stub(&d, Platform::Md, "Sonic");
    write_stub(&d, Platform::Snes, "Zelda");
    write_stub(&d, Platform::Intv, "Astrosmash");
    write_stub(&d, Platform::Arcade, "mslug");
    write_stub(&d, Platform::Gw, "donkykong");
    write_slot_state(d.path(), &SlotState { clock_set: true, ..state }).unwrap();
    std::fs::write(d.path().join(LIBRARY_FILE), library).unwrap();
    let (a, _) = app_booting_at(d.path(), 1_800_000_000);
    (d, a)
}

#[test]
fn a_favorites_shelf_holds_starred_games_of_every_system() {
    let library = concat!(
        "GBA\tAlpha\t0\t0\t1\n",
        "MD\tSonic\t0\t50\t1\n",
        "SNES\tZelda\t0\t80\t1\n",
        "INTV\tAstrosmash\t0\t0\t1\n",
        "ARCADE\tmslug\t0\t10\t1\n",
        "GW\tdonkykong\t0\t0\t1\n",
        "FAVADD\tMD\tSonic\t30\n",
        "FAVADD\tSNES\tZelda\t20\n",
        "FAVADD\tGBA\tAlpha\t10\n",
        "FAVADD\tINTV\tAstrosmash\t40\n",
        "FAVADD\tARCADE\tmslug\t15\n",
        "FAVADD\tGW\tdonkykong\t5\n",
    );
    let (_d, mut a) = mixed_favorites_app(library, SlotState::default());
    go_favorites(&mut a);
    assert_eq!(fav_order(&a), "INTV:Astrosmash MD:Sonic SNES:Zelda ARCADE:mslug GBA:Alpha GW:donkykong");
    // SELECT on Favorites: last played (Zelda 80, Sonic 50, mslug 10, the rest never).
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.slot_text().as_deref(), Some("LAST PLAYED"));
    assert_eq!(fav_order(&a), "SNES:Zelda MD:Sonic ARCADE:mslug GBA:Alpha INTV:Astrosmash GW:donkykong");
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.slot_text().as_deref(), Some("MOST PLAYED"));
    assert_eq!(a.filter_text(), Some("MOST PLAYED"));
    a.apply(Action::GbaDown(Btn::Select));
    assert_eq!(a.slot_text().as_deref(), Some("ADDED"));
    assert_eq!(a.filter_text(), Some("ADDED"));
    assert_eq!(fav_order(&a), "INTV:Astrosmash MD:Sonic SNES:Zelda ARCADE:mslug GBA:Alpha GW:donkykong");
}

fn snes_order(a: &App) -> String {
    a.system_carts()
        .filter(|c| c.platform == Platform::Snes)
        .map(|c| c.stem.split(" (").next().unwrap())
        .collect::<Vec<_>>()
        .join(" | ")
}

fn ranked_app() -> (tempfile::TempDir, App) {
    let d = tmp_root_with_carts(&["Alpha", "Beta"]);
    for stem in [
        "Chrono Trigger (USA)",
        "Legend of Zelda, The - A Link to the Past (USA)",
        "Not A Real Game (USA)",
        "Super Mario World (USA)",
    ] {
        write_stub(&d, Platform::Snes, stem);
    }
    write_slot_state(d.path(), &SlotState { clock_set: true, ..SlotState::default() }).unwrap();
    let (mut a, _) = app_booting_at(d.path(), 1_800_000_000);
    while a.current_shelf_kind() != ShelfKind::System(Platform::Snes) {
        press(&mut a, Btn::R1);
    }
    (d, a)
}

#[test]
fn select_steps_a_shelf_through_the_filters_it_has_data_for_and_the_pill_names_each() {
    let (d, mut a) = ranked_app();
    assert_eq!(a.filter_text(), Some("A–Z"));
    let mut seen = Vec::new();
    for _ in 0..7 {
        a.apply(Action::GbaDown(Btn::Select));
        seen.push(a.filter_text().unwrap());
        assert_eq!(a.slot_text().as_deref(), a.filter_text());
        assert_eq!(a.filter_pill_text(), a.filter_text().filter(|t| *t != "A–Z"));
        if a.filter_text() == Some("TOP 100") {
            // Only the games the rankings know, best first, numbered on the title card.
            assert_eq!(snes_order(&a).split(" | ").count(), 3, "{}", snes_order(&a));
            assert!(!snes_order(&a).contains("Not A Real Game"));
            assert!(a.title_card_text().unwrap().starts_with("#1 "));
        }
        if a.filter_text() == Some("BEST SELLING") {
            assert!(snes_order(&a).starts_with("Super Mario World"), "{}", snes_order(&a));
        }
        if a.filter_text() == Some("FIRST PARTY") {
            assert_eq!(snes_order(&a), "Legend of Zelda, The - A Link to the Past | Super Mario World");
            assert!(!a.title_card_text().unwrap().starts_with('#'));
        }
    }
    assert_eq!(
        seen,
        ["LAST PLAYED", "MOST PLAYED", "TOP 100", "POPULAR", "BEST SELLING", "FIRST PARTY", "A–Z"]
    );
    // Each shelf keeps its own: SNES on TOP 100, and the GBA shelf (no data for its games)
    // still A–Z, and SELECT there skips the filters it has nothing for.
    for _ in 0..3 {
        a.apply(Action::GbaDown(Btn::Select));
    }
    assert_eq!(a.filter_text(), Some("TOP 100"));
    a.settle_saves();
    let state = read_slot_state(d.path());
    assert_eq!(state.shelf_views[Platform::Snes.index()], Some(slot_store::ShelfView::Top100));
    assert_eq!(state.shelf_views[Platform::Gba.index()], None);
    press(&mut a, Btn::L1);
    assert_eq!(a.current_shelf_kind(), ShelfKind::System(Platform::Gba));
    assert_eq!(a.filter_text(), Some("A–Z"));
    for want in ["LAST PLAYED", "MOST PLAYED", "A–Z"] {
        a.apply(Action::GbaDown(Btn::Select));
        assert_eq!(a.filter_text(), Some(want));
    }
    // Back after a restart: the SNES shelf is still on TOP 100.
    a.settle_saves();
    let (mut b, _) = app_booting_at(d.path(), 1_800_000_100);
    while b.current_shelf_kind() != ShelfKind::System(Platform::Snes) {
        press(&mut b, Btn::R1);
    }
    assert_eq!(b.filter_text(), Some("TOP 100"));
    assert_eq!(snes_order(&b).split(" | ").count(), 3);
}

#[test]
fn y_on_the_favorites_shelf_removes_the_game_and_the_empty_shelf_leaves() {
    let library = "GBA\tAlpha\t0\t0\t1\nFAVADD\tGBA\tAlpha\t10\n";
    let (d, mut a) = mixed_favorites_app(library, SlotState::default());
    go_favorites(&mut a);
    assert_eq!(a.selected_stem(), Some("Alpha"));
    a.apply(Action::GbaDown(Btn::Y));
    a.settle_saves();
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(!Library::read(d.path()).get(Platform::Gba, "Alpha").favorite);
    assert!(a.favorite_carts().next().is_none());
    assert_ne!(a.current_shelf_kind(), ShelfKind::Favorites);
}

#[test]
fn the_favorites_shelf_can_be_hidden_from_systems_and_stays_hidden() {
    let library = "GBA\tAlpha\t0\t0\t1\n";
    let (d, mut a) = mixed_favorites_app(library, SlotState::default());
    go_favorites(&mut a);
    assert_eq!(fav_order(&a), "GBA:Alpha");
    open_systems_at(&mut a, QuickRow::ShowFavorites);
    assert_eq!(a.quick_value(QuickRow::ShowFavorites), Some(slot_ui::QuickValue::On));
    press(&mut a, Btn::Right);
    assert_eq!(a.quick_value(QuickRow::ShowFavorites), Some(slot_ui::QuickValue::Off));
    assert!(a.favorite_carts().next().is_none());
    assert!(!read_slot_state(d.path()).show_favorites);
    let (_d2, b) = mixed_favorites_app(library, read_slot_state(d.path()));
    assert!(b.favorite_carts().next().is_none());
    assert_eq!(b.current_shelf_kind(), ShelfKind::System(Platform::Gba));
}

#[test]
fn all_systems_can_be_hidden_when_the_favorites_shelf_remains() {
    let library = "GBA\tAlpha\t0\t0\t1\nMD\tSonic\t0\t0\t1\n";
    let mut state = SlotState::default();
    // Hide every system that will have games; Favorites stays.
    for p in [Platform::Gba, Platform::Gb, Platform::Md, Platform::Snes, Platform::Intv, Platform::Arcade, Platform::Gw] {
        state.hidden_systems[p.index()] = true;
    }
    let (_d, a) = mixed_favorites_app(library, state);
    assert_eq!(order(&a), "", "system shelves are hidden");
    assert_eq!(fav_order(&a), "GBA:Alpha MD:Sonic");
    assert_eq!(a.current_shelf_kind(), ShelfKind::Favorites);
}

#[test]
fn launching_from_the_favorites_shelf_keeps_the_games_own_platform() {
    let library = "MD\tSonic\t0\t0\t1\nGBA\tAlpha\t0\t0\t1\nFAVADD\tMD\tSonic\t20\nFAVADD\tGBA\tAlpha\t10\n";
    let (_d, mut a) = mixed_favorites_app(library, SlotState::default());
    go_favorites(&mut a);
    assert_eq!(a.selected_cart().map(|c| c.platform), Some(Platform::Md));
    assert_eq!(a.selected_stem(), Some("Sonic"));
    // Play time and starring use the cart's platform, not a shelf-wide one.
    a.apply(Action::GbaDown(Btn::Y));
    a.settle_saves();
    // Sonic left the Favorites shelf; Alpha remains, still a GBA cart.
    assert_eq!(fav_order(&a), "GBA:Alpha");
    assert_eq!(a.selected_cart().map(|c| c.platform), Some(Platform::Gba));
}
