mod common;

use common::{app_booting_at, tmp_root_with_carts, write_gb_cart};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, SlotState};
use slot_ui::QuickRow;

fn press(app: &mut App, btn: Btn) {
    app.apply(Action::GbaDown(btn));
    app.apply(Action::GbaUp(btn));
}

fn save_video(root: &std::path::Path, id: &str, title: &str) {
    std::fs::create_dir_all(root.join("Videos")).unwrap();
    std::fs::write(root.join("Videos").join(format!("{id}.mp4")), b"mp4").unwrap();
    std::fs::write(
        root.join("Videos/index.txt"),
        format!("{id}\t{title}\tChannel\t3\t60000\t12000\n"),
    )
    .unwrap();
}

fn boot(root: &std::path::Path, state: SlotState) -> App {
    write_slot_state(
        root,
        &SlotState {
            clock_set: true,
            ..state
        },
    )
    .unwrap();
    app_booting_at(root, 1_800_000_000).0
}

fn open_systems_at(app: &mut App, row: QuickRow) {
    while app.quick_menu().is_some() {
        press(app, Btn::B);
    }
    app.apply(Action::QuickMenu);
    for _ in 0..QuickRow::Library.position() {
        press(app, Btn::Down);
    }
    press(app, Btn::A);
    for _ in 0..QuickRow::LibrarySystems.position() {
        press(app, Btn::Down);
    }
    press(app, Btn::A);
    for _ in 0..row.position() {
        press(app, Btn::Down);
    }
    assert_eq!(app.quick_menu(), Some(row));
}

#[test]
fn an_empty_cache_has_no_videos_shelf() {
    let root = tmp_root_with_carts(&["Emerald", "Fusion"]);
    let mut app = boot(root.path(), SlotState::default());
    assert_ne!(app.slot_text().as_deref(), Some("Videos"));
    press(&mut app, Btn::R1);
    assert_ne!(app.slot_text().as_deref(), Some("Videos"));
    assert!(matches!(app.phase(), Phase::Shelf));
}

#[test]
fn l1_and_r1_reach_the_videos_shelf_and_a_plays_without_inserting_a_cart() {
    let root = tmp_root_with_carts(&["Emerald", "Fusion"]);
    save_video(root.path(), "abcdefghij", "Concert");
    let mut app = boot(root.path(), SlotState::default());
    assert_eq!(app.slot_text().as_deref(), Some("Game Boy Advance"));
    press(&mut app, Btn::R1);
    assert_eq!(app.slot_text().as_deref(), Some("Videos"));
    assert_eq!(app.title_card_text().as_deref(), Some("Concert"));
    press(&mut app, Btn::A);
    assert!(app.videos_open(), "A plays the video");
    assert!(
        matches!(app.phase(), Phase::Shelf),
        "A does not insert a cart"
    );
}

#[test]
fn y_asks_then_deletes_the_video_and_the_shelf_goes_away() {
    let root = tmp_root_with_carts(&["Emerald", "Fusion"]);
    save_video(root.path(), "abcdefghij", "Concert");
    let mut app = boot(root.path(), SlotState::default());
    press(&mut app, Btn::R1);
    press(&mut app, Btn::Y);
    assert!(root.path().join("Videos/abcdefghij.mp4").is_file());
    press(&mut app, Btn::Y);
    assert!(!root.path().join("Videos/abcdefghij.mp4").is_file());
    assert_ne!(app.slot_text().as_deref(), Some("Videos"));
    assert!(matches!(app.phase(), Phase::Shelf));
}

#[test]
fn library_systems_can_hide_the_videos_shelf() {
    let root = tmp_root_with_carts(&["Emerald"]);
    write_gb_cart(&root, "Pocket", "POCKET");
    save_video(root.path(), "abcdefghij", "Concert");
    let mut app = boot(root.path(), SlotState::default());
    open_systems_at(&mut app, QuickRow::ShowVideos);
    assert_eq!(
        app.quick_value(QuickRow::ShowVideos),
        Some(slot_ui::QuickValue::On)
    );
    press(&mut app, Btn::Right);
    assert_eq!(
        app.quick_value(QuickRow::ShowVideos),
        Some(slot_ui::QuickValue::Off)
    );
    for _ in 0..3 {
        press(&mut app, Btn::B);
    }
    assert!(matches!(app.phase(), Phase::Shelf));
    for _ in 0..4 {
        press(&mut app, Btn::R1);
        assert_ne!(app.slot_text().as_deref(), Some("Videos"));
    }
}

#[test]
fn videos_are_the_shelf_when_the_library_has_no_games() {
    let root = tmp_root_with_carts(&[]);
    save_video(root.path(), "abcdefghij", "Concert");
    let app = boot(root.path(), SlotState::default());
    assert_eq!(app.slot_text().as_deref(), Some("Videos"));
    assert_eq!(app.title_card_text().as_deref(), Some("Concert"));
}
