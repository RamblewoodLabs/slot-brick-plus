mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};
use slot::app::{App, Phase};
use slot_input::{Action, Btn};
use slot_store::{write_slot_state, Companion, SlotState};

fn on_carousel() -> (tempfile::TempDir, App) {
    let root = tmp_root_with_carts(&["Emerald", "Fusion"]);
    write_slot_state(
        root.path(),
        &SlotState {
            clock_set: true,
            ..SlotState::default()
        },
    )
    .unwrap();
    let (app, _clock) = app_booting_at(root.path(), CLOCK_IS_SET);
    (root, app)
}

fn press(a: &mut App, btn: Btn) {
    a.apply(Action::GbaDown(btn));
}

#[test]
fn x_opens_pip_and_b_closes_it() {
    let (_d, mut app) = on_carousel();
    press(&mut app, Btn::X);
    assert!(app.pip_open());
    press(&mut app, Btn::B);
    assert!(!app.pip_open());
    assert!(matches!(app.phase(), Phase::Shelf));
}

#[test]
fn the_home_menu_opens_videos_and_pip() {
    let (_d, mut app) = on_carousel();
    app.apply(Action::QuickMenu);
    for _ in 0..3 {
        press(&mut app, Btn::Down);
    }
    press(&mut app, Btn::A);
    assert!(app.pip_open());
    press(&mut app, Btn::B);
    press(&mut app, Btn::Down);
    press(&mut app, Btn::A);
    assert!(app.videos_open());
    press(&mut app, Btn::B);
    assert!(!app.videos_open());
}

#[test]
fn saved_companion_xp_is_read_at_startup() {
    let root = tmp_root_with_carts(&["Emerald"]);
    let mut pip = Companion::default();
    pip.xp = 40;
    pip.write(root.path()).unwrap();
    let (app, _clock) = app_booting_at(root.path(), CLOCK_IS_SET);
    assert_eq!(app.companion_xp(), 40);
}
