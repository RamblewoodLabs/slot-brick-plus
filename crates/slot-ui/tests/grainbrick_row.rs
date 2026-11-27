//! System → GrainBrick: on the Brick's System page only while GrainBrick is installed.
//! Its own test binary, as it flips a process-wide switch the other menu tests read.
use slot_ui::{set_grainbrick, QuickRow};

#[test]
#[cfg(all(feature = "brick", not(feature = "rg35xxsp")))]
fn grainbrick_row_follows_the_install() {
    set_grainbrick(false);
    assert_eq!(QuickRow::system(), &QuickRow::SYSTEM[..]);
    assert!(!QuickRow::system().contains(&QuickRow::GrainBrick));
    set_grainbrick(true);
    assert!(QuickRow::system().contains(&QuickRow::GrainBrick));
    assert_eq!(QuickRow::GrainBrick.page(), QuickRow::system());
    assert_eq!(QuickRow::GrainBrick.parent(), Some(QuickRow::System));
    assert_eq!(QuickRow::System.child(), Some(QuickRow::Lights));
    assert_eq!(QuickRow::GrainBrick.label(), "GrainBrick");
    // The rows around it keep their page.
    assert_eq!(QuickRow::About.page(), QuickRow::system());
    set_grainbrick(false);
}

#[test]
#[cfg(feature = "rg35xxsp")]
fn the_sp_never_shows_it() {
    set_grainbrick(true);
    assert!(!QuickRow::system().contains(&QuickRow::GrainBrick));
}
