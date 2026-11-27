//! Integer 320×240 letterboxing for a 320×480 (or 480×320) panel.
//!
//! The Slot layout stays 640×480 and is shown at exactly 1/2 scale. Until the
//! RG280V's framebuffer orientation is confirmed on the device, the placement
//! is chosen at runtime (env `SLOTTED_ORIENTATION` or a config file).

use std::sync::atomic::{AtomicU8, Ordering};

/// 320×240 is a sharp integer half of the 640×480 layout.
pub const CONTENT_W: u32 = 320;
pub const CONTENT_H: u32 = 240;

/// How the 320×240 picture sits on the panel, until we confirm the framebuffer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Orientation {
    /// Use the panel's own aspect: taller than wide is portrait, otherwise landscape.
    Auto = 0,
    /// 320×240 centred in a 320×480 panel (bars above and below).
    Portrait = 1,
    /// 320×240 centred in a 480×320 panel (bars on the sides).
    Landscape = 2,
    PortraitFlip = 3,
    LandscapeFlip = 4,
    /// Panel is landscape but the unit is held as portrait (or the reverse).
    Rotate90 = 5,
    Rotate270 = 6,
}

impl Orientation {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "auto" | "" => Some(Self::Auto),
            "portrait" | "port" => Some(Self::Portrait),
            "landscape" | "land" => Some(Self::Landscape),
            "portrait-flip" | "portrait-180" | "flip" => Some(Self::PortraitFlip),
            "landscape-flip" | "landscape-180" => Some(Self::LandscapeFlip),
            "rotate-90" | "rot90" | "90" => Some(Self::Rotate90),
            "rotate-270" | "rot270" | "270" => Some(Self::Rotate270),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Portrait => "portrait",
            Self::Landscape => "landscape",
            Self::PortraitFlip => "portrait-flip",
            Self::LandscapeFlip => "landscape-flip",
            Self::Rotate90 => "rotate-90",
            Self::Rotate270 => "rotate-270",
        }
    }

    /// 0, 90, 180 or 270 degrees clockwise of the 320×240 picture.
    pub fn degrees(self, panel: (u32, u32)) -> u16 {
        let resolved = self.resolve(panel);
        match resolved {
            Self::Portrait | Self::Landscape | Self::Auto => 0,
            Self::PortraitFlip | Self::LandscapeFlip => 180,
            Self::Rotate90 => 90,
            Self::Rotate270 => 270,
        }
    }

    pub fn resolve(self, panel: (u32, u32)) -> Self {
        match self {
            Self::Auto => {
                if panel.1 >= panel.0 {
                    Self::Portrait
                } else {
                    Self::Landscape
                }
            }
            other => other,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_orientation(o: Orientation) {
    CURRENT.store(o as u8, Ordering::Relaxed);
}

pub fn orientation() -> Orientation {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Orientation::Portrait,
        2 => Orientation::Landscape,
        3 => Orientation::PortraitFlip,
        4 => Orientation::LandscapeFlip,
        5 => Orientation::Rotate90,
        6 => Orientation::Rotate270,
        _ => Orientation::Auto,
    }
}

/// Where the 320×240 picture lands on `panel`, and how it is rotated.
///
/// The picture is always an integer half of 640×480. Extra panel pixels become
/// black bars. A 90°/270° rotation uses a 240×320 dest rectangle.
pub fn letterbox_rect(panel: (u32, u32), orient: Orientation) -> (i32, i32, i32, i32, u16) {
    let resolved = orient.resolve(panel);
    let deg = resolved.degrees(panel);
    let (cw, ch) = if deg == 90 || deg == 270 {
        (CONTENT_H, CONTENT_W)
    } else {
        (CONTENT_W, CONTENT_H)
    };
    let w = cw.min(panel.0) as i32;
    let h = ch.min(panel.1) as i32;
    let x = (panel.0 as i32 - w) / 2;
    let y = (panel.1 as i32 - h) / 2;
    (x, y, w, h, deg)
}

/// Load orientation from `SLOTTED_ORIENTATION`, then `SLOT_ORIENTATION`,
/// then the first existing file in `files` (one word, same as the env values).
pub fn load_from(files: &[&std::path::Path]) -> Orientation {
    for key in ["SLOTTED_ORIENTATION", "SLOT_ORIENTATION"] {
        if let Ok(v) = std::env::var(key) {
            if let Some(o) = Orientation::parse(&v) {
                eprintln!("slot: orientation {key}={}", o.as_str());
                return o;
            }
            eprintln!("slot: ignoring unknown {key}={v}");
        }
    }
    for path in files {
        if let Ok(text) = std::fs::read_to_string(path) {
            if let Some(o) = Orientation::parse(&text) {
                eprintln!("slot: orientation {}={}", path.display(), o.as_str());
                return o;
            }
        }
    }
    Orientation::Auto
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portrait_panel_centres_320x240_vertically() {
        let (x, y, w, h, deg) = letterbox_rect((320, 480), Orientation::Portrait);
        assert_eq!((x, y, w, h, deg), (0, 120, 320, 240, 0));
    }

    #[test]
    fn landscape_panel_centres_320x240() {
        let (x, y, w, h, deg) = letterbox_rect((480, 320), Orientation::Landscape);
        assert_eq!((x, y, w, h, deg), (80, 40, 320, 240, 0));
    }

    #[test]
    fn auto_follows_the_panel() {
        assert_eq!(
            Orientation::Auto.resolve((320, 480)),
            Orientation::Portrait
        );
        assert_eq!(
            Orientation::Auto.resolve((480, 320)),
            Orientation::Landscape
        );
        let (_, _, w, h, deg) = letterbox_rect((320, 480), Orientation::Auto);
        assert_eq!((w, h, deg), (320, 240, 0));
    }

    #[test]
    fn flip_is_180_in_the_same_box() {
        let a = letterbox_rect((320, 480), Orientation::Portrait);
        let b = letterbox_rect((320, 480), Orientation::PortraitFlip);
        assert_eq!((a.0, a.1, a.2, a.3), (b.0, b.1, b.2, b.3));
        assert_eq!(b.4, 180);
    }

    #[test]
    fn rotate_90_uses_a_240x320_box() {
        let (x, y, w, h, deg) = letterbox_rect((320, 480), Orientation::Rotate90);
        assert_eq!((x, y, w, h, deg), (40, 80, 240, 320, 90));
    }

    #[test]
    fn parse_accepts_common_names() {
        assert_eq!(Orientation::parse("portrait-flip"), Some(Orientation::PortraitFlip));
        assert_eq!(Orientation::parse("ROT90"), Some(Orientation::Rotate90));
        assert_eq!(Orientation::parse("nope"), None);
    }
}
