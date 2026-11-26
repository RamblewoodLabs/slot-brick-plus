//! The selected game's name on a card under its cart (HOME → Library → Title Card).
use slot_gfx::{Draw, TexId, OUT_H, OUT_W};

use crate::plate::UndoFace;
use crate::slot_chrome::MOUTH_H;
use crate::text;

/// Slim: the Game Boy and NES carts leave about 40px below them.
const CARD_H: u32 = 32;
const CARD_MAX_W: u32 = OUT_W - 2 * 40;
const PAD_X: u32 = 20;
const RADIUS: f32 = 10.0;
const PX: f32 = 20.0;
const MIN_PX: f32 = 12.0;
const GROUND: [u8; 4] = [0, 0, 0, 150];
const INK: [u8; 3] = [0xf6, 0xf4, 0xef];

/// The card: the name on one line, shrunk to fit, on a rounded dark ground as wide as it needs.
pub fn title_card_face(name: &str) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace { rgba: Vec::new(), w: 0, h: 0 };
    };
    let room = (CARD_MAX_W - 2 * PAD_X) as f32;
    let layout = text::fit(font, name, room, 1, PX, MIN_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = (set.ceil() as u32 + 2 * PAD_X).min(CARD_MAX_W);
    let mut rgba = rounded(w, CARD_H);
    text::draw_centred(&mut rgba, w, CARD_H, &layout, INK);
    UndoFace { rgba, w, h: CARD_H }
}

/// The filter pill in the shelf's top-left corner ("TOP 100", "A–Z"): what SELECT has the
/// shelf showing, there for as long as it does.
const PILL_H: u32 = 28;
const PILL_PAD_X: u32 = 14;
const PILL_PX: f32 = 15.0;
const PILL_X: f32 = 24.0;
const PILL_Y: f32 = 16.0;
const PILL_INK: [u8; 3] = [0xf2, 0xc6, 0x4b];

pub fn filter_pill_face(text: &str) -> UndoFace {
    let Some(font) = text::label_font() else {
        return UndoFace { rgba: Vec::new(), w: 0, h: 0 };
    };
    let layout = text::fit(font, text, (CARD_MAX_W / 2) as f32, 1, PILL_PX, MIN_PX);
    let set = layout
        .lines
        .iter()
        .map(|l| text::line_width(font, l, layout.px, layout.tracking))
        .fold(0.0, f32::max);
    let w = set.ceil() as u32 + 2 * PILL_PAD_X;
    let mut rgba = rounded_r(w, PILL_H, PILL_H as f32 / 2.0);
    text::draw_centred(&mut rgba, w, PILL_H, &layout, PILL_INK);
    UndoFace { rgba, w, h: PILL_H }
}

pub fn draw_filter_pill(pill: Option<(TexId, u32, u32)>, out: &mut Vec<Draw>) {
    let Some((tex, w, h)) = pill else {
        return;
    };
    out.push(Draw::Tex { x: PILL_X, y: PILL_Y, w: w as f32, h: h as f32, tex, alpha: 1.0 });
}

fn rounded(w: u32, h: u32) -> Vec<u8> {
    rounded_r(w, h, RADIUS)
}

fn rounded_r(w: u32, h: u32, radius: f32) -> Vec<u8> {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            // How far the pixel's centre is outside the rounded corner, in pixels.
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let cx = px.clamp(radius, w as f32 - radius);
            let cy = py.clamp(radius, h as f32 - radius);
            let out = ((px - cx).hypot(py - cy) - radius + 0.5).clamp(0.0, 1.0);
            let at = ((y * w + x) * 4) as usize;
            rgba[at..at + 3].copy_from_slice(&GROUND[..3]);
            rgba[at + 3] = (GROUND[3] as f32 * (1.0 - out)).round() as u8;
        }
    }
    rgba
}

/// The card, centred across the screen and in the room between the selected cart's foot and the
/// slot. Drawn before the shelf, so where the room is short the cart stays on top.
pub fn draw_title_card(card: Option<(TexId, u32, u32)>, cart_foot: f32, alpha: f32, out: &mut Vec<Draw>) {
    let (Some((tex, w, h)), true) = (card, alpha > 0.0) else {
        return;
    };
    let (w, h) = (w as f32, h as f32);
    let slot = OUT_H as f32 - MOUTH_H;
    let y = (cart_foot + (slot - cart_foot - h) / 2.0).min(slot - h);
    out.push(Draw::Tex {
        x: ((OUT_W as f32 - w) / 2.0).round(),
        y: y.round(),
        w,
        h,
        tex,
        alpha,
    });
}
