//! A cart turning to face the slot before it goes in. The shell is pre-rendered per platform
//! (`assets/carts/<sys>/insert/`); the game's own label is laid onto each frame here through
//! the frame's label map, lit as the shelf face is. A Game & Watch has no shell: its frames
//! are its own, rendered for each game (`gw_faces`). See `docs/cart-shells.md`.
use slot_store::{Cart, Platform};

use crate::cart::CartFace;
use crate::shell_png;

include!(concat!(env!("OUT_DIR"), "/turns.rs"));

/// The turn a platform's cart makes, when it has one.
pub fn of(platform: Platform) -> Option<&'static Turn> {
    turn_by_dir(platform.def().shell?)
}

/// How long a platform's cart takes to turn; 0 when it goes straight in.
pub fn seconds(platform: Platform) -> f32 {
    if platform == Platform::Gw {
        return GW_OPEN_S + GW_HOLD_S;
    }
    if spins(platform) {
        return SPIN_FRAMES as f32 / SPIN_FPS + SPIN_HOLD_S;
    }
    of(platform).map_or(0.0, Turn::seconds)
}

/// Which of `n` frames shows `u` (0..1) of the way through a platform's turn. A rendered turn
/// plays its frames at their own rate (they carry their own easing) and then rests on the
/// last one for its `hold`; a Game & Watch strip is eased across `GW_OPEN_S`, then rests.
pub fn frame_index(platform: Platform, u: f32, n: usize) -> usize {
    if n == 0 {
        return 0;
    }
    let total = seconds(platform);
    let (motion, eased) = if platform == Platform::Gw {
        (GW_OPEN_S, true)
    } else if spins(platform) {
        (SPIN_FRAMES as f32 / SPIN_FPS, false)
    } else {
        match of(platform) {
            Some(t) => (t.frames.len() as f32 / t.fps, false),
            None => return n - 1,
        }
    };
    if total <= 0.0 || motion <= 0.0 {
        return n - 1;
    }
    let mut v = (u.clamp(0.0, 1.0) * total / motion).min(1.0);
    if eased {
        v = ease(v);
    }
    ((v * n as f32) as usize).min(n - 1)
}

/// Ease in and out with no overshoot: slow at both ends, never past either.
pub fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Whether a platform's carts turn (or, a Game & Watch, open) before they go in.
pub fn turns(platform: Platform) -> bool {
    platform == Platform::Gw || spins(platform) || of(platform).is_some()
}

/// The six first systems have no rendered shell: their shelf face is drawn by `cart`/`disc`,
/// so their turn is made here from that very face (`spin_faces`), at the same pace as the
/// rendered ones: 64 eased frames at 30 fps, then a settled half second before the drop.
pub const SPIN_FRAMES: usize = 64;
pub const SPIN_FPS: f32 = 30.0;
pub const SPIN_HOLD_S: f32 = 0.5;

pub fn spins(platform: Platform) -> bool {
    matches!(
        platform,
        Platform::Gb
            | Platform::Gbc
            | Platform::Gba
            | Platform::Nes
            | Platform::Snes
            | Platform::Ps1
    )
}

/// A cart's depth as a share of its width (Nintendo's own sizes, in mm).
fn depth(platform: Platform) -> f32 {
    match platform {
        Platform::Gb | Platform::Gbc => 8.0 / 57.0,
        Platform::Gba => 8.0 / 57.5,
        Platform::Nes => 17.5 / 120.5,
        Platform::Snes => 20.0 / 128.0,
        _ => 0.12,
    }
}

/// The turn of a cart with no rendered shell, made from its shelf face. A cart goes once round
/// on its upright axis, showing its edge and plain back on the way (shaded as it turns from
/// the light), and lifts a little; a PlayStation disc spins twice in its own plane, like the
/// disc in a UMD. Both are eased with no overshoot and end exactly on the shelf face, square
/// to the slot, so nothing turns while it goes down.
pub fn spin_faces(platform: Platform, face: &CartFace) -> TurnFaces {
    let n = SPIN_FRAMES;
    let mut frames: Vec<TurnFace> = (0..n)
        .map(|i| {
            let s = ease(i as f32 / (n - 1) as f32);
            let lift = (face.h as f32 * 0.05 * (std::f32::consts::PI * s).sin()).round() as i32;
            let mut f = if platform == Platform::Ps1 {
                disc_turn(face, s)
            } else {
                cart_turn(face, s, depth(platform))
            };
            f.y -= lift;
            f
        })
        .collect();
    shrink_frames(&mut frames);
    TurnFaces {
        frames,
        seated: None,
    }
}

fn px_at(face: &CartFace, x: i32, y: i32) -> [f32; 4] {
    if x < 0 || y < 0 || x >= face.w as i32 || y >= face.h as i32 {
        return [0.0; 4];
    }
    let i = ((y as u32 * face.w + x as u32) * 4) as usize;
    let p = &face.rgba[i..i + 4];
    // premultiplied, so blending at an edge does not pick up a dark fringe
    let a = p[3] as f32 / 255.0;
    [
        p[0] as f32 * a,
        p[1] as f32 * a,
        p[2] as f32 * a,
        p[3] as f32,
    ]
}

/// Bilinear sample at (x, y) in pixel-centre coordinates, premultiplied; clear outside.
fn sample_clear(face: &CartFace, x: f32, y: f32) -> [f32; 4] {
    let (x, y) = (x - 0.5, y - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let (x0, y0) = (x0 as i32, y0 as i32);
    let mut out = [0.0; 4];
    for (dx, dy, k) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        if k == 0.0 {
            continue;
        }
        let p = px_at(face, x0 + dx, y0 + dy);
        for c in 0..4 {
            out[c] += p[c] * k;
        }
    }
    out
}

fn unpremul(p: [f32; 4]) -> [u8; 4] {
    if p[3] <= 0.0 {
        return [0; 4];
    }
    let a = p[3] / 255.0;
    [
        (p[0] / a).round().clamp(0.0, 255.0) as u8,
        (p[1] / a).round().clamp(0.0, 255.0) as u8,
        (p[2] / a).round().clamp(0.0, 255.0) as u8,
        p[3].round().clamp(0.0, 255.0) as u8,
    ]
}

fn cart_turn(face: &CartFace, s: f32, depth: f32) -> TurnFace {
    let (w, h) = (face.w as i32, face.h as i32);
    let solid = |x: i32, y: i32| face.rgba[((y * w + x) * 4 + 3) as usize] > 128;
    let rows: Vec<Option<(f32, f32)>> = (0..h)
        .map(|y| {
            let l = (0..w).find(|&x| solid(x, y))?;
            let r = (0..w).rev().find(|&x| solid(x, y))?;
            Some((l as f32, r as f32 + 1.0))
        })
        .collect();
    let (l, r) = rows
        .iter()
        .flatten()
        .fold((f32::MAX, f32::MIN), |(l, r), &(a, b)| (l.min(a), r.max(b)));
    if l >= r {
        return TurnFace {
            face: CartFace {
                rgba: face.rgba.clone(),
                w: face.w,
                h: face.h,
            },
            x: 0,
            y: 0,
            dest_scale: 1,
        };
    }
    let cx = (l + r) / 2.0;
    let d = depth * (r - l);
    // the plastic's own colour: the rim just inside the outline, all round
    let mut sum = [0.0f32; 3];
    let mut count = 0.0;
    for (y, row) in rows.iter().enumerate() {
        if let Some((a, b)) = row {
            for x in (*a as i32 + 1..*a as i32 + 4).chain(*b as i32 - 4..*b as i32 - 1) {
                if x >= 0 && x < w && solid(x, y as i32) {
                    let i = ((y as i32 * w + x) * 4) as usize;
                    for c in 0..3 {
                        sum[c] += face.rgba[i + c] as f32;
                    }
                    count += 1.0;
                }
            }
        }
    }
    let base = if count > 0.0 {
        [sum[0] / count, sum[1] / count, sum[2] / count]
    } else {
        [90.0, 90.0, 90.0]
    };
    let th = std::f32::consts::TAU * s;
    let (sn, c) = th.sin_cos();
    let pad = (d / 2.0).ceil() as i32 + 2;
    let ow = w + 2 * pad;
    let mut rgba = vec![0u8; (ow * h * 4) as usize];
    for (y, row) in rows.iter().enumerate() {
        for ox in 0..ow {
            let xo = ox as f32 + 0.5 - pad as f32 - cx;
            let mut out = [0.0f32; 4];
            // the edge that has come round toward the viewer
            if let (Some((xl, xr)), true) = (*row, sn.abs() > 1e-3 && d > 0.0) {
                let xe = if sn > 0.0 { xl - cx } else { xr - cx };
                let z = (xo - xe * c) / sn;
                let k = 1.0 - (2.0 * z / d).abs();
                if k >= 0.0 {
                    // darker than the face, a little lighter across its rounded middle
                    let shade = 0.5 + 0.25 * k.sqrt();
                    let cover = (k * d / 2.0).min(1.0);
                    out = [
                        base[0] * shade * cover,
                        base[1] * shade * cover,
                        base[2] * shade * cover,
                        255.0 * cover,
                    ];
                }
            }
            // the front (or, past a quarter turn, the plain back) over it
            if c.abs() > 1e-3 {
                let zf = if c > 0.0 { d / 2.0 } else { -d / 2.0 };
                let xf = (xo - zf * sn) / c + cx;
                let p = sample_clear(face, xf, y as f32 + 0.5);
                if p[3] > 0.0 {
                    let col = if c > 0.0 {
                        let lit = 0.7 + 0.3 * c;
                        [p[0] * lit, p[1] * lit, p[2] * lit]
                    } else {
                        let lit = (0.5 + 0.3 * -c) * p[3] / 255.0;
                        [base[0] * lit, base[1] * lit, base[2] * lit]
                    };
                    let k = 1.0 - p[3] / 255.0;
                    out = [
                        col[0] + out[0] * k,
                        col[1] + out[1] * k,
                        col[2] + out[2] * k,
                        p[3] + out[3] * k,
                    ];
                }
            }
            let i = ((y as i32 * ow + ox) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&unpremul(out));
        }
    }
    TurnFace {
        face: CartFace {
            rgba,
            w: ow as u32,
            h: face.h,
        },
        x: -pad,
        y: 0,
        dest_scale: 1,
    }
}

fn disc_turn(face: &CartFace, s: f32) -> TurnFace {
    let (w, h) = (face.w as i32, face.h as i32);
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    // twice round, clockwise, ending upright
    let (sn, c) = (-2.0 * std::f32::consts::TAU * s).sin_cos();
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let (sx, sy) = (c * dx + sn * dy + cx, -sn * dx + c * dy + cy);
            let i = ((y * w + x) * 4) as usize;
            rgba[i..i + 4].copy_from_slice(&unpremul(sample_clear(face, sx, sy)));
        }
    }
    TurnFace {
        face: CartFace {
            rgba,
            w: face.w,
            h: face.h,
        },
        x: 0,
        y: 0,
        dest_scale: 1,
    }
}

/// A Game & Watch opens (a clamshell) or lights its screen (the rest) in this many frames,
/// rendered for each game by brick/gw-art and kept beside its label as one strip,
/// `Labels/GW/open/<stem>.png`: the frames side by side, each the shelf face's size.
pub const GW_FRAMES: usize = 18;
/// How the strip was authored: 18 frames at 30 fps (0.6 s).
pub const GW_FPS: f32 = 30.0;
/// Played eased over this long, then held open, so a Game & Watch takes about as long as
/// every other shelf's cart turn (2.6-2.7 s) before it goes in.
pub const GW_OPEN_S: f32 = 2.13;
pub const GW_HOLD_S: f32 = 0.5;
pub const GW_OPEN_DIR: &str = "open";

/// The frames of a Game & Watch's strip, each placed over the shelf face; it goes in open
/// (the last frame). None when the strip is not whole frames of the face's size.
pub fn gw_faces(strip: &[u8]) -> Option<TurnFaces> {
    let shell = shell_png::of(Platform::Gw)?;
    let (w, h) = (shell.w, shell.h);
    let (rgba, sw, sh) = decode8(strip)?;
    if sh != h || sw < w || sw % w != 0 {
        return None;
    }
    let n = sw / w;
    let mut frames: Vec<TurnFace> = (0..n)
        .map(|i| {
            let mut px = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                let at = ((y * sw + i * w) * 4) as usize;
                px.extend_from_slice(&rgba[at..at + (w * 4) as usize]);
            }
            TurnFace {
                face: CartFace { rgba: px, w, h },
                x: 0,
                y: 0,
                dest_scale: 1,
            }
        })
        .collect();
    let seated = frames.last().map(|f| CartFace {
        rgba: f.face.rgba.clone(),
        w,
        h,
    });
    shrink_frames(&mut frames);
    Some(TurnFaces { frames, seated })
}

impl Turn {
    /// The frames at their own rate, then the still hold before the cart drops.
    pub fn seconds(&self) -> f32 {
        self.frames.len() as f32 / self.fps + self.hold
    }

    /// The frame shown `u` (0..1) of the way through the turn, hold included.
    pub fn frame_at(&self, u: f32) -> usize {
        let n = self.frames.len();
        let motion = n as f32 / self.fps;
        let v = (u.clamp(0.0, 1.0) * self.seconds() / motion).min(1.0);
        ((v * n as f32) as usize).min(n.saturating_sub(1))
    }
}

/// One frame, placed relative to the shelf face's top-left corner (in shelf pixels).
pub struct TurnFace {
    pub face: CartFace,
    pub x: i32,
    pub y: i32,
    /// Dest size is `face` size times this (2 when rgba was stored at half res).
    pub dest_scale: u32,
}

/// Every frame of `cart`'s turn, with `art` (the fitted label, as the shelf face uses) on it.
/// A cart that ends the turn the other way up (`flip`) also gets the face it goes in with.
pub struct TurnFaces {
    pub frames: Vec<TurnFace>,
    pub seated: Option<CartFace>,
}

pub fn faces(cart: &Cart, art: Option<Vec<u8>>) -> Option<TurnFaces> {
    // The first six systems turn their modelled cart (or disc) for the cart's own shape, its
    // plastic recoloured as the game's (model.rs); the rest their platform's rendered shell.
    let (turn, shell, plastic) = match crate::cart::model_dir(cart) {
        Some(dir) => (
            turn_by_dir(dir)?,
            shell_png::by_dir(dir)?,
            (cart.platform != Platform::Ps1).then(|| crate::shell::shell_for(cart)),
        ),
        None => (of(cart.platform)?, shell_png::of(cart.platform)?, None),
    };
    let (lw, lh) = (shell.lw, shell.lh);
    let label = match art {
        Some(px) if px.len() == (lw * lh * 4) as usize => px,
        // a disc with no art of its own turns with Slot's printed disc
        _ if cart.platform == Platform::Ps1 => crate::disc::disc_print(cart, None, None).rgba,
        _ => shell_png::plain_label_for(cart, lw, lh),
    };
    let mut frames = Vec::with_capacity(turn.frames.len());
    for f in turn.frames {
        let face = frame(f, &label, lw, lh, shell.backlit, plastic.as_ref())?;
        frames.push(TurnFace {
            face,
            x: f.x,
            y: f.y,
            dest_scale: 1,
        });
    }
    shrink_frames(&mut frames);
    let seated = if turn.flip {
        frames.last().map(|last| crop(last, shell.w, shell.h))
    } else {
        None
    };
    Some(TurnFaces { frames, seated })
}

fn frame(
    f: &TurnFrame,
    label: &[u8],
    lw: u32,
    lh: u32,
    backlit: bool,
    plastic: Option<&crate::shell::Shell>,
) -> Option<CartFace> {
    let (mut rgba, w, h) = decode8(f.shell)?;
    let (map, mw, mh) = decode8(f.map)?;
    if (w, h) != (f.w, f.h) || (mw, mh) != (w, h) {
        return None;
    }
    if let (Some(plastic), Some(bytes)) = (plastic, f.tint) {
        let (tint, tw, th) = decode8(bytes)?;
        if (tw, th) != (w, h) {
            return None;
        }
        crate::model::tint(
            &mut rgba,
            w,
            h,
            &tint,
            &crate::model::Plastic::Flat(plastic),
        );
    }
    let m = &f.uv;
    for y in 0..h {
        let fy = y as f32 + 0.5;
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let cover = map[i + 3] as u32;
            if cover == 0 {
                continue;
            }
            let fx = x as f32 + 0.5;
            let den = m[6] * fx + m[7] * fy + m[8];
            if den.abs() < 1e-8 {
                continue;
            }
            let u = ((m[0] * fx + m[1] * fy + m[2]) / den).clamp(0.0, 1.0);
            let v = ((m[3] * fx + m[4] * fy + m[5]) / den).clamp(0.0, 1.0);
            let px = shell_png::lit_px(
                sample(
                    label,
                    lw,
                    lh,
                    u * (lw - 1) as f32,
                    (1.0 - v) * (lh - 1) as f32,
                ),
                map[i],
                backlit,
            );
            let a = cover * px[3] as u32 / 255;
            let d = &mut rgba[i..i + 4];
            // The shell is in front: the label shows only where the shell leaves room.
            let room = 255 - d[3] as u32;
            let la = a * room / 255;
            let out_a = d[3] as u32 + la;
            if out_a == 0 {
                continue;
            }
            for c in 0..3 {
                d[c] = ((d[c] as u32 * d[3] as u32 + px[c] as u32 * la) / out_a) as u8;
            }
            d[3] = out_a.min(255) as u8;
        }
    }
    Some(CartFace { rgba, w, h })
}

fn sample(src: &[u8], w: u32, h: u32, x: f32, y: f32) -> [u8; 4] {
    let (x0, y0) = (x.floor().max(0.0) as u32, y.floor().max(0.0) as u32);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let at = |x: u32, y: u32, c: usize| src[((y * w + x) * 4) as usize + c] as f32;
    let mut out = [0u8; 4];
    for (c, o) in out.iter_mut().enumerate() {
        let top = at(x0, y0, c) * (1.0 - fx) + at(x1, y0, c) * fx;
        let bot = at(x0, y1, c) * (1.0 - fx) + at(x1, y1, c) * fx;
        *o = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// The last frame cut to the shelf face's box, for a cart that goes in turned over.
fn crop(last: &TurnFace, w: u32, h: u32) -> CartFace {
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    let f = &last.face;
    for y in 0..h as i32 {
        let sy = y - last.y;
        if sy < 0 || sy >= f.h as i32 {
            continue;
        }
        for x in 0..w as i32 {
            let sx = x - last.x;
            if sx < 0 || sx >= f.w as i32 {
                continue;
            }
            let s = ((sy as u32 * f.w + sx as u32) * 4) as usize;
            let d = ((y as u32 * w + x as u32) * 4) as usize;
            rgba[d..d + 4].copy_from_slice(&f.rgba[s..s + 4]);
        }
    }
    CartFace { rgba, w, h }
}


/// On the RG280V the 640×480 layout is shown at 1/2, and only one cart's frames
/// are resident. Storing those frames at half resolution (drawn at 2×) cuts the
/// decoded cache to about a quarter, which is the budget we want on 512 MB.
fn shrink_frames(frames: &mut [TurnFace]) {
    if !cfg!(feature = "rg280v") {
        return;
    }
    for f in frames {
        if f.dest_scale != 1 || f.face.w < 2 || f.face.h < 2 {
            continue;
        }
        let (nw, nh) = (f.face.w / 2, f.face.h / 2);
        if nw == 0 || nh == 0 {
            continue;
        }
        f.face.rgba = half_nearest(&f.face.rgba, f.face.w, f.face.h, nw, nh);
        f.face.w = nw;
        f.face.h = nh;
        f.dest_scale = 2;
    }
}

fn half_nearest(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for y in 0..dh {
        let sy = (y * sh / dh).min(sh - 1);
        for x in 0..dw {
            let sx = (x * sw / dw).min(sw - 1);
            let si = ((sy * sw + sx) * 4) as usize;
            let di = ((y * dw + x) * 4) as usize;
            out[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    out
}

pub(crate) fn decode8(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => px.to_vec(),
        png::ColorType::Rgb => px
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => px
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        _ => return None,
    };
    (rgba.len() == (w * h * 4) as usize).then_some((rgba, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_store::Platform;

    #[test]
    fn a_game_and_watch_strip_becomes_its_frames_and_goes_in_open() {
        let shell = shell_png::of(Platform::Gw).unwrap();
        let (w, h, n) = (shell.w, shell.h, 3u32);
        let mut strip = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut strip, w * n, h);
            enc.set_color(png::ColorType::Rgba);
            let mut px = vec![0u8; (w * n * h * 4) as usize];
            for (i, p) in px.chunks_exact_mut(4).enumerate() {
                let frame = (i as u32 % (w * n)) / w;
                p.copy_from_slice(&[frame as u8 * 100, 0, 0, 255]);
            }
            enc.write_header().unwrap().write_image_data(&px).unwrap();
        }
        let faces = gw_faces(&strip).unwrap();
        assert_eq!(faces.frames.len(), 3);
        for (i, f) in faces.frames.iter().enumerate() {
            assert_eq!((f.face.w, f.face.h, f.x, f.y), (w, h, 0, 0));
            assert!(
                f.face.rgba.chunks_exact(4).all(|p| p[0] == i as u8 * 100),
                "frame {i}"
            );
        }
        assert_eq!(
            faces.seated.unwrap().rgba[0],
            200,
            "it goes in as the last frame shows it"
        );
        assert!(
            gw_faces(&strip[..strip.len() / 2]).is_none(),
            "a broken strip"
        );
        assert!(turns(Platform::Gw));
        let gw = seconds(Platform::Gw);
        assert!(gw > 2.4 && gw < 3.0, "{gw}");
        assert_eq!(frame_index(Platform::Gw, 0.0, GW_FRAMES), 0);
        assert_eq!(frame_index(Platform::Gw, 1.0, GW_FRAMES), GW_FRAMES - 1);
        assert_eq!(
            frame_index(Platform::Gw, GW_OPEN_S / gw + 0.01, GW_FRAMES),
            GW_FRAMES - 1,
            "open before the hold"
        );
        let mut last = 0;
        for k in 0..=100 {
            let i = frame_index(Platform::Gw, k as f32 / 100.0, GW_FRAMES);
            assert!(i >= last, "never steps back");
            last = i;
        }
        assert!(ease(0.25) < 0.25 && ease(0.75) > 0.75 && ease(0.0) == 0.0 && ease(1.0) == 1.0);
    }

    #[test]
    fn only_the_new_shells_turn_on_the_way_in() {
        for p in Platform::ALL {
            let turns = of(p).is_some();
            let new = matches!(
                p.def().shell,
                Some(
                    "md" | "sms"
                        | "gg"
                        | "arcade"
                        | "pce"
                        | "a2600"
                        | "intv"
                        | "lynx"
                        | "a7800"
                        | "coleco"
                        | "a5200"
                        | "a800"
                        | "vectrex"
                        | "s32x"
                        | "n64"
                        | "nds"
                        | "psp"
                )
            );
            assert_eq!(turns, new, "{p:?}");
            // the rest turn too: the first six from their drawn face, Game & Watch its strip
            assert!(super::turns(p), "{p:?} goes straight in");
            assert_eq!(spins(p), !new && p != Platform::Gw, "{p:?}");
        }
    }

    #[test]
    fn a_drawn_cart_turns_once_round_and_ends_on_its_shelf_face() {
        for p in [Platform::Gba, Platform::Nes, Platform::Ps1] {
            let face = crate::cart::cart_face_with(
                &cart(p),
                Some(quarters(
                    crate::cart::label_size(&cart(p)).0,
                    crate::cart::label_size(&cart(p)).1,
                )),
            );
            let turn = spin_faces(p, &face);
            assert_eq!(turn.frames.len(), SPIN_FRAMES);
            assert!(turn.seated.is_none());
            // the last frame is the shelf face itself, square and down on the shelf
            let last = turn.frames.last().unwrap();
            assert_eq!(last.y, 0, "{p:?}");
            let (ox, w) = ((-last.x) as u32, face.w);
            let mut off = 0u32;
            for y in 0..face.h {
                for x in 0..w {
                    let a = &face.rgba[((y * w + x) * 4) as usize..][..4];
                    let b = &last.face.rgba[((y * last.face.w + x + ox) * 4) as usize..][..4];
                    // compared as they show (premultiplied): a clear pixel's colour is moot
                    let pm = |p: &[u8]| {
                        [
                            p[0] as u32 * p[3] as u32 / 255,
                            p[1] as u32 * p[3] as u32 / 255,
                            p[2] as u32 * p[3] as u32 / 255,
                            p[3] as u32,
                        ]
                    };
                    if pm(a).iter().zip(pm(b)).any(|(a, b)| a.abs_diff(b) > 3) {
                        off += 1;
                    }
                }
            }
            assert!(
                off < face.w * face.h / 200,
                "{p:?}: {off} pixels off the shelf face"
            );
            // half way round it is the plain back (or, a disc, still the disc), up off the shelf
            let mid = &turn.frames[SPIN_FRAMES / 2];
            assert!(mid.y < 0, "{p:?} does not lift");
            assert!((seconds(p) - (SPIN_FRAMES as f32 / SPIN_FPS + SPIN_HOLD_S)).abs() < 1e-4);
        }
    }

    fn cart(platform: Platform) -> Cart {
        Cart {
            platform,
            stem: "Turn".into(),
            rom: "Turn.bin".into(),
            label: None,
            famicom_label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
        }
    }

    /// A label in four colours: red top-left, green top-right, blue bottom-left, white
    /// bottom-right.
    fn quarters(w: u32, h: u32) -> Vec<u8> {
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                px.extend_from_slice(match (x * 2 < w, y * 2 < h) {
                    (true, true) => &[255, 0, 0, 255],
                    (false, true) => &[0, 255, 0, 255],
                    (true, false) => &[0, 0, 255, 255],
                    (false, false) => &[255, 255, 255, 255],
                });
            }
        }
        px
    }

    /// Mean (x, y) of the pixels clearly of one colour.
    fn centre(face: &CartFace, want: fn(&[u8]) -> bool) -> Option<(f32, f32)> {
        let (mut sx, mut sy, mut n) = (0.0, 0.0, 0.0);
        for (i, p) in face.rgba.chunks_exact(4).enumerate() {
            if p[3] > 200 && want(p) {
                sx += (i as u32 % face.w) as f32;
                sy += (i as u32 / face.w) as f32;
                n += 1.0;
            }
        }
        (n > 20.0).then(|| (sx / n, sy / n))
    }

    fn red(p: &[u8]) -> bool {
        p[0] > 120 && p[1] < 60 && p[2] < 60
    }
    fn green(p: &[u8]) -> bool {
        p[1] > 120 && p[0] < 60 && p[2] < 60
    }
    fn blue(p: &[u8]) -> bool {
        p[2] > 120 && p[0] < 60 && p[1] < 60
    }

    #[test]
    fn the_label_reads_upright_at_the_start_and_end_of_every_turn() {
        for p in Platform::ALL {
            let Some(shell) = shell_png::of(p) else {
                continue;
            };
            if of(p).is_none() {
                continue;
            }
            let art = quarters(shell.lw, shell.lh);
            let turned = faces(&cart(p), Some(art.clone())).expect("frames");
            let first = &turned.frames[0].face;
            // The Vectrex starts switched off: its screen (the label) is dim until it lights.
            let wake = |f: &CartFace| CartFace {
                rgba: f
                    .rgba
                    .chunks_exact(4)
                    .flat_map(|q| {
                        let k = if p == Platform::Vectrex { 3 } else { 1 };
                        [
                            q[0].saturating_mul(k),
                            q[1].saturating_mul(k),
                            q[2].saturating_mul(k),
                            q[3],
                        ]
                    })
                    .collect(),
                w: f.w,
                h: f.h,
            };
            let woken = wake(first);
            let first = &woken;
            let last = turned
                .seated
                .as_ref()
                .unwrap_or(&turned.frames.last().unwrap().face);
            for (when, face) in [("first", first), ("last", last)] {
                let (r, g, b) = (centre(face, red), centre(face, green), centre(face, blue));
                let (Some(r), Some(g), Some(b)) = (r, g, b) else {
                    panic!("{p:?} {when}: label colours not all visible");
                };
                assert!(r.0 < g.0, "{p:?} {when}: red is not left of green");
                assert!(r.1 < b.1, "{p:?} {when}: red is not above blue");
            }
            // The first frame stands where the shelf face does.
            let shelf = wake(&shell_png::cart_face(&cart(p), Some(art)).expect("face"));
            let (a, b) = (centre(&shelf, red).unwrap(), centre(first, red).unwrap());
            let f = &turned.frames[0];
            let (bx, by) = (b.0 + f.x as f32, b.1 + f.y as f32);
            assert!(
                (a.0 - bx).abs() < 6.0 && (a.1 - by).abs() < 6.0,
                "{p:?}: shelf {a:?} vs turn {bx},{by}"
            );
        }
    }

    #[test]
    fn every_turn_frame_decodes_at_its_listed_size() {
        for p in Platform::ALL {
            let Some(turn) = of(p) else { continue };
            // Every shelf's cart takes about as long to turn, the UMD's spin and two turns
            // included: 2.4-3.0 s, hold and all.
            assert!(
                turn.seconds() > 2.4 && turn.seconds() < 3.0,
                "{p:?} turn length {}",
                turn.seconds()
            );
            // Whatever the hold, the last frame shows at the end and the first at the start.
            assert_eq!(turn.frame_at(0.0), 0, "{p:?}");
            assert_eq!(turn.frame_at(1.0), turn.frames.len() - 1, "{p:?}");
            assert_eq!(
                frame_index(p, 1.0, turn.frames.len()),
                turn.frames.len() - 1
            );
            if turn.hold > 0.0 {
                let rest = 1.0 - turn.hold / turn.seconds();
                assert_eq!(
                    turn.frame_at(rest + 0.01),
                    turn.frames.len() - 1,
                    "{p:?} is still turning in its hold"
                );
            }
            for (i, f) in turn.frames.iter().enumerate() {
                let (_, w, h) = decode8(f.shell).unwrap_or_else(|| panic!("{p:?} frame {i}"));
                assert_eq!((w, h), (f.w, f.h), "{p:?} frame {i}");
                let (_, mw, mh) = decode8(f.map).unwrap_or_else(|| panic!("{p:?} map {i}"));
                assert_eq!((mw, mh), (w, h), "{p:?} map {i}");
            }
        }
    }

    /// Face-on HuCard pins are a dull gold, about (110, 90, 60).
    fn gold_rows(rgba: &[u8], w: u32, h: u32) -> (u32, u32, u32) {
        let mut top = 0u32;
        let mut bot = 0u32;
        let mut mid = 0u32;
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            if p[3] < 200 {
                continue;
            }
            let (r, g, b) = (p[0] as i16, p[1] as i16, p[2] as i16);
            if !(90..=160).contains(&r)
                || !(70..=130).contains(&g)
                || b >= 90
                || r <= g + 8
                || g <= b + 15
            {
                continue;
            }
            let y = i as u32 / w;
            if y * 3 < h {
                top += 1;
            } else if y * 3 > h * 2 {
                bot += 1;
            } else {
                mid += 1;
            }
        }
        (top, mid, bot)
    }

    #[test]
    fn a_hucard_goes_in_connector_first_with_the_pins_facing_away() {
        let turn = of(Platform::Pce).expect("HuCard turn");
        assert!(turn.flip, "the last frame is the face it goes in with");
        let (first, fw, fh) = decode8(turn.frames[0].shell).expect("first shell");
        let (last, lw, lh) = decode8(turn.frames.last().unwrap().shell).expect("last shell");
        let (ft, _, fb) = gold_rows(&first, fw, fh);
        assert!(
            ft > 20 && ft > fb * 2,
            "on the shelf the HuCard's contacts should be at the top, got top={ft} bot={fb}"
        );
        let (lt, lm, lb) = gold_rows(&last, lw, lh);
        let gold = lt + lm + lb;
        assert!(
            gold < 8,
            "the last frame still shows face-on gold pins (they would point out of the slot): top={lt} mid={lm} bot={lb}"
        );
        assert!(
            faces(&cart(Platform::Pce), None)
                .expect("faces")
                .seated
                .is_some(),
            "flip seats the last frame for the slide"
        );
    }
}
