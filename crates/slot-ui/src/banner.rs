//! A small two-line toast in the top-right corner, over whatever is showing, in the
//! manner of Steam's achievement pop-ups: used for RetroAchievements (unlocks, the
//! game's achievement count, being offline). The app slides it in and out.
use crate::slot_chrome::{edge, opening};
use crate::text;
use crate::UndoFace;

pub const BANNER_W: u32 = 264;
pub const BANNER_H: u32 = 52;
/// Where it rests: the top-right corner, inset from both edges.
pub const BANNER_X: f32 = (slot_gfx::OUT_W - BANNER_W) as f32 - 10.0;
pub const BANNER_Y: f32 = 10.0;

/// The accent strip down the left edge, and the text inset beside it.
const STRIP: u32 = 5;
const INSET: u32 = STRIP + 10;
/// A badge's size, square, at the right end; the text stops short of it.
pub const BADGE_PX: u32 = 40;
const BADGE_GAP: u32 = 6;
const TITLE_PX: f32 = 17.0;
const DETAIL_PX: f32 = 13.0;

/// How long the unlock burst plays inside the toast, after it has slid in.
pub const BURST_MS: u32 = 700;

/// `badge`: BADGE_PX x BADGE_PX RGBA shown at the right end, like Steam's achievement icon.
pub fn banner_face(title: &str, detail: &str, badge: Option<&[u8]>) -> UndoFace {
    banner_face_at(title, detail, badge, None)
}

/// `burst` is 0..=1 through the celebration on an unlock toast, or None for a plain card.
pub fn banner_face_at(
    title: &str,
    detail: &str,
    badge: Option<&[u8]>,
    burst: Option<f32>,
) -> UndoFace {
    let (w, h) = (BANNER_W, BANNER_H);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let fill = |c: [f32; 4], alpha: u8| {
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
            alpha,
        ]
    };
    let (plate, rim) = (fill(opening(), 240), fill(edge(), 255));
    for y in 0..h {
        for x in 0..w {
            let accent = x < STRIP || y == 0 || y == h - 1 || x == w - 1;
            rgba[((y * w + x) * 4) as usize..][..4].copy_from_slice(if accent {
                &rim
            } else {
                &plate
            });
        }
    }
    let badge = badge.filter(|b| b.len() == (BADGE_PX * BADGE_PX * 4) as usize);
    // Room the text leaves at the right: the badge and its margins, or just a margin.
    let right = match badge {
        Some(px) => {
            let (bx, by) = (w - 1 - BADGE_GAP - BADGE_PX, (h - BADGE_PX) / 2);
            for y in 0..BADGE_PX {
                for x in 0..BADGE_PX {
                    let s = ((y * BADGE_PX + x) * 4) as usize;
                    let d = (((by + y) * w + bx + x) * 4) as usize;
                    let a = px[s + 3] as u32;
                    for c in 0..3 {
                        rgba[d + c] =
                            ((px[s + c] as u32 * a + rgba[d + c] as u32 * (255 - a)) / 255) as u8;
                    }
                }
            }
            BADGE_PX + BADGE_GAP + 8
        }
        None => 8,
    };
    if let Some(font) = text::label_font() {
        let lw = w - INSET - right;
        let mut line = |s: &str, top: u32, lh: u32, px: f32, ink: [u8; 3]| {
            let layout = text::fit(font, s, lw as f32, 1, px, px * 0.75);
            let cov = text::coverage(lw, lh, &layout);
            // fit centres the line; slide it to the left edge of its box.
            let shift = (0..lw)
                .find(|x| (0..lh).any(|y| cov[(y * lw + x) as usize] > 0))
                .unwrap_or(0);
            for y in 0..lh {
                for x in shift..lw {
                    let a = cov[(y * lw + x) as usize] as u32;
                    if a == 0 {
                        continue;
                    }
                    let b = (((top + y) * w + INSET + x - shift) * 4) as usize;
                    for (ch, c) in ink.iter().enumerate() {
                        rgba[b + ch] =
                            ((*c as u32 * a + rgba[b + ch] as u32 * (255 - a)) / 255) as u8;
                    }
                }
            }
        };
        line(title, 6, 24, TITLE_PX, [244, 244, 248]);
        line(detail, 29, 18, DETAIL_PX, [184, 184, 196]);
    }
    if let Some(u) = burst {
        let (bx, by) = match badge {
            Some(_) => (w - 1 - BADGE_GAP - BADGE_PX / 2, h / 2),
            None => (w * 3 / 4, h / 2),
        };
        paint_burst(&mut rgba, w, h, bx as i32, by as i32, u.clamp(0.0, 1.0));
    }
    UndoFace { rgba, w, h }
}

fn paint_burst(rgba: &mut [u8], w: u32, h: u32, cx: i32, cy: i32, u: f32) {
    // Grows quickly, then fades so the card is readable for the rest of the toast.
    let grow = (u / 0.35).clamp(0.0, 1.0);
    let fade = if u < 0.45 {
        1.0
    } else {
        (1.0 - (u - 0.45) / 0.55).clamp(0.0, 1.0)
    };
    if fade <= 0.01 {
        return;
    }
    let blend = |rgba: &mut [u8], x: i32, y: i32, r: u8, g: u8, b: u8, a: f32| {
        if x <= STRIP as i32 || y <= 0 || x >= w as i32 - 1 || y >= h as i32 - 1 {
            return;
        }
        let a = (a * 255.0).clamp(0.0, 255.0) as u32;
        if a == 0 {
            return;
        }
        let d = ((y as u32 * w + x as u32) * 4) as usize;
        if d + 3 >= rgba.len() {
            return;
        }
        for (i, c) in [r, g, b].into_iter().enumerate() {
            rgba[d + i] = ((c as u32 * a + rgba[d + i] as u32 * (255 - a)) / 255) as u8;
        }
        rgba[d + 3] = rgba[d + 3].max(240);
    };
    // Soft flash around the badge.
    let flash = ((1.0 - grow) * fade * 0.55).clamp(0.0, 1.0);
    if flash > 0.02 {
        let rad = 10.0 + grow * 16.0;
        let r2 = rad * rad;
        let x0 = (cx as f32 - rad).floor() as i32;
        let x1 = (cx as f32 + rad).ceil() as i32;
        let y0 = (cy as f32 - rad).floor() as i32;
        let y1 = (cy as f32 + rad).ceil() as i32;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                let d2 = dx * dx + dy * dy;
                if d2 > r2 {
                    continue;
                }
                let fall = 1.0 - (d2 / r2).sqrt();
                blend(rgba, x, y, 255, 236, 170, flash * fall);
            }
        }
    }
    // Gold rays.
    let rays = 10;
    let len = 6.0 + grow * 22.0;
    for i in 0..rays {
        let ang = (i as f32) * (std::f32::consts::TAU / rays as f32) + 0.12 + u * 0.15;
        let (ca, sa) = (ang.cos(), ang.sin());
        let steps = (len * 1.4) as i32;
        for s in 0..=steps {
            let t = s as f32 / steps.max(1) as f32;
            let x = cx as f32 + ca * len * t;
            let y = cy as f32 + sa * len * t * 0.72; // slightly flatter in the short card
            let a = fade * (1.0 - t) * (0.35 + 0.55 * grow);
            let (r, g, b) = if i % 2 == 0 {
                (255, 232, 140)
            } else {
                (255, 196, 72)
            };
            blend(rgba, x.round() as i32, y.round() as i32, r, g, b, a);
            if s % 2 == 0 {
                blend(
                    rgba,
                    x.round() as i32 + 1,
                    y.round() as i32,
                    r,
                    g,
                    b,
                    a * 0.5,
                );
            }
        }
    }
    // Sparks that fly out and fade.
    for i in 0..12 {
        let seed = (i as u32).wrapping_add(3).wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let ang = (seed % 360) as f32 * (std::f32::consts::PI / 180.0);
        let dist = 4.0 + grow * (10.0 + (seed % 17) as f32);
        let x = cx as f32 + ang.cos() * dist;
        let y = cy as f32 + ang.sin() * dist * 0.72;
        let twinkle = 0.45 + 0.55 * (((u * 13.0 + i as f32) * 1.7).sin() * 0.5 + 0.5);
        let a = fade * twinkle * (0.4 + 0.6 * grow);
        blend(rgba, x.round() as i32, y.round() as i32, 255, 248, 230, a);
        blend(
            rgba,
            x.round() as i32 + 1,
            y.round() as i32,
            255,
            220,
            120,
            a * 0.55,
        );
    }
    // Thin ring that expands around the badge.
    let rad = 8.0 + grow * 18.0;
    let a = fade * (1.0 - grow) * 0.85;
    if a > 0.02 {
        for i in 0..48 {
            let ang = i as f32 * (std::f32::consts::TAU / 48.0);
            let x = cx as f32 + ang.cos() * rad;
            let y = cy as f32 + ang.sin() * rad * 0.8;
            blend(rgba, x.round() as i32, y.round() as i32, 255, 214, 90, a);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toast_fits_the_corner_and_is_opaque_enough_to_read_over_a_game() {
        // A stand-in badge: a gold square with a darker ring.
        let badge: Vec<u8> = (0..BADGE_PX * BADGE_PX)
            .flat_map(|i| {
                let (x, y) = (i % BADGE_PX, i / BADGE_PX);
                let ring = x < 4 || y < 4 || x >= BADGE_PX - 4 || y >= BADGE_PX - 4;
                if ring {
                    [120, 84, 20, 255]
                } else {
                    [232, 182, 60, 255]
                }
            })
            .collect();
        // SLOT_BADGE_PNG=<file>: a real badge instead, scaled as the frontend does.
        let badge = std::env::var("SLOT_BADGE_PNG")
            .ok()
            .and_then(|p| crate::art::cover(std::path::Path::new(&p), BADGE_PX, BADGE_PX))
            .unwrap_or(badge);
        let f = banner_face("First Steps", "5 pts - Leave the house", Some(&badge));
        assert_eq!(
            (f.w, f.h, f.rgba.len()),
            (BANNER_W, BANNER_H, (BANNER_W * BANNER_H * 4) as usize)
        );
        assert!(f.rgba.chunks_exact(4).all(|p| p[3] >= 240));
        assert!(
            BANNER_X + BANNER_W as f32 <= slot_gfx::OUT_W as f32 - 8.0,
            "inside the right edge"
        );
        // SLOT_BANNER_PREVIEW=<file>: the toast in place on a 640x480 screen.
        if let Ok(path) = std::env::var("SLOT_BANNER_PREVIEW") {
            let (sw, sh) = (slot_gfx::OUT_W, slot_gfx::OUT_H);
            let mut screen: Vec<u8> = (0..sw * sh)
                .flat_map(|i| {
                    let v = if (i % sw / 32 + i / sw / 32) % 2 == 0 {
                        70
                    } else {
                        110
                    };
                    [v, v + 20, v, 255]
                })
                .collect();
            for y in 0..f.h {
                for x in 0..f.w {
                    let (src, dst) = (
                        ((y * f.w + x) * 4) as usize,
                        (((BANNER_Y as u32 + y) * sw + BANNER_X as u32 + x) * 4) as usize,
                    );
                    let a = f.rgba[src + 3] as u32;
                    for c in 0..3 {
                        screen[dst + c] = ((f.rgba[src + c] as u32 * a
                            + screen[dst + c] as u32 * (255 - a))
                            / 255) as u8;
                    }
                }
            }
            let mut e = png::Encoder::new(std::fs::File::create(path).unwrap(), sw, sh);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            e.write_header().unwrap().write_image_data(&screen).unwrap();
        }
    }

    #[test]
    fn text_starts_at_the_left_beside_the_strip() {
        let f = banner_face("I", "", None);
        let ink = |x: u32| (8..30).any(|y| f.rgba[((y * f.w + x) * 4) as usize] > 200);
        assert!(
            (INSET..INSET + 14).any(ink),
            "title starts beside the strip"
        );
        assert!(!(f.w / 2..f.w - 2).any(ink), "and not centred");
    }

    #[test]
    fn a_badge_sits_at_the_right_and_the_text_stops_short_of_it() {
        let white = vec![255u8; (BADGE_PX * BADGE_PX * 4) as usize];
        let f = banner_face("I", "", Some(&white));
        let px = |x: u32, y: u32| f.rgba[((y * f.w + x) * 4) as usize];
        let badge_x = f.w - 1 - BADGE_GAP - BADGE_PX;
        assert_eq!(
            px(badge_x + 2, BANNER_H / 2),
            255,
            "badge drawn at the right"
        );
        assert!(
            px(STRIP + BADGE_GAP + 2, BANNER_H / 2) < 200,
            "nothing at the left"
        );
        assert!(
            (INSET..INSET + 14).any(|x| (8..30).any(|y| px(x, y) > 200)),
            "text beside the strip"
        );
        let long = banner_face(&"W".repeat(60), "", Some(&white));
        let ink_until = (0..badge_x)
            .rev()
            .find(|&x| (8..30).any(|y| long.rgba[((y * long.w + x) * 4) as usize] > 200));
        assert!(
            ink_until.is_some_and(|x| x < badge_x - 4),
            "long text stops before the badge: {ink_until:?}"
        );
        let wrong = vec![0u8; 12];
        assert_eq!(
            banner_face("I", "", Some(&wrong)).rgba,
            banner_face("I", "", None).rgba,
            "a bad image is left out"
        );
    }

    #[test]
    fn an_unlock_burst_lights_the_badge_and_then_clears() {
        let white = vec![255u8; (BADGE_PX * BADGE_PX * 4) as usize];
        let still = banner_face("First Steps", "5 pts - Leave the house", Some(&white));
        let mid = banner_face_at(
            "First Steps",
            "5 pts - Leave the house",
            Some(&white),
            Some(0.25),
        );
        let done = banner_face_at(
            "First Steps",
            "5 pts - Leave the house",
            Some(&white),
            Some(1.0),
        );
        let lit = |f: &UndoFace| {
            f.rgba
                .chunks_exact(4)
                .zip(still.rgba.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count()
        };
        assert!(
            lit(&mid) > 40,
            "a mid-burst paints sparkles inside the card: {}",
            lit(&mid)
        );
        assert_eq!(
            done.rgba, still.rgba,
            "the burst is gone at the end, so the text stays readable"
        );
        // SLOT_BANNER_GIF=<dir>: 640x480 frames of the toast sliding in with its burst.
        if let Ok(dir) = std::env::var("SLOT_BANNER_GIF") {
            let _ = std::fs::create_dir_all(&dir);
            let (sw, sh) = (slot_gfx::OUT_W, slot_gfx::OUT_H);
            let bg: Vec<u8> = (0..sw * sh)
                .flat_map(|i| {
                    let v = if (i % sw / 32 + i / sw / 32) % 2 == 0 {
                        70
                    } else {
                        110
                    };
                    [v, v + 20, v, 255]
                })
                .collect();
            // 20 fps through the slide, the burst, and a second of rest (the extra hold).
            let fps = 20u32;
            let end_ms = BURST_MS + 300 + 1000;
            let frames = (end_ms * fps + 999) / 1000;
            for i in 0..frames {
                let ms = i * 1000 / fps;
                let shown = (ms as f32 / 300.0).clamp(0.0, 1.0);
                let eased = 1.0 - (1.0 - shown) * (1.0 - shown);
                let away = sw as f32 - BANNER_X;
                let x = (BANNER_X + away * (1.0 - eased)).round() as i32;
                let burst =
                    Some(((ms as i32 - 300).max(0) as f32 / BURST_MS as f32).clamp(0.0, 1.0));
                let f = banner_face_at(
                    "First Steps",
                    "5 pts - Leave the house",
                    Some(&white),
                    burst,
                );
                let mut screen = bg.clone();
                for y in 0..f.h {
                    for px in 0..f.w {
                        let dx = x + px as i32;
                        let dy = BANNER_Y as i32 + y as i32;
                        if dx < 0 || dy < 0 || dx >= sw as i32 || dy >= sh as i32 {
                            continue;
                        }
                        let (src, dst) = (
                            ((y * f.w + px) * 4) as usize,
                            ((dy as u32 * sw + dx as u32) * 4) as usize,
                        );
                        let a = f.rgba[src + 3] as u32;
                        for c in 0..3 {
                            screen[dst + c] = ((f.rgba[src + c] as u32 * a
                                + screen[dst + c] as u32 * (255 - a))
                                / 255) as u8;
                        }
                    }
                }
                let path = std::path::Path::new(&dir).join(format!("frame-{i:03}.png"));
                let mut e = png::Encoder::new(std::fs::File::create(&path).unwrap(), sw, sh);
                e.set_color(png::ColorType::Rgba);
                e.set_depth(png::BitDepth::Eight);
                e.write_header().unwrap().write_image_data(&screen).unwrap();
            }
        }
    }
}
