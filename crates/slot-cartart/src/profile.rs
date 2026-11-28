//! What each system's ScreenScraper pictures look like: where the label sits on its cart
//! photo, what its label scan holds, and which picture to try first.

use crate::Kind;

/// Where the label is on a system's `support-2D` cart picture.
#[derive(Clone, Copy, Debug)]
pub struct Photo {
    /// The label, as fractions `[x0, y0, x1, y1]` of the cart's outline (backdrop removed).
    pub label: [f32; 4],
    /// Plain plastic near the label, the same way: its colour tells the label's edge.
    pub plastic: [f32; 4],
    /// The cart's outline's width / height; a picture far from it is not this cart.
    pub aspect: f32,
    pub tolerance: f32,
    /// Move the edges to where the plastic stops (off for a disc, or a label the colour of its cart).
    pub refine: bool,
    /// Straighten a cart that sits askew (never a disc: it is round).
    pub deskew: bool,
}

impl Photo {
    /// The label's own width / height (of a cart of exactly `aspect`).
    pub fn label_aspect(&self) -> f32 {
        let l = self.label;
        (l[2] - l[0]) / (l[3] - l[1]) * self.aspect
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub dir: &'static str,
    /// The pictures to try, best first.
    pub order: &'static [Kind],
    pub photo: Option<Photo>,
    /// A second cart the system came on (the Famicom for NES, the PAL shape for SNES).
    pub photo_alt: Option<Photo>,
    /// The part of the label scan that is the label window, when the scan holds more.
    pub texture: Option<[f32; 4]>,
    /// The shape the scan should have (the window's when None) and how far off it may be.
    pub texture_aspect: Option<f32>,
    pub texture_tolerance: f32,
    /// How much of the scan's edge to cut away (rounded corners, backdrop fringe).
    pub texture_inset: f32,
    /// Straighten a label scan that sits askew (not a disc's).
    pub texture_deskew: bool,
    /// A box cover is cut from the top (its title) rather than where the detail is.
    pub box_top: bool,
    /// A logo goes on black (a backlit marquee) rather than a colour from the art.
    pub black_field: bool,
    /// The label is the whole object, kept with its own outline on transparency and standing
    /// on the bottom edge (a Game & Watch: the "label" is the handheld itself).
    pub whole_object: bool,
    /// How far (ln of the shapes' ratio) a label may be cut to the window's shape; past that
    /// its flat edge is carried out instead, so no lettering is lost (a square HuCard label
    /// in a tall window).
    pub label_cut: f32,
}

const CART: &[Kind] = &[Kind::Texture, Kind::Photo, Kind::Box, Kind::Logo];
const PHOTO_FIRST: &[Kind] = &[Kind::Photo, Kind::Texture, Kind::Box, Kind::Logo];
const BOX_FIRST: &[Kind] = &[Kind::Box, Kind::Texture, Kind::Photo, Kind::Logo];
const ARCADE: &[Kind] = &[Kind::Marquee, Kind::Logo, Kind::Box];
/// A disk's sticker: a box's top band is the publisher's banner, so the title logo comes first.
const STICKER: &[Kind] = &[Kind::Texture, Kind::Photo, Kind::Logo, Kind::Box];

const BASE: Profile = Profile {
    dir: "",
    order: CART,
    photo: None,
    photo_alt: None,
    texture: None,
    texture_aspect: None,
    texture_tolerance: 0.223, // ln(1.25)
    texture_inset: 0.012,
    texture_deskew: true,
    box_top: true,
    black_field: false,
    whole_object: false,
    label_cut: 0.419, // 1.6 × ln(1.3)
};

const fn photo(label: [f32; 4], plastic: [f32; 4], aspect: f32) -> Option<Photo> {
    Some(Photo {
        label,
        plastic,
        aspect,
        tolerance: 0.10,
        refine: true,
        deskew: true,
    })
}

/// Every system Slot has a shelf for, by card folder.
pub const SYSTEMS: &[&str] = &[
    "GBA", "GB", "GBC", "NES", "SNES", "PS1", "MD", "SMS", "GG", "ARCADE", "PCE", "A2600", "INTV",
    "LYNX", "A7800", "GW", "COLECO", "A5200", "A800", "VECTREX", "32X", "N64", "NDS", "PSP",
];

pub fn profile(dir: &str) -> Profile {
    let p = |dir: &'static str| Profile { dir, ..BASE };
    match dir {
        "GB" | "GBC" => Profile {
            photo: photo([0.15, 0.40, 0.85, 0.93], [0.05, 0.42, 0.11, 0.9], 0.885),
            ..p("GB")
        },
        "GBA" => Profile {
            photo: photo([0.17, 0.22, 0.83, 0.86], [0.03, 0.4, 0.08, 0.8], 1.70),
            ..p("GBA")
        },
        "NES" => Profile {
            photo: photo([0.405, 0.006, 0.865, 0.68], [0.42, 0.73, 0.85, 0.80], 0.93),
            photo_alt: photo([0.08, 0.12, 0.92, 0.80], [0.3, 0.88, 0.7, 0.95], 1.5),
            ..p("NES")
        },
        "SNES" => Profile {
            photo: photo([0.20, 0.008, 0.807, 0.432], [0.25, 0.47, 0.75, 0.55], 1.08),
            ..p("SNES")
        },
        // The disc is the label: the scan is kept whole, cut round by the shelf.
        "PS1" => Profile {
            photo: Some(Photo {
                label: [0.0, 0.0, 1.0, 1.0],
                plastic: [0.0; 4],
                aspect: 1.0,
                tolerance: 0.08,
                refine: false,
                deskew: false,
            }),
            order: PHOTO_FIRST,
            texture_inset: 0.0,
            texture_deskew: false,
            ..p("PS1")
        },
        "MD" => Profile {
            photo: photo([0.158, 0.02, 0.842, 0.848], [0.03, 0.4, 0.09, 0.8], 1.47),
            ..p("MD")
        },
        "32X" => Profile {
            photo: photo([0.18, 0.03, 0.82, 0.74], [0.03, 0.4, 0.09, 0.8], 1.52),
            ..p("32X")
        },
        // The Sega Card's label is box-style art.
        "SMS" => Profile {
            order: CART,
            ..p("SMS")
        },
        "GG" => Profile {
            photo: photo([0.10, 0.18, 0.90, 0.78], [0.03, 0.4, 0.07, 0.7], 0.99),
            ..p("GG")
        },
        "ARCADE" => Profile {
            order: ARCADE,
            black_field: true,
            box_top: false,
            ..p("ARCADE")
        },
        // The HuCard photo is framed as the shell is (213 x 331, window 16, 77, 181, 237).
        // The label scan is the square HuCard label; the shelf's window is the card's taller
        // face, so the scan keeps every letter and its flat ground is carried out to fill it.
        "PCE" => Profile {
            photo: photo([0.075, 0.233, 0.925, 0.949], [0.3, 0.18, 0.7, 0.21], 0.643),
            texture_aspect: Some(1.0),
            texture_tolerance: 0.15,
            label_cut: 0.06,
            ..p("PCE")
        },
        "A2600" => Profile {
            photo: photo([0.07, 0.08, 0.93, 0.93], [0.02, 0.4, 0.05, 0.8], 0.84),
            ..p("A2600")
        },
        "A7800" => Profile {
            photo: photo([0.07, 0.04, 0.93, 0.93], [0.02, 0.4, 0.05, 0.8], 0.84),
            ..p("A7800")
        },
        "INTV" => Profile {
            photo: photo([0.076, 0.796, 0.924, 0.951], [0.1, 0.6, 0.9, 0.75], 0.81),
            ..p("INTV")
        },
        "LYNX" => Profile {
            photo: photo([0.05, 0.04, 0.96, 0.84], [0.1, 0.88, 0.9, 0.97], 0.96),
            ..p("LYNX")
        },
        "COLECO" => Profile {
            photo: photo([0.06, 0.02, 0.94, 0.66], [0.1, 0.75, 0.9, 0.9], 0.87),
            ..p("COLECO")
        },
        "A5200" => Profile {
            photo: photo([0.12, 0.04, 0.88, 0.78], [0.02, 0.4, 0.08, 0.8], 1.09),
            ..p("A5200")
        },
        "A800" => Profile {
            photo: photo([0.05, 0.02, 0.95, 0.30], [0.1, 0.4, 0.9, 0.6], 0.99),
            order: STICKER,
            ..p("A800")
        },
        // The console's screen shows the overlay art cut from the box (not the whole cover).
        "VECTREX" => Profile {
            order: BOX_FIRST,
            box_top: false,
            ..p("VECTREX")
        },
        // Each game is its own handheld (rendered by brick/gw-art); failing that, its box.
        // ScreenScraper's G&W "cart" pictures are the handheld itself, cut out: just what the shelf shows.
        "GW" => Profile {
            order: PHOTO_FIRST,
            photo: Some(Photo {
                label: [0.0, 0.0, 1.0, 1.0],
                plastic: [0.0; 4],
                aspect: 1.0,
                tolerance: 9.0,
                refine: false,
                deskew: false,
            }),
            whole_object: true,
            box_top: false,
            ..p("GW")
        },
        "N64" => Profile {
            photo: photo([0.31, 0.14, 0.71, 0.95], [0.1, 0.5, 0.2, 0.9], 1.55),
            ..p("N64")
        },
        "NDS" => Profile {
            photo: photo([0.07, 0.12, 0.93, 0.86], [0.1, 0.9, 0.9, 0.97], 0.95),
            ..p("NDS")
        },
        "PSP" => Profile {
            photo: Some(Photo {
                label: [0.079, 0.055, 0.934, 0.94],
                plastic: [0.0; 4],
                aspect: 1.01,
                tolerance: 0.08,
                refine: false,
                deskew: false,
            }),
            // The label scan is the UMD's printed face, flat and whole: better than the photo,
            // which carries the disc's clear rim and the shell's corners.
            texture_deskew: false,
            ..p("PSP")
        },
        _ => BASE,
    }
}
