//! A saved video on the Videos shelf: a short strip of film, picture in the gate.
use std::path::Path;

use crate::art;
use crate::plate::UndoFace;

pub const TAPE_W: u32 = 300;
pub const TAPE_H: u32 = 176;
/// Where the selected tape sits on the shelf, so the title card can clear its foot.
pub const TAPE_Y: f32 = 148.0;

const BASE: [u8; 3] = [0x16, 0x16, 0x18];
const SHEEN: [u8; 3] = [0x2a, 0x2a, 0x30];
const HOLE: [u8; 3] = [0x08, 0x08, 0x0a];
const FRAME: [u8; 3] = [0x3a, 0x38, 0x36];
const GATE: [u8; 3] = [0x0c, 0x0c, 0x0e];

const BAND: u32 = 22;
const HOLE_W: u32 = 10;
const HOLE_H: u32 = 8;
const HOLE_PITCH: u32 = 16;
const GATE_X: u32 = 28;
const GATE_Y: u32 = 18;
const GATE_W: u32 = 244;
const GATE_H: u32 = 140;

pub fn tape_foot() -> f32 {
    TAPE_Y + TAPE_H as f32
}

/// The film strip for one saved video. `thumb` is a PNG drawn inside the gate.
pub fn tape_face(thumb: Option<&Path>) -> UndoFace {
    let mut rgba = vec![0u8; (TAPE_W * TAPE_H * 4) as usize];
    for y in 0..TAPE_H {
        for x in 0..TAPE_W {
            let sheen = x > TAPE_W / 2 - 8 && x < TAPE_W / 2 + 8;
            put(&mut rgba, x, y, if sheen { SHEEN } else { BASE });
        }
    }
    sprockets(&mut rgba, 6);
    sprockets(&mut rgba, TAPE_W - BAND + 6);
    for y in GATE_Y..GATE_Y + GATE_H {
        for x in GATE_X..GATE_X + GATE_W {
            if rounded(x, y) {
                put(&mut rgba, x, y, GATE);
            }
        }
    }
    if let Some(path) = thumb {
        if let Some(pic) = art::cover(path, GATE_W - 8, GATE_H - 8) {
            blit_rounded(
                &mut rgba,
                GATE_X + 4,
                GATE_Y + 4,
                GATE_W - 8,
                GATE_H - 8,
                &pic,
            );
        }
    }
    border(&mut rgba);
    UndoFace {
        rgba,
        w: TAPE_W,
        h: TAPE_H,
    }
}

fn sprockets(rgba: &mut [u8], x0: u32) {
    let mut y = 8u32;
    while y + HOLE_H < TAPE_H - 4 {
        for yy in y..y + HOLE_H {
            for xx in x0..x0 + HOLE_W {
                put(rgba, xx, yy, HOLE);
            }
        }
        y += HOLE_PITCH;
    }
}

fn rounded(x: u32, y: u32) -> bool {
    let r = 6i32;
    let (x, y) = (x as i32 - GATE_X as i32, y as i32 - GATE_Y as i32);
    let (w, h) = (GATE_W as i32, GATE_H as i32);
    let cx = if x < r {
        r
    } else if x >= w - r {
        w - r - 1
    } else {
        x
    };
    let cy = if y < r {
        r
    } else if y >= h - r {
        h - r - 1
    } else {
        y
    };
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= r * r
}

fn border(rgba: &mut [u8]) {
    for x in GATE_X..GATE_X + GATE_W {
        if rounded(x, GATE_Y) {
            put(rgba, x, GATE_Y, FRAME);
        }
        if rounded(x, GATE_Y + GATE_H - 1) {
            put(rgba, x, GATE_Y + GATE_H - 1, FRAME);
        }
    }
    for y in GATE_Y..GATE_Y + GATE_H {
        if rounded(GATE_X, y) {
            put(rgba, GATE_X, y, FRAME);
        }
        if rounded(GATE_X + GATE_W - 1, y) {
            put(rgba, GATE_X + GATE_W - 1, y, FRAME);
        }
    }
}

fn put(rgba: &mut [u8], x: u32, y: u32, rgb: [u8; 3]) {
    if x >= TAPE_W || y >= TAPE_H {
        return;
    }
    let i = ((y * TAPE_W + x) * 4) as usize;
    rgba[i] = rgb[0];
    rgba[i + 1] = rgb[1];
    rgba[i + 2] = rgb[2];
    rgba[i + 3] = 255;
}

fn blit_rounded(dst: &mut [u8], x: u32, y: u32, w: u32, h: u32, src: &[u8]) {
    for yy in 0..h {
        for xx in 0..w {
            if !rounded(x + xx, y + yy) {
                continue;
            }
            let s = ((yy * w + xx) * 4) as usize;
            if s + 3 >= src.len() || src[s + 3] == 0 {
                continue;
            }
            let i = (((y + yy) * TAPE_W + x + xx) * 4) as usize;
            if i + 3 >= dst.len() {
                continue;
            }
            dst[i..i + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
}
