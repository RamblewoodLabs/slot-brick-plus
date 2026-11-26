use std::path::{Path, PathBuf};

/// Every platform in Platform::ALL has its four folders here (tests/dist.rs checks).
pub const DIRS: [&str; 130] = [
    "BIOS",
    "Config",
    "Fonts",
    "Games",
    "Games/GBA",
    "Games/GB",
    "Games/GBC",
    "Games/NES",
    "Games/SNES",
    "Games/PS1",
    "Games/MD",
    "Games/SMS",
    "Games/GG",
    "Games/ARCADE",
    "Games/PCE",
    "Games/A2600",
    "Games/INTV",
    "Games/LYNX",
    "Games/A7800",
    "Games/GW",
    "Games/COLECO",
    "Games/A5200",
    "Games/A800",
    "Games/VECTREX",
    "Games/32X",
    "Games/N64",
    "Games/NDS",
    "Games/PSP",
    "Labels",
    "Labels/GBA",
    "Labels/GB",
    "Labels/GBC",
    "Labels/NES",
    "Labels/SNES",
    "Labels/PS1",
    "Labels/MD",
    "Labels/SMS",
    "Labels/GG",
    "Labels/ARCADE",
    "Labels/PCE",
    "Labels/A2600",
    "Labels/INTV",
    "Labels/LYNX",
    "Labels/A7800",
    "Labels/GW",
    "Labels/COLECO",
    "Labels/A5200",
    "Labels/A800",
    "Labels/VECTREX",
    "Labels/32X",
    "Labels/N64",
    "Labels/NDS",
    "Labels/PSP",
    "Overlays",
    "Overlays/GBA",
    "Overlays/GB",
    "Overlays/GBC",
    "Overlays/NES",
    "Overlays/SNES",
    "Overlays/PS1",
    "Overlays/MD",
    "Overlays/SMS",
    "Overlays/GG",
    "Overlays/ARCADE",
    "Overlays/PCE",
    "Overlays/A2600",
    "Overlays/INTV",
    "Overlays/LYNX",
    "Overlays/A7800",
    "Overlays/GW",
    "Overlays/COLECO",
    "Overlays/A5200",
    "Overlays/A800",
    "Overlays/VECTREX",
    "Overlays/32X",
    "Overlays/N64",
    "Overlays/NDS",
    "Overlays/PSP",
    "Saves",
    "Saves/GBA",
    "Saves/GB",
    "Saves/GBC",
    "Saves/NES",
    "Saves/SNES",
    "Saves/PS1",
    "Saves/MD",
    "Saves/SMS",
    "Saves/GG",
    "Saves/ARCADE",
    "Saves/PCE",
    "Saves/A2600",
    "Saves/INTV",
    "Saves/LYNX",
    "Saves/A7800",
    "Saves/GW",
    "Saves/COLECO",
    "Saves/A5200",
    "Saves/A800",
    "Saves/VECTREX",
    "Saves/32X",
    "Saves/N64",
    "Saves/NDS",
    "Saves/PSP",
    "States",
    "States/GBA",
    "States/GB",
    "States/GBC",
    "States/NES",
    "States/SNES",
    "States/PS1",
    "States/MD",
    "States/SMS",
    "States/GG",
    "States/ARCADE",
    "States/PCE",
    "States/A2600",
    "States/INTV",
    "States/LYNX",
    "States/A7800",
    "States/GW",
    "States/COLECO",
    "States/A5200",
    "States/A800",
    "States/VECTREX",
    "States/32X",
    "States/N64",
    "States/NDS",
    "States/PSP",
    "System",
    "Wallpapers",
];

pub fn ensure(root: &Path) {
    for sub in DIRS {
        let _ = std::fs::create_dir_all(root.join(sub));
    }
    if let Err(e) = slot_store::move_config(root) {
        eprintln!("slot: config: {e}");
    }
}

pub fn bios_dir(root: &Path) -> PathBuf {
    root.join("BIOS")
}

const BIOS_FILE: &str = "gba_bios.bin";

const BIOS_BYTES: u64 = 16 * 1024;
const BIOS_FIRST_BYTE: u8 = 0x18;

pub fn has_real_bios(root: &Path) -> bool {
    let Ok(mut f) = std::fs::File::open(bios_dir(root).join(BIOS_FILE)) else {
        return false;
    };
    if !f.metadata().is_ok_and(|m| m.len() == BIOS_BYTES) {
        return false;
    }
    let mut first = [0u8; 1];
    std::io::Read::read_exact(&mut f, &mut first).is_ok() && first[0] == BIOS_FIRST_BYTE
}

pub fn saves_dir(root: &Path) -> PathBuf {
    root.join("Saves")
}

/// Space on the card that holds `path`, in bytes: (free, size). Free is what the player can
/// still fill, the filesystem's own reserve left out.
pub fn card_space(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut st: libc::statvfs = unsafe { std::mem::zeroed() };
    // SAFETY: `c` is a NUL-terminated path and `st` a statvfs for the call to fill.
    if unsafe { libc::statvfs(c.as_ptr(), &mut st) } != 0 {
        return None;
    }
    let block = st.f_frsize as u64;
    Some((
        (st.f_bavail as u64).saturating_mul(block),
        (st.f_blocks as u64).saturating_mul(block),
    ))
}
