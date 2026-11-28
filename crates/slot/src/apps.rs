//! Apps the player installs on the card themselves, which Slot opens in its own place.
//!
//! GrainBrick (https://github.com/ETRNLIAM/GrainBrick, a granular ambient sound instrument) is
//! free to download but not open source, and its terms do not allow passing it on, so Slot does
//! not ship it: HOME → Apps → GrainBrick opens it only once the player has put it on the card, at
//! `Slot/Apps/GrainBrick/` (the contents of the release's `grainbrick-beta2/` folder, with its
//! `main.py`) or as the release extracts it, `Slot/Apps/grainbrick-beta2/`.
//!
//! It is a Python 3.12 program using pygame and its own NumPy, so it also needs a Python with
//! pygame: `Slot/Apps/python/bin/python3` (the runtime the Brick card package carries,
//! `brick/python-runtime`), else the system's `python3`. Without both, the row says Unavailable.
//!
//! Opening it: Slot settles its saves and replaces itself with `System/run-app.sh`, so the
//! screen, the sound card and the buttons are all let go; the script runs the app and then
//! starts Slot again in the same process, which launch.sh (stock) or Slot OS's init is waiting on.

use std::path::{Path, PathBuf};

/// The folders GrainBrick may be installed in, under the Slot folder.
const GRAINBRICK_DIRS: [&str; 2] = ["Apps/GrainBrick", "Apps/grainbrick-beta2"];

/// Where a Python for it may be, in order: the player's own runtime first.
const PYTHONS: [&str; 3] = ["Apps/python/bin/python3", "/usr/bin/python3", "/usr/local/bin/python3"];

/// GrainBrick's folder, if the player has installed it.
pub fn grainbrick(root: &Path) -> Option<PathBuf> {
    GRAINBRICK_DIRS
        .iter()
        .map(|d| root.join(d))
        .find(|d| d.join("main.py").is_file())
}

/// A Python to run it with: a relative entry is under the Slot folder.
pub fn python(root: &Path) -> Option<PathBuf> {
    PYTHONS
        .iter()
        .map(|p| if p.starts_with('/') { PathBuf::from(p) } else { root.join(p) })
        .find(|p| p.is_file())
}

/// Replaces Slot with `System/run-app.sh <app> <python>`. Returns only if that could not start.
#[cfg(all(feature = "brick", not(feature = "sim")))]
pub fn exec(root: &Path, app: &Path, python: &Path) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("/bin/sh")
        .arg(root.join("System/run-app.sh"))
        .arg(app)
        .arg(python)
        .env("SLOT_ROOT", root)
        .current_dir(root)
        .exec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn found_only_when_installed() {
        let dir = std::env::temp_dir().join(format!("slot-apps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("Apps/grainbrick-beta2")).unwrap();
        assert_eq!(grainbrick(&dir), None);
        std::fs::write(dir.join("Apps/grainbrick-beta2/main.py"), "").unwrap();
        assert_eq!(grainbrick(&dir), Some(dir.join("Apps/grainbrick-beta2")));
        std::fs::create_dir_all(dir.join("Apps/GrainBrick")).unwrap();
        std::fs::write(dir.join("Apps/GrainBrick/main.py"), "").unwrap();
        assert_eq!(grainbrick(&dir), Some(dir.join("Apps/GrainBrick")));
        std::fs::create_dir_all(dir.join("Apps/python/bin")).unwrap();
        std::fs::write(dir.join("Apps/python/bin/python3"), "").unwrap();
        assert_eq!(python(&dir), Some(dir.join("Apps/python/bin/python3")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
