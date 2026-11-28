//! A card that still has Slot/Config/companion.txt from before the Companion was removed:
//! Slot does not read it, and starts exactly as it does without it.
mod common;

use common::{app_booting_at, tmp_root_with_carts, CLOCK_IS_SET};

#[test]
fn a_leftover_companion_file_is_ignored() {
    let clean = tmp_root_with_carts(&["Emerald"]);
    let (clean_app, _clock) = app_booting_at(clean.path(), CLOCK_IS_SET);

    let root = tmp_root_with_carts(&["Emerald"]);
    let config = root.path().join("Slot").join("Config");
    std::fs::create_dir_all(&config).unwrap();
    std::fs::write(config.join("companion.txt"), "xp=40\nlevel=3\nnot a valid line\n\0\0").unwrap();
    let (app, _clock) = app_booting_at(root.path(), CLOCK_IS_SET);

    assert_eq!(
        std::mem::discriminant(app.phase()),
        std::mem::discriminant(clean_app.phase())
    );
    assert!(config.join("companion.txt").exists(), "Slot leaves the file alone");
}
