//! Anbernic RG280V: OpenDingux, 320×480 portrait panel, no Wi-Fi.
//! The user-facing name is Slotted.

use std::path::{Path, PathBuf};

pub const PRODUCT: &str = "Slotted";

/// Candidate Slot library roots on a typical Adam / OpenDingux install.
/// TF2 is usually `/media/sdcard` or `/media/ext`; the library folder stays `Slot/`
/// so a card can be shared with the Brick / SP.
pub const ROOT_CANDIDATES: &[&str] = &[
    "/media/sdcard/Slot",
    "/media/ext/Slot",
    "/media/mmcblk1p1/Slot",
    "/media/data/Slot",
];

/// The library root: `SLOT_ROOT` if set, else the first candidate that exists
/// (preferring one that already has a `Games` folder), else `/media/sdcard/Slot`.
pub fn find_root() -> PathBuf {
    if let Some(v) = std::env::var_os("SLOT_ROOT") {
        return PathBuf::from(v);
    }
    let mut fallback = PathBuf::from(ROOT_CANDIDATES[0]);
    for c in ROOT_CANDIDATES {
        let p = Path::new(c);
        if p.join("Games").is_dir() {
            return p.to_path_buf();
        }
        if p.is_dir() {
            fallback = p.to_path_buf();
        }
    }
    fallback
}

/// Orientation files, first existing wins after the env vars in `slot_gfx::load_orientation`.
pub fn orientation_files(root: &Path) -> [PathBuf; 2] {
    [
        root.join("Config/orientation"),
        root.join("Config/slotted-orientation"),
    ]
}

pub fn apply_orientation(root: &Path) {
    let files = orientation_files(root);
    let refs: Vec<&Path> = files.iter().map(PathBuf::as_path).collect();
    let o = slot_gfx::load_orientation(&refs);
    slot_gfx::set_orientation(o);
    eprintln!("slotted: orientation {}", o.as_str());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_facing_name_is_slotted() {
        assert_eq!(PRODUCT, "Slotted");
    }

    #[test]
    fn slot_root_env_wins() {
        let prev = std::env::var_os("SLOT_ROOT");
        std::env::set_var("SLOT_ROOT", "/tmp/slotted-test-root");
        let found = find_root();
        match prev {
            Some(v) => std::env::set_var("SLOT_ROOT", v),
            None => std::env::remove_var("SLOT_ROOT"),
        }
        assert_eq!(found, PathBuf::from("/tmp/slotted-test-root"));
    }
}
