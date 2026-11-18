//! A saved video on the Videos shelf: a tape, with the picture as its label.
use std::path::Path;

use crate::art;
use crate::plate::UndoFace;

pub const TAPE_W: u32 = 300;
pub const TAPE_H: u32 = 176;
/// Where the selected tape sits on the shelf, so the title card can clear its foot.
pub const TAPE_Y: f32 = 148.0;

const BODY: [u8; 3] = [0x2c, 0x26, 0x22];
const EDGE: [u8; 3] = [0x14, 0x11, 0x10];
const WINDOW: [u8; 3] = [0x18, 0x1c, 0x24];
const REEL: [u8; 3] = [0x1a, 0x17, 0x16];
const HUB: [u8; 3] = [0xc4, 0xb8, 0xa4];
const SPINE: [u8; 3] = [0x8c, 0x2e, 0x2a];

const WIN_X: u32 = 22;
const WIN_Y: u32 = 16;
const WIN_W: u32 = 256;
const WIN_H: u32 = 104;

pub fn tape_foot() -> f32 {
    TAPE_Y + TAPE_H as f32
}

/// The tape drawn for one saved video. `thumb` is a PNG; the picture fills the label window.
pub fn tape_face(thumb: Option<&Path>) -> UndoFace {
    let mut rgba = vec![0u8; (TAPE_W * TAPE_H * 4) as usize];
    for y in 0..TAPE_H {
        for x in 0..TAPE_W {
            if !inside(x, y) {
                continue;
            }
            put(&mut rgba, x, y, BODY);
        }
    }
    for y in 8..TAPE_H - 8 {
        put(&mut rgba, 8, y, SPINE);
        put(&mut rgba, 9, y, SPINE);
    }
    for y in WIN_Y..WIN_Y + WIN_H {
        for x in WIN_X..WIN_X + WIN_W {
            put(&mut rgba, x, y, WINDOW);
        }
    }
    if let Some(path) = thumb {
        if let Some(pic) = art::cover(path, WIN_W - 4, WIN_H - 4) {
            blit(&mut rgba, WIN_X + 2, WIN_Y + 2, WIN_W - 4, WIN_H - 4, &pic);
        }
    }
    frame(&mut rgba, WIN_X - 2, WIN_Y - 2, WIN_W + 4, WIN_H + 4);
    reel(&mut rgba, 78, 148, 16);
    reel(&mut rgba, 222, 148, 16);
    UndoFace {
        rgba,
        w: TAPE_W,
        h: TAPE_H,
    }
}

fn inside(x: u32, y: u32) -> bool {
    let r = 12i32;
    let (x, y) = (x as i32, y as i32);
    let (w, h) = (TAPE_W as i32, TAPE_H as i32);
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

fn put(rgba: &mut [u8], x: u32, y: u32, rgb: [u8; 3]) {
    let i = ((y * TAPE_W + x) * 4) as usize;
    rgba[i] = rgb[0];
    rgba[i + 1] = rgb[1];
    rgba[i + 2] = rgb[2];
    rgba[i + 3] = 255;
}

fn blit(dst: &mut [u8], x: u32, y: u32, w: u32, h: u32, src: &[u8]) {
    for yy in 0..h {
        for xx in 0..w {
            let s = ((yy * w + xx) * 4) as usize;
            if s + 3 >= src.len() || src[s + 3] == 0 {
                continue;
            }
            let i = (((y + yy) * TAPE_W + x + xx) * 4) as usize;
            dst[i..i + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
}

fn frame(rgba: &mut [u8], x: u32, y: u32, w: u32, h: u32) {
    for xx in x..x + w {
        put(rgba, xx, y, EDGE);
        put(rgba, xx, y + h - 1, EDGE);
    }
    for yy in y..y + h {
        put(rgba, x, yy, EDGE);
        put(rgba, x + w - 1, yy, EDGE);
    }
}

fn reel(rgba: &mut [u8], cx: i32, cy: i32, r: i32) {
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            let (dx, dy) = (x - cx, y - cy);
            let d = dx * dx + dy * dy;
            if d > r * r || x < 0 || y < 0 {
                continue;
            }
            let rgb = if d <= 5 * 5 { HUB } else { REEL };
            put(rgba, x as u32, y as u32, rgb);
        }
    }
}
