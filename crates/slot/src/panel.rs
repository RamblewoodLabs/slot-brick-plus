//! Bring the H700 panel back after sleep. The kernel's mem suspend, and BaseOS's
//! Super Standby, leave the framebuffer blank and the backlight off.

use std::fs;
use std::path::Path;

/// Unblank fb0 and clear `bl_power` on every backlight class device.
/// Brightness itself is restored by the power driver when the lid opens.
pub fn unblank() {
    let blank = Path::new("/sys/class/graphics/fb0/blank");
    if blank.is_file() {
        if let Err(e) = fs::write(blank, "0\n") {
            eprintln!("slot: panel blank: {e}");
        }
    }
    let class = Path::new("/sys/class/backlight");
    let Ok(entries) = fs::read_dir(class) else {
        return;
    };
    for entry in entries.flatten() {
        let power = entry.path().join("bl_power");
        if power.is_file() {
            if let Err(e) = fs::write(&power, "0\n") {
                eprintln!("slot: {}: {e}", power.display());
            }
        }
    }
}
