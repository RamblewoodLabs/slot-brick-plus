//! Anbernic RG35XX SP: Allwinner H700, 640×480, lid, TF1 for BaseOS and TF2 for Slot.
//!
//! The Brick draws this same 640×480 layout and scales it to 1024×768 (exactly 8/5).
//! On the SP the panel is already 640×480, so the picture is 1:1. The card layout
//! matches the Brick (`Slot/Games`, `Saves`, `States`, `BIOS`, `Config`) so a
//! library can be copied across. See `docs/rg35xxsp.md`.

/// Native panel. Also the logical layout size (`slot_gfx::OUT_W` / `OUT_H` on a brick build).
pub const PANEL_W: u32 = 640;
pub const PANEL_H: u32 = 480;

/// Where BaseOS mounts the frontend card (TF2 when it is present, otherwise TF1's
/// FAT volume). Slot's own files live in `Slot/` under that mount, which is what
/// `SLOT_ROOT` is set to.
pub const CARD_ROOT: &str = "/mnt/sdcard/Slot";

/// BaseOS also symlinks this name at the same mount, for older frontends.
pub const CARD_ALIAS: &str = "/mnt/SDCARD/Slot";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_brick_panel_is_this_one_scaled_by_eight_fifths() {
        assert_eq!(PANEL_W * 8 / 5, 1024);
        assert_eq!(PANEL_H * 8 / 5, 768);
        assert_eq!((PANEL_W * 8) % 5, 0);
        assert_eq!((PANEL_H * 8) % 5, 0);
    }

    #[test]
    fn games_live_in_the_brick_slot_folder() {
        assert!(CARD_ROOT.ends_with("/Slot"));
        assert!(CARD_ALIAS.ends_with("/Slot"));
    }
}
