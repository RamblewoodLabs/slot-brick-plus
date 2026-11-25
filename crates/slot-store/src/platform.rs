//! One row per system. Adding a shelf is a `Platform` variant plus a `DEFS` row:
//! folder, extensions, whether a zip is unpacked or is the game itself, the libretro
//! core, the BIOS files, the button legend, and (for a shell drawn from a PNG) the
//! asset directory under `assets/carts/`. The original six are drawn in code; their
//! `shell` is `None`.
use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum Platform {
    #[default]
    Gba = 0,
    Gb,
    Gbc,
    Nes,
    Snes,
    Ps1,
    Md,
    Sms,
    Gg,
    Arcade,
    Pce,
    A2600,
    Intv,
    Lynx,
    A7800,
}

/// Which saved shader a system follows. New television systems share the NES one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShaderFamily {
    Gba,
    Gb,
    Nes,
    Snes,
    Ps1,
}

/// Whose colour-correction switch applies. A television has none.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ColourCorrect {
    None,
    Gb,
    Gba,
}

/// A file the core reads from `Slot/BIOS`. `required` means the game does not start without it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BiosFile {
    pub name: &'static str,
    pub required: bool,
}

/// Which physical bit (a libretro joypad mask) feeds each libretro button.
/// Identity leaves the pad as Slot already sends it. The legend is what those buttons
/// mean on that console.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PadMap {
    pub from: [u16; 14],
    pub legend: &'static str,
}

const fn pad(legend: &'static str) -> PadMap {
    PadMap {
        from: [
            1 << 0,
            1 << 1,
            1 << 2,
            1 << 3,
            1 << 4,
            1 << 5,
            1 << 6,
            1 << 7,
            1 << 8,
            1 << 9,
            1 << 10,
            1 << 11,
            1 << 12,
            1 << 13,
        ],
        legend,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlatformDef {
    pub id: Platform,
    pub name: &'static str,
    pub dir: &'static str,
    pub extensions: &'static [&'static str],
    /// A `.zip` or `.7z` of one ROM is unpacked and played. A PlayStation disc is not.
    pub archives: bool,
    /// The archive itself is the game (an arcade zip). It is not unpacked.
    pub archive_is_rom: bool,
    pub picture: (u32, u32),
    pub canvas: (u32, u32),
    /// `Core::as_str` of the core that runs it.
    pub core: &'static str,
    pub options: &'static [(&'static str, &'static str)],
    pub bios: &'static [BiosFile],
    pub pad: PadMap,
    /// Directory under `assets/carts/` (`shell.png`, `meta.toml`, optional `mask.png`).
    pub shell: Option<&'static str>,
    /// Other shelves whose labels are worth trying, after this one's.
    pub label_with: &'static [Platform],
    pub shader: ShaderFamily,
    pub colour: ColourCorrect,
    /// L1 and R1 belong to Slot, not the game.
    pub shoulders_are_slot: bool,
    /// X and Y are the console's own buttons, not turbo.
    pub native_xy: bool,
    pub rewind: bool,
    /// Every frame is stretched over the canvas.
    pub fill: bool,
    /// L2 and R2 are the game's.
    pub triggers: bool,
    pub dualshock: bool,
    /// A DMG-only Game Boy can take a palette.
    pub gb_palette: bool,
    /// RetroAchievements console id. 0 means Slot does not ask.
    pub ra: u32,
}

const GB_WITH: [Platform; 1] = [Platform::Gbc];
const GBC_WITH: [Platform; 1] = [Platform::Gb];

const MD_OPT: &[(&str, &str)] = &[("genesis_plus_gx_system_hw", "mega drive / genesis")];
const SMS_OPT: &[(&str, &str)] = &[("genesis_plus_gx_system_hw", "master system")];
const GG_OPT: &[(&str, &str)] = &[("genesis_plus_gx_system_hw", "game gear")];

const NEOGEO: &[BiosFile] = &[BiosFile {
    name: "neogeo.zip",
    required: false,
}];
const INTV_BIOS: &[BiosFile] = &[
    BiosFile {
        name: "exec.bin",
        required: true,
    },
    BiosFile {
        name: "grom.bin",
        required: true,
    },
];
const LYNX_BIOS: &[BiosFile] = &[BiosFile {
    name: "lynxboot.img",
    required: true,
}];

const PAD_STANDARD: PadMap =
    pad("A and B are the face buttons. L and R are the shoulders when the console has them.");
const PAD_GENESIS: PadMap =
    pad("Six buttons: B is B, A is C, Y is A, X is Y, L is X, R is Z. Select is Mode.");
const PAD_SMS: PadMap = pad("B is button 1, A is button 2. Start is pause.");
const PAD_GG: PadMap = pad("B is button 1, A is button 2. Start is Start.");
const PAD_ARCADE: PadMap =
    pad("A B X Y L R are the panel buttons. Select inserts a coin, Start starts.");
const PAD_PCE: PadMap = pad("B is II, A is I. Select is Select, Start is Run.");
const PAD_2600: PadMap = pad(
    "B is fire. Select is Select, Start is Reset. L is the left difficulty switch, R the right.",
);
const PAD_INTV: PadMap = pad(
    "The D-pad is the disc. A is the left action button, B the right, Y the top. Hold L or R and the core draws its keypad: the D-pad moves the cursor, and A, B, X or Y presses that key. X repeats the last key. L2 is Clear, R2 is Enter. Start pauses. Select swaps the two controllers.",
);
const PAD_LYNX: PadMap =
    pad("A and B are the face buttons. L is Option 1, R is Option 2. Start pauses.");
const PAD_7800: PadMap =
    pad("B is fire. Select is Select, Start is Pause. L and R are the difficulty switches.");

const DEFS: [PlatformDef; 15] = [
    PlatformDef {
        id: Platform::Gba,
        name: "Game Boy Advance",
        dir: "GBA",
        extensions: &["gba"],
        archives: true,
        archive_is_rom: false,
        picture: (240, 160),
        canvas: (240, 160),
        core: "mgba",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &[],
        shader: ShaderFamily::Gba,
        colour: ColourCorrect::Gba,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 5,
    },
    PlatformDef {
        id: Platform::Gb,
        name: "Game Boy",
        dir: "GB",
        extensions: &["gb", "gbc"],
        archives: true,
        archive_is_rom: false,
        picture: (160, 144),
        canvas: (240, 160),
        core: "mgba",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &GB_WITH,
        shader: ShaderFamily::Gb,
        colour: ColourCorrect::Gb,
        shoulders_are_slot: true,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: true,
        ra: 4,
    },
    PlatformDef {
        id: Platform::Gbc,
        name: "Game Boy Color",
        dir: "GBC",
        extensions: &["gb", "gbc"],
        archives: true,
        archive_is_rom: false,
        picture: (160, 144),
        canvas: (240, 160),
        core: "mgba",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &GBC_WITH,
        shader: ShaderFamily::Gb,
        colour: ColourCorrect::Gb,
        shoulders_are_slot: true,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: true,
        ra: 6,
    },
    PlatformDef {
        id: Platform::Nes,
        name: "Nintendo Entertainment System",
        dir: "NES",
        extensions: &["nes"],
        archives: true,
        archive_is_rom: false,
        picture: (256, 224),
        canvas: (256, 240),
        core: "fceumm",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: true,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 7,
    },
    PlatformDef {
        id: Platform::Snes,
        name: "Super Nintendo Entertainment System",
        dir: "SNES",
        extensions: &["sfc", "smc"],
        archives: true,
        archive_is_rom: false,
        picture: (256, 224),
        canvas: (256, 240),
        core: "snes9x",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &[],
        shader: ShaderFamily::Snes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: true,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 3,
    },
    PlatformDef {
        id: Platform::Ps1,
        name: "PlayStation",
        dir: "PS1",
        extensions: &["cue", "chd", "pbp", "m3u", "bin", "iso", "exe"],
        archives: false,
        archive_is_rom: false,
        picture: (640, 480),
        canvas: (640, 480),
        core: "pcsx_rearmed",
        options: &[],
        bios: &[],
        pad: PAD_STANDARD,
        shell: None,
        label_with: &[],
        shader: ShaderFamily::Ps1,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: true,
        rewind: false,
        fill: true,
        triggers: true,
        dualshock: true,
        gb_palette: false,
        ra: 12,
    },
    PlatformDef {
        id: Platform::Md,
        name: "Genesis / Mega Drive",
        dir: "MD",
        extensions: &["md", "gen", "smd", "bin"],
        archives: true,
        archive_is_rom: false,
        picture: (320, 224),
        canvas: (320, 240),
        core: "genesis_plus_gx",
        options: MD_OPT,
        bios: &[],
        pad: PAD_GENESIS,
        shell: Some("md"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: true,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 1,
    },
    PlatformDef {
        id: Platform::Sms,
        name: "Master System",
        dir: "SMS",
        extensions: &["sms"],
        archives: true,
        archive_is_rom: false,
        picture: (256, 192),
        canvas: (256, 240),
        core: "genesis_plus_gx",
        options: SMS_OPT,
        bios: &[],
        pad: PAD_SMS,
        shell: Some("sms"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 11,
    },
    PlatformDef {
        id: Platform::Gg,
        name: "Game Gear",
        dir: "GG",
        extensions: &["gg"],
        archives: true,
        archive_is_rom: false,
        picture: (160, 144),
        canvas: (160, 144),
        core: "genesis_plus_gx",
        options: GG_OPT,
        bios: &[],
        pad: PAD_GG,
        shell: Some("gg"),
        label_with: &[],
        shader: ShaderFamily::Gb,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 15,
    },
    PlatformDef {
        id: Platform::Arcade,
        name: "Arcade",
        dir: "ARCADE",
        extensions: &["zip"],
        archives: false,
        archive_is_rom: true,
        picture: (320, 240),
        canvas: (320, 240),
        core: "fbneo",
        options: &[],
        bios: NEOGEO,
        pad: PAD_ARCADE,
        shell: Some("arcade"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: true,
        rewind: false,
        fill: true,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 27,
    },
    PlatformDef {
        id: Platform::Pce,
        name: "TurboGrafx-16 / PC Engine",
        dir: "PCE",
        extensions: &["pce", "sgx"],
        archives: true,
        archive_is_rom: false,
        picture: (256, 224),
        canvas: (256, 240),
        core: "mednafen_pce_fast",
        options: &[],
        bios: &[],
        pad: PAD_PCE,
        shell: Some("pce"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 8,
    },
    PlatformDef {
        id: Platform::A2600,
        name: "Atari 2600",
        dir: "A2600",
        extensions: &["a26", "bin"],
        archives: true,
        archive_is_rom: false,
        picture: (320, 240),
        canvas: (320, 240),
        core: "stella2014",
        options: &[],
        bios: &[],
        pad: PAD_2600,
        shell: Some("a2600"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: true,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 25,
    },
    PlatformDef {
        id: Platform::Intv,
        name: "Intellivision",
        dir: "INTV",
        extensions: &["int", "bin"],
        archives: true,
        archive_is_rom: false,
        picture: (352, 224),
        canvas: (352, 224),
        core: "freeintv",
        options: &[],
        bios: INTV_BIOS,
        pad: PAD_INTV,
        shell: Some("intv"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: true,
        rewind: true,
        fill: false,
        triggers: true,
        dualshock: false,
        gb_palette: false,
        ra: 45,
    },
    PlatformDef {
        id: Platform::Lynx,
        name: "Atari Lynx",
        dir: "LYNX",
        extensions: &["lnx"],
        archives: true,
        archive_is_rom: false,
        picture: (160, 102),
        canvas: (160, 102),
        core: "handy",
        options: &[],
        bios: LYNX_BIOS,
        pad: PAD_LYNX,
        shell: Some("lynx"),
        label_with: &[],
        shader: ShaderFamily::Gb,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 13,
    },
    PlatformDef {
        id: Platform::A7800,
        name: "Atari 7800",
        dir: "A7800",
        extensions: &["a78", "bin"],
        archives: true,
        archive_is_rom: false,
        picture: (320, 240),
        canvas: (320, 240),
        core: "prosystem",
        options: &[],
        bios: &[],
        pad: PAD_7800,
        shell: Some("a7800"),
        label_with: &[],
        shader: ShaderFamily::Nes,
        colour: ColourCorrect::None,
        shoulders_are_slot: false,
        native_xy: false,
        rewind: true,
        fill: false,
        triggers: false,
        dualshock: false,
        gb_palette: false,
        ra: 51,
    },
];

impl Platform {
    pub const ALL: [Platform; 15] = [
        Platform::Gba,
        Platform::Gb,
        Platform::Gbc,
        Platform::Nes,
        Platform::Snes,
        Platform::Ps1,
        Platform::Md,
        Platform::Sms,
        Platform::Gg,
        Platform::Arcade,
        Platform::Pce,
        Platform::A2600,
        Platform::Intv,
        Platform::Lynx,
        Platform::A7800,
    ];

    pub fn def(self) -> &'static PlatformDef {
        &DEFS[self as usize]
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Where the platform is in Platform::ALL, for arrays kept one per platform.
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn dir_name(self) -> &'static str {
        self.def().dir
    }

    pub fn extensions(self) -> &'static [&'static str] {
        self.def().extensions
    }

    pub fn picture(self) -> (u32, u32) {
        self.def().picture
    }

    /// The frame canvas a core draws into, its picture centred.
    pub fn canvas(self) -> (u32, u32) {
        self.def().canvas
    }

    pub fn accepts(self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|k| ext.eq_ignore_ascii_case(k))
            })
    }

    /// Whether the shelf takes ROMs zipped or 7z'd. A disc would take too long to unpack on the
    /// Brick, and a .chd is compressed already. An arcade zip is the game, not a wrapper.
    pub fn takes_archives(self) -> bool {
        self.def().archives
    }

    /// The physical pad, rearranged into the bits this console's core reads.
    pub fn apply_pad(self, mask: u16) -> u16 {
        let mut out = 0u16;
        for (bit, from) in self.def().pad.from.into_iter().enumerate() {
            if mask & from != 0 {
                out |= 1 << bit;
            }
        }
        out
    }

    /// This shelf, then any other whose labels are the same shape.
    pub fn label_platforms(self) -> impl Iterator<Item = Platform> {
        std::iter::once(self).chain(self.def().label_with.iter().copied())
    }
}

/// BIOS files that are not on the card. Required ones stop the game from starting.
pub fn missing_bios(root: &Path, platform: Platform) -> Vec<&'static BiosFile> {
    platform
        .def()
        .bios
        .iter()
        .filter(|file| !bios_present(root, file.name))
        .collect()
}

fn bios_present(root: &Path, name: &str) -> bool {
    std::fs::metadata(root.join("BIOS").join(name)).is_ok_and(|m| m.is_file() && m.len() > 0)
}

/// The archive is the ROM the core loads, because it lives on a shelf that says so.
pub fn archive_is_the_rom(path: &Path) -> bool {
    let Some(dir) = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
    else {
        return false;
    };
    Platform::ALL.iter().any(|p| {
        let def = p.def();
        def.archive_is_rom && def.dir.eq_ignore_ascii_case(dir) && p.accepts(path)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_in_platform_order() {
        for (i, p) in Platform::ALL.iter().enumerate() {
            assert_eq!(p.index(), i);
            assert_eq!(DEFS[i].id, *p);
            assert_eq!(p.def().dir, p.dir_name());
        }
    }

    #[test]
    fn the_original_pad_is_unchanged() {
        let mask = 0b11_1111_1111_1111;
        for p in [
            Platform::Gba,
            Platform::Gb,
            Platform::Gbc,
            Platform::Nes,
            Platform::Snes,
            Platform::Ps1,
        ] {
            assert_eq!(p.apply_pad(mask), mask, "{p:?}");
            assert_eq!(p.apply_pad(0b1_0000_0001), 0b1_0000_0001, "{p:?}");
        }
    }

    #[test]
    fn a_new_shell_names_its_asset_directory() {
        assert_eq!(Platform::Md.def().shell, Some("md"));
        assert!(Platform::Gba.def().shell.is_none());
        assert!(Platform::Intv
            .def()
            .bios
            .iter()
            .any(|b| b.required && b.name == "exec.bin"));
        assert!(Platform::Lynx
            .def()
            .bios
            .iter()
            .any(|b| b.name == "lynxboot.img"));
        assert!(!Platform::Arcade.takes_archives());
        assert!(Platform::Arcade.def().archive_is_rom);
    }
}
