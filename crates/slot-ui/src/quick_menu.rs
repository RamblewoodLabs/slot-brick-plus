use crate::draw::{Draw, TexId, OUT_H, OUT_W};
use crate::plate::{
    arrows_hint_face, centred_hints, hint_face, UndoFace, HINT_H, LEGEND_GAP, LEGEND_Y,
};
use crate::power_menu::{MENU_H, MENU_INK, MENU_PAD, MENU_PX};
use crate::slot_chrome::{edge, opening};
use crate::text;

/// The Brick Pro: set once at startup, it adds the Brick Pro's own rows (the stick lights).
static BRICK_PRO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn set_brick_pro(pro: bool) {
    BRICK_PRO.store(pro, std::sync::atomic::Ordering::Relaxed);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickRow {
    FastForward,
    FastForwardSound,
    Screen,
    Rumble,
    DateTime,
    About,
    GbaShader,
    GbShader,
    GbaColourCorrection,
    Game,
    EjectSave,
    Turbo,
    Rewind,
    SwapAb,
    GbPalettes,
    Lights,
    LedMode,
    LedTop,
    LedButtons,
    LedTriggers,
    LedColor,
    LedEffect,
    Wifi,
    CartArt,
    Achievements,
    ScreenGb,
    ScreenGba,
    ScreenNes,
    ScreenSnes,
    GbColourCorrection,
    NesShader,
    SnesShader,
    GbOverlay,
    GbcOverlay,
    GbaOverlay,
    NesOverlay,
    SnesOverlay,
    AutoSave,
    BatteryShutdown,
    Update,
    System,
    AutoClock,
    LedSticks,
    Library,
    LibrarySort,
    LibraryShow,
    LibraryPlayTime,
    LibraryTitleCard,
    ScreenPs1,
    Ps1Shader,
    Ps1Dithering,
    Ps1Overlay,
    Font,
    Online,
    Clock,
    Saving,
    LibrarySystems,
    ShowGba,
    ShowGb,
    ShowGbc,
    ShowNes,
    ShowSnes,
    ShowPs1,
    ResetSettings,
    AboutVersion,
    AboutBuild,
    AboutDevice,
    AboutFirmware,
    AboutCard,
    AboutLabel,
    SoundMenu,
    SystemSounds,
    SoundStar,
    SoundCart,
    SoundCartOpen,
    SoundLabel,
    SoundVolume,
    UpdateCheck,
    Companion,
    Aspect,
    AspectGb,
    AspectGba,
    AspectNes,
    AspectSnes,
    AspectPs1,
    ShowMd,
    ShowSms,
    ShowGg,
    ShowArcade,
    ShowPce,
    ShowA2600,
    ShowIntv,
    ShowLynx,
    ShowA7800,
    ShowGw,
    ShowColeco,
    ShowA5200,
    ShowA800,
    ShowVectrex,
    Show32x,
    ShowN64,
    ShowNds,
    ShowPsp,
    ShowFavorites,
    RaEncore,
}

impl QuickRow {
    /// HOME → Companion, and Pip's toasts. Off until this is set back to true.
    pub const PIP: bool = false;

    pub const ALL: [QuickRow; 105] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Screen,
        QuickRow::Rumble,
        QuickRow::DateTime,
        QuickRow::About,
        QuickRow::GbaShader,
        QuickRow::GbShader,
        QuickRow::GbaColourCorrection,
        QuickRow::Game,
        QuickRow::EjectSave,
        QuickRow::Turbo,
        QuickRow::Rewind,
        QuickRow::SwapAb,
        QuickRow::GbPalettes,
        QuickRow::Lights,
        QuickRow::LedMode,
        QuickRow::LedTop,
        QuickRow::LedButtons,
        QuickRow::LedTriggers,
        QuickRow::LedColor,
        QuickRow::LedEffect,
        QuickRow::Wifi,
        QuickRow::CartArt,
        QuickRow::Achievements,
        QuickRow::ScreenGb,
        QuickRow::ScreenGba,
        QuickRow::ScreenNes,
        QuickRow::ScreenSnes,
        QuickRow::GbColourCorrection,
        QuickRow::NesShader,
        QuickRow::SnesShader,
        QuickRow::GbOverlay,
        QuickRow::GbcOverlay,
        QuickRow::GbaOverlay,
        QuickRow::NesOverlay,
        QuickRow::SnesOverlay,
        QuickRow::AutoSave,
        QuickRow::BatteryShutdown,
        QuickRow::Update,
        QuickRow::System,
        QuickRow::AutoClock,
        QuickRow::LedSticks,
        QuickRow::Library,
        QuickRow::LibrarySort,
        QuickRow::LibraryShow,
        QuickRow::LibraryPlayTime,
        QuickRow::LibraryTitleCard,
        QuickRow::ScreenPs1,
        QuickRow::Ps1Shader,
        QuickRow::Ps1Dithering,
        QuickRow::Ps1Overlay,
        QuickRow::Font,
        QuickRow::Online,
        QuickRow::Clock,
        QuickRow::Saving,
        QuickRow::LibrarySystems,
        QuickRow::ShowGba,
        QuickRow::ShowGb,
        QuickRow::ShowGbc,
        QuickRow::ShowNes,
        QuickRow::ShowSnes,
        QuickRow::ShowPs1,
        QuickRow::ResetSettings,
        QuickRow::AboutVersion,
        QuickRow::AboutBuild,
        QuickRow::AboutDevice,
        QuickRow::AboutFirmware,
        QuickRow::AboutCard,
        QuickRow::AboutLabel,
        QuickRow::SoundMenu,
        QuickRow::SystemSounds,
        QuickRow::SoundStar,
        QuickRow::SoundCart,
        QuickRow::SoundCartOpen,
        QuickRow::SoundLabel,
        QuickRow::SoundVolume,
        QuickRow::UpdateCheck,
        QuickRow::Companion,
        QuickRow::Aspect,
        QuickRow::AspectGb,
        QuickRow::AspectGba,
        QuickRow::AspectNes,
        QuickRow::AspectSnes,
        QuickRow::AspectPs1,
        QuickRow::ShowMd,
        QuickRow::ShowSms,
        QuickRow::ShowGg,
        QuickRow::ShowArcade,
        QuickRow::ShowPce,
        QuickRow::ShowA2600,
        QuickRow::ShowIntv,
        QuickRow::ShowLynx,
        QuickRow::ShowA7800,
        QuickRow::ShowGw,
        QuickRow::ShowColeco,
        QuickRow::ShowA5200,
        QuickRow::ShowA800,
        QuickRow::ShowVectrex,
        QuickRow::Show32x,
        QuickRow::ShowN64,
        QuickRow::ShowNds,
        QuickRow::ShowPsp,
        QuickRow::ShowFavorites,
        QuickRow::RaEncore,
    ];

    #[cfg(not(feature = "brick"))]
    pub const MAIN: [QuickRow; 4] = [
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::Library,
        QuickRow::System,
    ];
    #[cfg(not(feature = "brick"))]
    const MAIN_PIP: [QuickRow; 5] = [
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::Library,
        QuickRow::Companion,
        QuickRow::System,
    ];
    #[cfg(not(feature = "brick"))]
    pub const SYSTEM: [QuickRow; 7] = [
        QuickRow::DateTime,
        QuickRow::Font,
        QuickRow::Rumble,
        QuickRow::SystemSounds,
        QuickRow::BatteryShutdown,
        QuickRow::ResetSettings,
        QuickRow::About,
    ];

    #[cfg(feature = "brick")]
    pub const MAIN: [QuickRow; 5] = [
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::Library,
        QuickRow::Online,
        QuickRow::System,
    ];
    #[cfg(feature = "brick")]
    const MAIN_PIP: [QuickRow; 6] = [
        QuickRow::Screen,
        QuickRow::Game,
        QuickRow::Library,
        QuickRow::Companion,
        QuickRow::Online,
        QuickRow::System,
    ];

    /// The HOME menu. Companion is on it only while [`PIP`](Self::PIP) is on.
    pub fn home() -> &'static [QuickRow] {
        if Self::PIP {
            &Self::MAIN_PIP
        } else {
            &Self::MAIN
        }
    }

    pub const fn home_len() -> usize {
        if Self::PIP {
            Self::MAIN_PIP.len()
        } else {
            Self::MAIN.len()
        }
    }
    /// System: the Brick's lights, its clock, the font, rumble, Slot's sounds, the battery,
    /// asking GitHub for a newer Slot, every setting back to how it came, and what Slot is.
    #[cfg(all(feature = "brick", not(feature = "rg35xxsp")))]
    pub const SYSTEM: [QuickRow; 9] = [
        QuickRow::Lights,
        QuickRow::Clock,
        QuickRow::Font,
        QuickRow::Rumble,
        QuickRow::SystemSounds,
        QuickRow::BatteryShutdown,
        QuickRow::UpdateCheck,
        QuickRow::ResetSettings,
        QuickRow::About,
    ];
    /// The SP has no Brick light bar, and Online → Update would install the Brick binary.
    #[cfg(feature = "rg35xxsp")]
    pub const SYSTEM: [QuickRow; 7] = [
        QuickRow::Clock,
        QuickRow::Font,
        QuickRow::Rumble,
        QuickRow::SystemSounds,
        QuickRow::BatteryShutdown,
        QuickRow::ResetSettings,
        QuickRow::About,
    ];
    /// System → System Sounds: how loud they all are, and each kind on or off.
    pub const SOUNDS: [QuickRow; 6] = [
        QuickRow::SoundVolume,
        QuickRow::SoundMenu,
        QuickRow::SoundStar,
        QuickRow::SoundCart,
        QuickRow::SoundCartOpen,
        QuickRow::SoundLabel,
    ];
    /// Clock: setting it by hand, and from the internet.
    pub const CLOCK: [QuickRow; 2] = [QuickRow::DateTime, QuickRow::AutoClock];
    /// Online: what needs Wi-Fi, and Wi-Fi itself.
    /// The SP build leaves Update off: that row installs the Brick's `slot` binary.
    #[cfg(not(feature = "rg35xxsp"))]
    pub const ONLINE: [QuickRow; 5] = [
        QuickRow::Wifi,
        QuickRow::CartArt,
        QuickRow::Achievements,
        QuickRow::RaEncore,
        QuickRow::Update,
    ];
    #[cfg(feature = "rg35xxsp")]
    pub const ONLINE: [QuickRow; 4] = [
        QuickRow::Wifi,
        QuickRow::CartArt,
        QuickRow::Achievements,
        QuickRow::RaEncore,
    ];
    pub const LIGHTS: [QuickRow; 6] = [
        QuickRow::LedMode,
        QuickRow::LedTop,
        QuickRow::LedButtons,
        QuickRow::LedTriggers,
        QuickRow::LedColor,
        QuickRow::LedEffect,
    ];
    /// The Brick Pro's: its sticks have lights of their own.
    pub const LIGHTS_PRO: [QuickRow; 7] = [
        QuickRow::LedMode,
        QuickRow::LedTop,
        QuickRow::LedButtons,
        QuickRow::LedTriggers,
        QuickRow::LedSticks,
        QuickRow::LedColor,
        QuickRow::LedEffect,
    ];
    /// The LEDs page of the Brick this is (see `set_brick_pro`).
    pub fn lights() -> &'static [QuickRow] {
        match BRICK_PRO.load(std::sync::atomic::Ordering::Relaxed) {
            true => &QuickRow::LIGHTS_PRO,
            false => &QuickRow::LIGHTS,
        }
    }
    pub fn led_setting(self) -> bool {
        Self::LIGHTS_PRO.contains(&self)
    }

    /// Library: how the shelves are ordered, whether they hold only the favorites, which
    /// systems have a shelf, and what is printed around them.
    pub const LIBRARY: [QuickRow; 5] = [
        QuickRow::LibrarySort,
        QuickRow::LibraryShow,
        QuickRow::LibrarySystems,
        QuickRow::LibraryTitleCard,
        QuickRow::LibraryPlayTime,
    ];
    /// Library → Systems: each system's shelf shown or hidden, in the shelves' order.
    pub const SYSTEMS: [QuickRow; 25] = [
        QuickRow::ShowFavorites,
        QuickRow::ShowGba,
        QuickRow::ShowGb,
        QuickRow::ShowGbc,
        QuickRow::ShowNes,
        QuickRow::ShowSnes,
        QuickRow::ShowPs1,
        QuickRow::ShowMd,
        QuickRow::ShowSms,
        QuickRow::ShowGg,
        QuickRow::ShowArcade,
        QuickRow::ShowPce,
        QuickRow::ShowA2600,
        QuickRow::ShowIntv,
        QuickRow::ShowLynx,
        QuickRow::ShowA7800,
        QuickRow::ShowGw,
        QuickRow::ShowColeco,
        QuickRow::ShowA5200,
        QuickRow::ShowA800,
        QuickRow::ShowVectrex,
        QuickRow::Show32x,
        QuickRow::ShowN64,
        QuickRow::ShowNds,
        QuickRow::ShowPsp,
    ];
    /// Gameplay → Saving: when a game is saved without being asked.
    pub const SAVING: [QuickRow; 2] = [QuickRow::AutoSave, QuickRow::EjectSave];

    /// Screen: a page for each system, holding that system's own picture settings.
    pub const SCREEN: [QuickRow; 6] = [
        QuickRow::ScreenGb,
        QuickRow::ScreenGba,
        QuickRow::ScreenNes,
        QuickRow::ScreenSnes,
        QuickRow::ScreenPs1,
        QuickRow::Aspect,
    ];
    pub const SCREEN_GB: [QuickRow; 6] = [
        QuickRow::GbShader,
        QuickRow::GbColourCorrection,
        QuickRow::GbPalettes,
        QuickRow::GbOverlay,
        QuickRow::GbcOverlay,
        QuickRow::AspectGb,
    ];
    pub const SCREEN_GBA: [QuickRow; 4] = [
        QuickRow::GbaShader,
        QuickRow::GbaColourCorrection,
        QuickRow::GbaOverlay,
        QuickRow::AspectGba,
    ];
    pub const SCREEN_NES: [QuickRow; 3] = [
        QuickRow::NesShader,
        QuickRow::NesOverlay,
        QuickRow::AspectNes,
    ];
    pub const SCREEN_SNES: [QuickRow; 3] = [
        QuickRow::SnesShader,
        QuickRow::SnesOverlay,
        QuickRow::AspectSnes,
    ];
    pub const SCREEN_PS1: [QuickRow; 4] = [
        QuickRow::Ps1Shader,
        QuickRow::Ps1Dithering,
        QuickRow::Ps1Overlay,
        QuickRow::AspectPs1,
    ];

    /// System → About: what this Slot is and what it runs on, then its label.
    pub const ABOUT: [QuickRow; 6] = [
        QuickRow::AboutVersion,
        QuickRow::AboutBuild,
        QuickRow::AboutDevice,
        QuickRow::AboutFirmware,
        QuickRow::AboutCard,
        QuickRow::AboutLabel,
    ];
    /// The About rows that only say something: nothing to change or open.
    pub const ABOUT_INFO: [QuickRow; 5] = [
        QuickRow::AboutVersion,
        QuickRow::AboutBuild,
        QuickRow::AboutDevice,
        QuickRow::AboutFirmware,
        QuickRow::AboutCard,
    ];
    pub fn info(self) -> bool {
        Self::ABOUT_INFO.contains(&self)
    }

    /// The system an Overlay row picks the overlay for.
    /// Rows that read a name rather than a fixed value (an overlay's, the font's, About's, the
    /// sounds' volume), numbered: the overlay rows by their platform, then Font, then About's,
    /// then Volume.
    pub const NAMED: usize = slot_store::Platform::ALL.len() + 1 + Self::ABOUT_INFO.len() + 1;
    pub fn named(self) -> Option<usize> {
        let about = slot_store::Platform::ALL.len() + 1;
        match self {
            QuickRow::Font => Some(slot_store::Platform::ALL.len()),
            QuickRow::SoundVolume => Some(Self::NAMED - 1),
            row if row.info() => Self::ABOUT_INFO
                .iter()
                .position(|r| *r == row)
                .map(|at| about + at),
            _ => self.overlay_of().map(|p| p.index()),
        }
    }

    /// The system a Library → Systems row shows or hides.
    pub fn shelf_of(self) -> Option<slot_store::Platform> {
        use slot_store::Platform;
        Some(match self {
            QuickRow::ShowGba => Platform::Gba,
            QuickRow::ShowGb => Platform::Gb,
            QuickRow::ShowGbc => Platform::Gbc,
            QuickRow::ShowNes => Platform::Nes,
            QuickRow::ShowSnes => Platform::Snes,
            QuickRow::ShowPs1 => Platform::Ps1,
            QuickRow::ShowMd => Platform::Md,
            QuickRow::ShowSms => Platform::Sms,
            QuickRow::ShowGg => Platform::Gg,
            QuickRow::ShowArcade => Platform::Arcade,
            QuickRow::ShowPce => Platform::Pce,
            QuickRow::ShowA2600 => Platform::A2600,
            QuickRow::ShowIntv => Platform::Intv,
            QuickRow::ShowLynx => Platform::Lynx,
            QuickRow::ShowA7800 => Platform::A7800,
            QuickRow::ShowGw => Platform::Gw,
            QuickRow::ShowColeco => Platform::Coleco,
            QuickRow::ShowA5200 => Platform::A5200,
            QuickRow::ShowA800 => Platform::A800,
            QuickRow::ShowVectrex => Platform::Vectrex,
            QuickRow::Show32x => Platform::S32x,
            QuickRow::ShowN64 => Platform::N64,
            QuickRow::ShowNds => Platform::Nds,
            QuickRow::ShowPsp => Platform::Psp,
            _ => return None,
        })
    }

    pub fn overlay_of(self) -> Option<slot_store::Platform> {
        use slot_store::Platform;
        Some(match self {
            QuickRow::GbOverlay => Platform::Gb,
            QuickRow::GbcOverlay => Platform::Gbc,
            QuickRow::GbaOverlay => Platform::Gba,
            QuickRow::NesOverlay => Platform::Nes,
            QuickRow::SnesOverlay => Platform::Snes,
            QuickRow::Ps1Overlay => Platform::Ps1,
            _ => return None,
        })
    }

    pub const GAME: [QuickRow; 6] = [
        QuickRow::FastForward,
        QuickRow::FastForwardSound,
        QuickRow::Rewind,
        QuickRow::Turbo,
        QuickRow::SwapAb,
        QuickRow::Saving,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page(self) -> &'static [QuickRow] {
        match self {
            QuickRow::LedMode
            | QuickRow::LedTop
            | QuickRow::LedButtons
            | QuickRow::LedTriggers
            | QuickRow::LedSticks
            | QuickRow::LedColor
            | QuickRow::LedEffect => QuickRow::lights(),
            QuickRow::ScreenGb
            | QuickRow::ScreenGba
            | QuickRow::ScreenNes
            | QuickRow::ScreenSnes
            | QuickRow::ScreenPs1
            | QuickRow::Aspect => &QuickRow::SCREEN,
            QuickRow::GbShader
            | QuickRow::GbColourCorrection
            | QuickRow::GbPalettes
            | QuickRow::GbOverlay
            | QuickRow::GbcOverlay
            | QuickRow::AspectGb => &QuickRow::SCREEN_GB,
            QuickRow::GbaShader
            | QuickRow::GbaColourCorrection
            | QuickRow::GbaOverlay
            | QuickRow::AspectGba => &QuickRow::SCREEN_GBA,
            QuickRow::NesShader | QuickRow::NesOverlay | QuickRow::AspectNes => {
                &QuickRow::SCREEN_NES
            }
            QuickRow::SnesShader | QuickRow::SnesOverlay | QuickRow::AspectSnes => {
                &QuickRow::SCREEN_SNES
            }
            QuickRow::Ps1Shader
            | QuickRow::Ps1Dithering
            | QuickRow::Ps1Overlay
            | QuickRow::AspectPs1 => &QuickRow::SCREEN_PS1,
            QuickRow::FastForward
            | QuickRow::FastForwardSound
            | QuickRow::Rewind
            | QuickRow::Turbo
            | QuickRow::SwapAb
            | QuickRow::Saving => &QuickRow::GAME,
            QuickRow::AutoSave | QuickRow::EjectSave => &QuickRow::SAVING,
            #[cfg(feature = "brick")]
            QuickRow::DateTime | QuickRow::AutoClock => &QuickRow::CLOCK,
            #[cfg(not(feature = "brick"))]
            QuickRow::DateTime | QuickRow::AutoClock => &Self::SYSTEM,
            QuickRow::Lights
            | QuickRow::Clock
            | QuickRow::Font
            | QuickRow::Rumble
            | QuickRow::SystemSounds
            | QuickRow::BatteryShutdown
            | QuickRow::ResetSettings
            | QuickRow::About => &Self::SYSTEM,
            #[cfg(feature = "brick")]
            QuickRow::UpdateCheck => &Self::SYSTEM,
            QuickRow::ShowFavorites
            | QuickRow::ShowGba
            | QuickRow::ShowGb
            | QuickRow::ShowGbc
            | QuickRow::ShowNes
            | QuickRow::ShowSnes
            | QuickRow::ShowPs1
            | QuickRow::ShowMd
            | QuickRow::ShowSms
            | QuickRow::ShowGg
            | QuickRow::ShowArcade
            | QuickRow::ShowPce
            | QuickRow::ShowA2600
            | QuickRow::ShowIntv
            | QuickRow::ShowLynx
            | QuickRow::ShowA7800
            | QuickRow::ShowGw
            | QuickRow::ShowColeco
            | QuickRow::ShowA5200
            | QuickRow::ShowA800
            | QuickRow::ShowVectrex
            | QuickRow::Show32x
            | QuickRow::ShowN64
            | QuickRow::ShowNds
            | QuickRow::ShowPsp => &QuickRow::SYSTEMS,
            QuickRow::SoundVolume
            | QuickRow::SoundMenu
            | QuickRow::SoundStar
            | QuickRow::SoundCart
            | QuickRow::SoundCartOpen
            | QuickRow::SoundLabel => &QuickRow::SOUNDS,
            QuickRow::AboutVersion
            | QuickRow::AboutBuild
            | QuickRow::AboutDevice
            | QuickRow::AboutFirmware
            | QuickRow::AboutCard
            | QuickRow::AboutLabel => &QuickRow::ABOUT,
            QuickRow::LibrarySort
            | QuickRow::LibraryShow
            | QuickRow::LibrarySystems
            | QuickRow::LibraryPlayTime
            | QuickRow::LibraryTitleCard => &QuickRow::LIBRARY,
            QuickRow::Wifi
            | QuickRow::CartArt
            | QuickRow::Achievements
            | QuickRow::RaEncore
            | QuickRow::Update => &QuickRow::ONLINE,
            _ => QuickRow::home(),
        }
    }

    pub fn parent(self) -> Option<QuickRow> {
        match self.page()[0] {
            QuickRow::LedMode => Some(QuickRow::Lights),
            QuickRow::ScreenGb => Some(QuickRow::Screen),
            QuickRow::GbShader => Some(QuickRow::ScreenGb),
            QuickRow::GbaShader => Some(QuickRow::ScreenGba),
            QuickRow::NesShader => Some(QuickRow::ScreenNes),
            QuickRow::SnesShader => Some(QuickRow::ScreenSnes),
            QuickRow::Ps1Shader => Some(QuickRow::ScreenPs1),
            QuickRow::FastForward => Some(QuickRow::Game),
            QuickRow::LibrarySort => Some(QuickRow::Library),
            QuickRow::ShowFavorites => Some(QuickRow::LibrarySystems),
            QuickRow::AboutVersion => Some(QuickRow::About),
            QuickRow::SoundVolume => Some(QuickRow::SystemSounds),
            QuickRow::Wifi => Some(QuickRow::Online),
            QuickRow::AutoSave => Some(QuickRow::Saving),
            #[cfg(feature = "brick")]
            QuickRow::DateTime => Some(QuickRow::Clock),
            first if first == QuickRow::SYSTEM[0] => Some(QuickRow::System),
            _ => None,
        }
    }

    pub fn child(self) -> Option<QuickRow> {
        match self {
            QuickRow::Lights => Some(QuickRow::lights()[0]),
            QuickRow::Screen => Some(QuickRow::SCREEN[0]),
            QuickRow::ScreenGb => Some(QuickRow::SCREEN_GB[0]),
            QuickRow::ScreenGba => Some(QuickRow::SCREEN_GBA[0]),
            QuickRow::ScreenNes => Some(QuickRow::SCREEN_NES[0]),
            QuickRow::ScreenSnes => Some(QuickRow::SCREEN_SNES[0]),
            QuickRow::ScreenPs1 => Some(QuickRow::SCREEN_PS1[0]),
            QuickRow::Game => Some(QuickRow::GAME[0]),
            QuickRow::System => Some(QuickRow::SYSTEM[0]),
            QuickRow::Library => Some(QuickRow::LIBRARY[0]),
            QuickRow::LibrarySystems => Some(QuickRow::SYSTEMS[0]),
            QuickRow::About => Some(QuickRow::ABOUT[0]),
            QuickRow::SystemSounds => Some(QuickRow::SOUNDS[0]),
            QuickRow::Online => Some(QuickRow::ONLINE[0]),
            QuickRow::Clock => Some(QuickRow::CLOCK[0]),
            QuickRow::Saving => Some(QuickRow::SAVING[0]),
            _ => None,
        }
    }

    pub fn position(self) -> usize {
        self.page().iter().position(|r| *r == self).unwrap_or(0)
    }

    pub fn label(self) -> &'static str {
        match self {
            QuickRow::Wifi => "Wi-Fi",
            QuickRow::CartArt => "Cart Art",
            QuickRow::Achievements => "RetroAchievements",
            QuickRow::RaEncore => "Encore Mode",
            QuickRow::Update => "Update",
            QuickRow::System => "System",
            QuickRow::Online => "Online",
            QuickRow::Clock => "Clock",
            QuickRow::Saving => "Saving",
            QuickRow::AutoClock => "Auto Clock",
            QuickRow::UpdateCheck => "Check for Updates",
            QuickRow::Companion => "Companion",
            QuickRow::Aspect
            | QuickRow::AspectGb
            | QuickRow::AspectGba
            | QuickRow::AspectNes
            | QuickRow::AspectSnes
            | QuickRow::AspectPs1 => "Aspect",
            QuickRow::Font => "Font",
            QuickRow::Lights => "LEDs",
            QuickRow::LedMode => "All LEDs",
            QuickRow::LedTop => "Top Bar",
            QuickRow::LedButtons => "Buttons",
            QuickRow::LedTriggers => "Triggers",
            QuickRow::LedColor => "Color",
            QuickRow::LedEffect => "Effect",
            QuickRow::LedSticks => "Sticks",
            QuickRow::Library => "Library",
            QuickRow::LibrarySort => "Sort By",
            QuickRow::LibraryShow => "Show",
            QuickRow::LibraryPlayTime => "Play Time",
            QuickRow::LibraryTitleCard => "Title Card",
            QuickRow::LibrarySystems => "Systems",
            QuickRow::ShowFavorites => "Favorites",
            QuickRow::ShowGba => "Game Boy Advance",
            QuickRow::ShowGb => "Game Boy",
            QuickRow::ShowGbc => "Game Boy Color",
            QuickRow::ShowNes => "NES",
            QuickRow::ShowSnes => "SNES",
            QuickRow::ShowPs1 => "PlayStation",
            QuickRow::ShowMd => "Genesis / Mega Drive",
            QuickRow::ShowSms => "Master System",
            QuickRow::ShowGg => "Game Gear",
            QuickRow::ShowArcade => "Arcade",
            QuickRow::ShowPce => "TurboGrafx-16",
            QuickRow::ShowA2600 => "Atari 2600",
            QuickRow::ShowIntv => "Intellivision",
            QuickRow::ShowLynx => "Atari Lynx",
            QuickRow::ShowA7800 => "Atari 7800",
            QuickRow::ShowGw => "Game & Watch",
            QuickRow::ShowColeco => "ColecoVision",
            QuickRow::ShowA5200 => "Atari 5200",
            QuickRow::ShowA800 => "Atari 800",
            QuickRow::ShowVectrex => "Vectrex",
            QuickRow::Show32x => "Sega 32X",
            QuickRow::ShowN64 => "Nintendo 64",
            QuickRow::ShowNds => "Nintendo DS",
            QuickRow::ShowPsp => "PSP",
            QuickRow::ResetSettings => "Reset Settings",
            QuickRow::AboutVersion => "Version",
            QuickRow::AboutBuild => "Build",
            QuickRow::AboutDevice => "Device",
            QuickRow::AboutFirmware => "Firmware",
            QuickRow::AboutCard => "SD Card",
            QuickRow::AboutLabel => "Label",
            QuickRow::SoundMenu => "Menu",
            QuickRow::SystemSounds => "System Sounds",
            QuickRow::SoundStar => "Favorite Star",
            QuickRow::SoundCart => "Cart In & Out",
            QuickRow::SoundCartOpen => "Cart Opening",
            QuickRow::SoundLabel => "Label On",
            QuickRow::SoundVolume => "Volume",
            QuickRow::FastForward => "Fast Forward",
            QuickRow::FastForwardSound => "Fast Forward Sound",
            QuickRow::Screen => "Emulation",
            QuickRow::Rumble => "Rumble",
            QuickRow::DateTime => "Date & Time",
            QuickRow::About => "About",
            QuickRow::GbaShader
            | QuickRow::GbShader
            | QuickRow::NesShader
            | QuickRow::SnesShader
            | QuickRow::Ps1Shader => "Shader",
            QuickRow::GbColourCorrection | QuickRow::GbaColourCorrection => "Color Correction",
            QuickRow::GbOverlay => "GB Overlay",
            QuickRow::GbcOverlay => "GBC Overlay",
            QuickRow::GbaOverlay
            | QuickRow::NesOverlay
            | QuickRow::SnesOverlay
            | QuickRow::Ps1Overlay => "Overlay",
            QuickRow::Ps1Dithering => "Dithering",
            QuickRow::ScreenGb => "Game Boy / GBC",
            QuickRow::ScreenGba => "Game Boy Advance",
            QuickRow::ScreenNes => "NES",
            QuickRow::ScreenSnes => "SNES",
            QuickRow::ScreenPs1 => "PlayStation",
            QuickRow::Game => "Gameplay",
            QuickRow::EjectSave => "Save on Eject",
            QuickRow::AutoSave => "Auto Save",
            QuickRow::BatteryShutdown => "Low Battery Shutdown",
            QuickRow::Turbo => "Turbo Buttons",
            QuickRow::Rewind => "Rewind",
            QuickRow::SwapAb => "Swap A/B",
            QuickRow::GbPalettes => "GB Palettes",
        }
    }

    pub fn opens(self) -> bool {
        matches!(
            self,
            QuickRow::DateTime
                | QuickRow::About
                | QuickRow::Screen
                | QuickRow::Game
                | QuickRow::Lights
                | QuickRow::Wifi
                | QuickRow::CartArt
                | QuickRow::Achievements
                | QuickRow::Update
                | QuickRow::System
                | QuickRow::Library
                | QuickRow::Companion
                | QuickRow::LibrarySystems
                | QuickRow::ResetSettings
                | QuickRow::AboutLabel
                | QuickRow::SystemSounds
                | QuickRow::Online
                | QuickRow::Clock
                | QuickRow::Saving
                | QuickRow::ScreenGb
                | QuickRow::ScreenGba
                | QuickRow::ScreenNes
                | QuickRow::ScreenSnes
                | QuickRow::ScreenPs1
        )
    }

    pub fn up(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + page.len() - 1) % page.len()]
    }

    pub fn down(self) -> QuickRow {
        let page = self.page();
        page[(self.position() + 1) % page.len()]
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum QuickValue {
    Speed2,
    Speed3,
    Speed4,
    Speed6,
    On,
    Off,
    Lcd3x,
    Grid,
    Dot,
    Simpletex,
    Stock,
    Low,
    Medium,
    High,
    White,
    Blue,
    Green,
    Amber,
    Red,
    Purple,
    Steady,
    Breathe,
    Unavailable,
    Min1,
    Min5,
    Min10,
    Percent3,
    Percent5,
    Percent10,
    SortAz,
    SortRecent,
    SortMost,
    AllGames,
    Favorites,
    Minutes,
    Hours,
    /// Reset Settings, asked once: A again to go ahead.
    ResetAsk,
    /// Reset Settings, done.
    ResetDone,
    AspectNative,
    Aspect43,
    AspectFull,
    AspectInteger,
    AspectDefault,
}

impl QuickValue {
    pub const ALL: [QuickValue; 43] = [
        QuickValue::Speed2,
        QuickValue::Speed3,
        QuickValue::Speed4,
        QuickValue::Speed6,
        QuickValue::On,
        QuickValue::Off,
        QuickValue::Lcd3x,
        QuickValue::Grid,
        QuickValue::Dot,
        QuickValue::Simpletex,
        QuickValue::Stock,
        QuickValue::Low,
        QuickValue::Medium,
        QuickValue::High,
        QuickValue::White,
        QuickValue::Blue,
        QuickValue::Green,
        QuickValue::Amber,
        QuickValue::Red,
        QuickValue::Purple,
        QuickValue::Steady,
        QuickValue::Breathe,
        QuickValue::Unavailable,
        QuickValue::Min1,
        QuickValue::Min5,
        QuickValue::Min10,
        QuickValue::Percent3,
        QuickValue::Percent5,
        QuickValue::Percent10,
        QuickValue::SortAz,
        QuickValue::SortRecent,
        QuickValue::SortMost,
        QuickValue::AllGames,
        QuickValue::Favorites,
        QuickValue::Minutes,
        QuickValue::Hours,
        QuickValue::ResetAsk,
        QuickValue::ResetDone,
        QuickValue::AspectNative,
        QuickValue::Aspect43,
        QuickValue::AspectFull,
        QuickValue::AspectInteger,
        QuickValue::AspectDefault,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn text(self) -> &'static str {
        match self {
            QuickValue::Speed2 => "2×",
            QuickValue::Speed3 => "3×",
            QuickValue::Speed4 => "4×",
            QuickValue::Speed6 => "6×",
            QuickValue::On => "On",
            QuickValue::Off => "Off",
            QuickValue::Lcd3x => "LCD3x",
            QuickValue::Grid => "Grid",
            QuickValue::Dot => "Dot",
            QuickValue::Simpletex => "Simpletex",
            QuickValue::Stock => "Stock",
            QuickValue::Low => "Low",
            QuickValue::Medium => "Medium",
            QuickValue::High => "High",
            QuickValue::White => "White",
            QuickValue::Blue => "Blue",
            QuickValue::Green => "Green",
            QuickValue::Amber => "Amber",
            QuickValue::Red => "Red",
            QuickValue::Purple => "Purple",
            QuickValue::Steady => "Steady",
            QuickValue::Breathe => "Breathe",
            QuickValue::Unavailable => "Unavailable",
            QuickValue::Min1 => "1 min",
            QuickValue::Min5 => "5 min",
            QuickValue::Min10 => "10 min",
            QuickValue::Percent3 => "3%",
            QuickValue::Percent5 => "5%",
            QuickValue::Percent10 => "10%",
            QuickValue::SortAz => "A–Z",
            QuickValue::SortRecent => "Last Played",
            QuickValue::SortMost => "Most Played",
            QuickValue::AllGames => "All Games",
            QuickValue::Favorites => "Favorites",
            QuickValue::Minutes => "Minutes",
            QuickValue::Hours => "Hours",
            QuickValue::ResetAsk => "Sure?",
            QuickValue::ResetDone => "Done",
            QuickValue::AspectNative => "Native",
            QuickValue::Aspect43 => "4:3",
            QuickValue::AspectFull => "Full",
            QuickValue::AspectInteger => "Integer",
            QuickValue::AspectDefault => "Default",
        }
    }

    pub fn speed(frames: u8) -> Option<QuickValue> {
        match frames {
            2 => Some(QuickValue::Speed2),
            3 => Some(QuickValue::Speed3),
            4 => Some(QuickValue::Speed4),
            6 => Some(QuickValue::Speed6),
            _ => None,
        }
    }

    /// An autosave interval in minutes, 0 for off.
    pub fn minutes(min: u8) -> Option<QuickValue> {
        match min {
            0 => Some(QuickValue::Off),
            1 => Some(QuickValue::Min1),
            5 => Some(QuickValue::Min5),
            10 => Some(QuickValue::Min10),
            _ => None,
        }
    }

    /// A battery percent, 0 for off.
    pub fn percent(pct: u8) -> Option<QuickValue> {
        match pct {
            0 => Some(QuickValue::Off),
            3 => Some(QuickValue::Percent3),
            5 => Some(QuickValue::Percent5),
            10 => Some(QuickValue::Percent10),
            _ => None,
        }
    }

    pub fn flag(on: bool) -> QuickValue {
        if on {
            QuickValue::On
        } else {
            QuickValue::Off
        }
    }
}

pub const PIP: bool = QuickRow::PIP;

pub const QUICK_PITCH: f32 = 52.0;
pub const QUICK_TOP: f32 =
    (OUT_H as f32 - quick_pitch(QuickRow::home_len()) * QuickRow::home_len() as f32) / 2.0;

/// Row spacing for a page of `rows`: QUICK_PITCH, or tighter for a page that would
/// otherwise run into the legend (the Brick's main menu).
pub const fn quick_pitch(rows: usize) -> f32 {
    let fit = ((QUICK_SPLIT - OUT_H as f32 / 2.0) * 2.0 / rows as f32) as u32 as f32;
    if fit < QUICK_PITCH {
        fit
    } else {
        QUICK_PITCH
    }
}
pub const QUICK_EDGE: f32 = 32.0;
const BAR_INSET: f32 = 4.0;
const TYPE_DROP: f32 = 4.0;
const CARET_GAP: f32 = 14.0;
const CARET_PX: f32 = 24.0;
pub const QUICK_SPLIT: f32 = LEGEND_Y - 2.0;
const DIM_INK: [u8; 3] = [0x9a, 0x9a, 0xa4];

pub fn quick_label_face(row: QuickRow) -> UndoFace {
    quick_text_face(row.label(), MENU_INK)
}

pub fn quick_value_face(text: &str, lit: bool) -> UndoFace {
    quick_text_face(text, if lit { MENU_INK } else { DIM_INK })
}

fn quick_text_face(label: &str, colour: [u8; 3]) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let layout = text::fit(font, label, OUT_W as f32, 1, MENU_PX, MENU_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = set.ceil() as u32 + 2 * MENU_PAD;
    let mut rgba = vec![0u8; (w * MENU_H * 4) as usize];
    text::draw_centred(&mut rgba, w, MENU_H, &layout, colour);
    UndoFace { rgba, w, h: MENU_H }
}

/// The least space kept between a row's label and its value.
const VALUE_GAP: f32 = 28.0;

/// The widest a value's face may be on a row whose label face is `label_w` wide, beside the
/// carets (their faces' widths, left and right) when the row is the one lit, so it never runs
/// into the label. Laid out as QuickMenu::draw lays it out.
pub fn quick_value_room(label_w: u32, carets: Option<(u32, u32)>) -> u32 {
    let pad = MENU_PAD as f32;
    let right = OUT_W as f32 - QUICK_EDGE;
    let label_end = QUICK_EDGE + label_w as f32 - 2.0 * pad;
    let room = match carets {
        None => right + 2.0 * pad - label_end - VALUE_GAP,
        Some((left, right_caret)) => {
            right - right_caret as f32 - 2.0 * CARET_GAP + 2.0 * pad
                - left as f32
                - label_end
                - VALUE_GAP
        }
    };
    room.max(0.0) as u32
}

pub fn quick_caret_face(right: bool) -> UndoFace {
    let glyph = if right { '\u{f0da}' } else { '\u{f0d9}' };
    let (Some(symbols), Some(label)) = (crate::icon::symbols_font(), text::label_font()) else {
        return UndoFace {
            rgba: Vec::new(),
            w: 0,
            h: 0,
        };
    };
    let (m, cov) = symbols.rasterize(glyph, CARET_PX);
    let (w, h) = (m.width as u32, MENU_H);
    let centre = match label.horizontal_line_metrics(MENU_PX) {
        Some(v) => {
            (h as f32 - v.new_line_size) / 2.0 + v.ascent
                - label.metrics('H', MENU_PX).height as f32 / 2.0
        }
        None => h as f32 / 2.0,
    };
    let top = (centre - m.height as f32 / 2.0).round() as i32;
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for gy in 0..m.height {
        let y = top + gy as i32;
        if y < 0 || y >= h as i32 {
            continue;
        }
        for gx in 0..m.width {
            let at = ((y as u32 * w + gx as u32) * 4) as usize;
            let a = cov[gy * m.width + gx];
            rgba[at..at + 4].copy_from_slice(&[MENU_INK[0], MENU_INK[1], MENU_INK[2], a]);
        }
    }
    UndoFace { rgba, w, h }
}

pub fn quick_legend_faces() -> [UndoFace; 4] {
    [
        hint_face("B", "Back"),
        arrows_hint_face("Change"),
        hint_face("A", "Open"),
        hint_face("A", "Reset"),
    ]
}

pub struct QuickMenuFaces {
    pub labels: Vec<(TexId, u32, u32)>,
    pub values: Vec<[(TexId, u32, u32); 2]>,
    pub carets: [(TexId, u32, u32); 2],
    /// Back, Change, Open, Reset.
    pub legend: [(TexId, u32); 4],
}

pub struct QuickMenu<'a> {
    pub row: QuickRow,
    pub values: [Option<QuickValue>; QuickRow::ALL.len()],
    pub clock: Option<[(TexId, u32, u32); 2]>,
    /// Rows whose value is free text rather than one of the QuickValues (an overlay's name):
    /// its faces, dim and lit.
    pub texts: [Option<[(TexId, u32, u32); 2]>; QuickRow::ALL.len()],
    pub faces: Option<&'a QuickMenuFaces>,
    /// Rows shown greyed out that do nothing, such as Cart Art and Update with Wi-Fi off.
    pub disabled: [bool; QuickRow::ALL.len()],
}

impl QuickMenu<'_> {
    pub fn draw(&self, out: &mut Vec<Draw>) {
        out.push(Draw::Rect {
            x: 0.0,
            y: 0.0,
            w: OUT_W as f32,
            h: OUT_H as f32,
            colour: opening(),
        });
        let layout = page_layout(self.row);
        out.push(Draw::Rect {
            x: 0.0,
            y: layout.row_top(self.row) + BAR_INSET,
            w: OUT_W as f32,
            h: layout.pitch - 2.0 * BAR_INSET,
            colour: edge(),
        });
        let Some(faces) = self.faces else {
            return;
        };
        let (right, pad) = (OUT_W as f32 - QUICK_EDGE, MENU_PAD as f32);
        for &row in self.row.page() {
            if !layout.shows(row) {
                continue;
            }
            let y = layout.row_top(row) + TYPE_DROP;
            let lit = row == self.row;
            if let Some(&(tex, w, h)) = faces.labels.get(row.index()) {
                let alpha = if self.disabled[row.index()] {
                    DISABLED_ALPHA
                } else {
                    1.0
                };
                push_alpha(out, tex, QUICK_EDGE - pad, y, w, h, alpha);
            }
            let value = match row {
                QuickRow::DateTime => self.clock.map(|c| c[lit as usize]),
                _ if self.texts[row.index()].is_some() => {
                    self.texts[row.index()].map(|t| t[lit as usize])
                }
                _ => self.values[row.index()]
                    .and_then(|v| faces.values.get(v.index()))
                    .map(|v| v[lit as usize]),
            };
            let Some((tex, w, h)) = value else {
                continue;
            };
            if !lit || row.opens() || row.info() {
                push(out, tex, right + pad - w as f32, y, w, h);
                continue;
            }
            let [(left_tex, lw, lh), (right_tex, rw, rh)] = faces.carets;
            let rx = right - rw as f32;
            push(out, right_tex, rx, y, rw, rh);
            let vx = rx - CARET_GAP + pad - w as f32;
            push(out, tex, vx, y, w, h);
            push(out, left_tex, vx + pad - CARET_GAP - lw as f32, y, lw, lh);
        }
        let [back, change, open, reset] = faces.legend;
        let other = match self.row {
            QuickRow::ResetSettings => reset,
            row if row.opens() => open,
            _ => change,
        };
        let done = self.values[self.row.index()] == Some(QuickValue::ResetDone);
        let only_back = self.disabled[self.row.index()] || done || self.row.info();
        let hints: &[(TexId, u32)] = if only_back { &[back] } else { &[back, other] };
        for (tex, w, x) in centred_hints(hints, LEGEND_GAP) {
            push(out, tex, x, LEGEND_Y, w, HINT_H);
        }
    }
}

/// Room kept at the top for the battery and Wi-Fi, so a long page does not cover them.
const STATUS_H: f32 = 40.0;
/// Room kept between the top of the last row and the split above the legend: the row's
/// drawn box (its type drop plus the row height) ends on the legend, not past it.
const LAST_ROW_CLEAR: f32 = TYPE_DROP + MENU_H as f32 - (LEGEND_Y - QUICK_SPLIT);

/// How a page is laid out: short pages are centred; a long one (Library → Systems) scrolls
/// so each row keeps the usual pitch and stays clear of the legend.
#[derive(Clone, Copy)]
struct PageLayout {
    pitch: f32,
    top: f32,
    start: usize,
    visible: usize,
}

impl PageLayout {
    fn shows(self, row: QuickRow) -> bool {
        let pos = row.position();
        pos >= self.start && pos < self.start + self.visible
    }

    fn row_top(self, row: QuickRow) -> f32 {
        self.top + self.pitch * (row.position() - self.start) as f32
    }
}

fn page_layout(selected: QuickRow) -> PageLayout {
    let n = selected.page().len();
    let room = QUICK_SPLIT - STATUS_H;
    let mut pitch = quick_pitch(n);
    if n > 0 && pitch * n as f32 > room {
        pitch = room / n as f32;
    }
    let block = pitch * n as f32;
    let mut top = (OUT_H as f32 - block) / 2.0;
    // A page that would centre its last row onto the legend is lifted so the type stays clear.
    let last = QUICK_SPLIT - LAST_ROW_CLEAR - pitch * (n.max(1) - 1) as f32;
    if top > last {
        top = last;
    }
    if top < STATUS_H {
        top = STATUS_H;
    }
    // When squeezing would stack the type (pitch below the row's type height),
    // scroll at the usual pitch instead. A short page, such as System, still
    // tightens slightly so every row stays on screen.
    if n > 0 && pitch + 0.5 < MENU_H as f32 && room >= QUICK_PITCH {
        pitch = QUICK_PITCH;
        let visible = ((room / pitch).floor() as usize).max(1).min(n);
        let pos = selected.position();
        let max_start = n.saturating_sub(visible);
        let mut start = pos.saturating_sub(visible / 3);
        if pos >= start + visible {
            start = pos + 1 - visible;
        }
        start = start.min(max_start);
        return PageLayout {
            pitch,
            top: STATUS_H,
            start,
            visible,
        };
    }
    PageLayout {
        pitch,
        top,
        start: 0,
        visible: n,
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn the_systems_list_scrolls_so_rows_stay_clear_of_the_legend() {
        let fav = page_layout(QuickRow::ShowFavorites);
        assert_eq!(fav.start, 0);
        assert!(fav.visible < QuickRow::SYSTEMS.len());
        assert!(fav.pitch >= QUICK_PITCH - 0.5);
        let y0 = fav.row_top(QuickRow::ShowFavorites);
        let y1 = fav.row_top(QuickRow::ShowGba);
        assert!(
            (y1 - y0 - fav.pitch).abs() < 0.1,
            "rows should sit a pitch apart"
        );
        let last_visible = QuickRow::SYSTEMS[fav.visible - 1];
        let bottom = fav.row_top(last_visible) + TYPE_DROP + MENU_H as f32;
        assert!(
            bottom <= LEGEND_Y,
            "last visible row {bottom} runs into the legend at {LEGEND_Y}"
        );

        let gw = page_layout(QuickRow::ShowGw);
        assert!(gw.shows(QuickRow::ShowGw));
        assert!(
            !gw.shows(QuickRow::ShowFavorites),
            "the first row should have scrolled off"
        );
        let gw_bottom = gw.row_top(QuickRow::ShowGw) + TYPE_DROP + MENU_H as f32;
        assert!(
            gw_bottom <= LEGEND_Y,
            "Game & Watch {gw_bottom} runs into the legend at {LEGEND_Y}"
        );
        assert!(gw.row_top(QuickRow::ShowGw) >= STATUS_H);
    }

    #[test]
    fn a_short_page_keeps_every_row_on_screen() {
        let leds = page_layout(QuickRow::SYSTEM[0]);
        assert_eq!(leds.start, 0);
        assert_eq!(leds.visible, QuickRow::SYSTEM.len());
        assert!(
            leds.shows(QuickRow::About),
            "About should stay on a short System page"
        );
        assert!(
            leds.pitch + 0.5 >= MENU_H as f32,
            "short pages should not drop below the type height, got {}",
            leds.pitch
        );
        let bottom = leds.row_top(QuickRow::About) + TYPE_DROP + MENU_H as f32;
        assert!(
            bottom <= LEGEND_Y,
            "About {bottom} runs into the legend at {LEGEND_Y}"
        );
    }
}

const DISABLED_ALPHA: f32 = 0.35;

fn push(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32) {
    push_alpha(out, tex, x, y, w, h, 1.0);
}

fn push_alpha(out: &mut Vec<Draw>, tex: TexId, x: f32, y: f32, w: u32, h: u32, alpha: f32) {
    out.push(Draw::Tex {
        x: x.round(),
        y: y.round(),
        w: w as f32,
        h: h as f32,
        tex,
        alpha,
    });
}
