//! HOME → Apps → GrainBrick: the Brick's HOME has Apps, which lists GrainBrick; the SP's has neither.
use slot_ui::QuickRow;

#[test]
#[cfg(all(feature = "brick", not(feature = "rg35xxsp")))]
fn apps_is_on_the_brick_home_and_lists_grainbrick() {
    assert!(QuickRow::MAIN.contains(&QuickRow::Apps));
    assert_eq!(QuickRow::Apps.child(), Some(QuickRow::GrainBrick));
    assert_eq!(QuickRow::GrainBrick.page(), &QuickRow::APPS[..]);
    assert_eq!(QuickRow::GrainBrick.parent(), Some(QuickRow::Apps));
    assert_eq!(QuickRow::Apps.page(), QuickRow::home());
    assert!(QuickRow::Apps.opens() && QuickRow::GrainBrick.opens());
    assert!(!QuickRow::SYSTEM.contains(&QuickRow::GrainBrick));
    assert_eq!(QuickRow::Apps.label(), "Apps");
    assert_eq!(QuickRow::GrainBrick.label(), "GrainBrick");
}

#[test]
#[cfg(feature = "rg35xxsp")]
fn the_sp_has_no_apps() {
    assert!(!QuickRow::MAIN.contains(&QuickRow::Apps));
}
