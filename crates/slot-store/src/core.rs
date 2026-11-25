use std::collections::HashMap;
use std::path::Path;

use crate::Platform;

pub const SELECTED_CORE_FILE: &str = "Config/selected_core.ini";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Core {
    #[default]
    Mgba,
    Gpsp,
    Fceumm,
    Snes9x,
    PcsxRearmed,
    GenesisPlusGx,
    Fbneo,
    MednafenPceFast,
    Stella2014,
    FreeIntv,
    Handy,
    Prosystem,
}

impl Core {
    pub const ALL: [Core; 12] = [
        Core::Mgba,
        Core::Gpsp,
        Core::Fceumm,
        Core::Snes9x,
        Core::PcsxRearmed,
        Core::GenesisPlusGx,
        Core::Fbneo,
        Core::MednafenPceFast,
        Core::Stella2014,
        Core::FreeIntv,
        Core::Handy,
        Core::Prosystem,
    ];
    /// The GBA's choice of cores, the picker board's two sockets (in Core::index order).
    pub const PICKER: [Core; 2] = [Core::Mgba, Core::Gpsp];

    pub fn as_str(&self) -> &'static str {
        match self {
            Core::Mgba => "mgba",
            Core::Gpsp => "gpsp",
            Core::Fceumm => "fceumm",
            Core::Snes9x => "snes9x",
            Core::PcsxRearmed => "pcsx_rearmed",
            Core::GenesisPlusGx => "genesis_plus_gx",
            Core::Fbneo => "fbneo",
            Core::MednafenPceFast => "mednafen_pce_fast",
            Core::Stella2014 => "stella2014",
            Core::FreeIntv => "freeintv",
            Core::Handy => "handy",
            Core::Prosystem => "prosystem",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            Core::Mgba => "mGBA",
            Core::Gpsp => "gpSP",
            Core::Fceumm => "FCEUmm",
            Core::Snes9x => "Snes9x",
            Core::PcsxRearmed => "PCSX ReARMed",
            Core::GenesisPlusGx => "GenPlus",
            Core::Fbneo => "FBNeo",
            Core::MednafenPceFast => "PCE",
            Core::Stella2014 => "Stella",
            Core::FreeIntv => "Intv",
            Core::Handy => "Handy",
            Core::Prosystem => "7800",
        }
    }

    pub fn parse(s: &str) -> Option<Core> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mgba" => Some(Core::Mgba),
            "gpsp" => Some(Core::Gpsp),
            "fceumm" => Some(Core::Fceumm),
            "snes9x" => Some(Core::Snes9x),
            "pcsx_rearmed" => Some(Core::PcsxRearmed),
            "genesis_plus_gx" => Some(Core::GenesisPlusGx),
            "fbneo" => Some(Core::Fbneo),
            "mednafen_pce_fast" => Some(Core::MednafenPceFast),
            "stella2014" => Some(Core::Stella2014),
            "freeintv" => Some(Core::FreeIntv),
            "handy" => Some(Core::Handy),
            "prosystem" => Some(Core::Prosystem),
            _ => None,
        }
    }

    pub fn runs(self, platform: Platform) -> bool {
        platform.def().core == self.as_str() || (self == Core::Gpsp && platform == Platform::Gba)
    }

    pub fn default_for(platform: Platform) -> Core {
        Core::parse(platform.def().core).unwrap_or(Core::Mgba)
    }
}

pub fn read_selected_cores(root: &Path) -> HashMap<String, Core> {
    crate::ini::read(root, SELECTED_CORE_FILE)
        .into_iter()
        .filter_map(|(stem, name)| Core::parse(&name).map(|core| (stem, core)))
        .collect()
}

pub fn core_for(root: &Path, stem: &str) -> Core {
    crate::ini::value(root, SELECTED_CORE_FILE, stem)
        .as_deref()
        .and_then(Core::parse)
        .unwrap_or_default()
}

pub fn core_for_platform(root: &Path, stem: &str, platform: Platform) -> Core {
    crate::ini::value(root, SELECTED_CORE_FILE, stem)
        .as_deref()
        .and_then(Core::parse)
        .filter(|core| core.runs(platform))
        .unwrap_or_else(|| Core::default_for(platform))
}

pub fn write_selected_core(root: &Path, stem: &str, core: Core) -> std::io::Result<()> {
    crate::ini::write(root, SELECTED_CORE_FILE, stem, core.as_str())
}
