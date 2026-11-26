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
        return GW_SECONDS;
    }
    of(platform).map_or(0.0, Turn::seconds)
}

/// Whether a platform's carts turn (or, a Game & Watch, open) before they go in.
pub fn turns(platform: Platform) -> bool {
    platform == Platform::Gw || of(platform).is_some()
}

/// A Game & Watch opens (a clamshell) or lights its screen (the rest) in this many frames,
/// rendered for each game by brick/gw-art and kept beside its label as one strip,
/// `Labels/GW/open/<stem>.png`: the frames side by side, each the shelf face's size.
pub const GW_FRAMES: usize = 18;
/// How the strip was authored: 18 frames at 30 fps, 0.6 s.
pub const GW_STRIP_FPS: f32 = 30.0;
/// Playback is about 1.6× that original, just under a second, so it can be savored.
pub const GW_SECONDS: f32 = GW_FRAMES as f32 / GW_STRIP_FPS * 1.6;
pub const GW_OPEN_DIR: &str = "open";

/// Ease-in-out for the open or light-up: slow at both ends, a little quicker in the middle.
pub fn ease_gw(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

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
    let frames: Vec<TurnFace> = (0..n)
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
            }
        })
        .collect();
    let seated = frames.last().map(|f| CartFace {
        rgba: f.face.rgba.clone(),
        w,
        h,
    });
    Some(TurnFaces { frames, seated })
}

impl Turn {
    pub fn seconds(&self) -> f32 {
        self.frames.len() as f32 / self.fps
    }

    /// The frame shown `u` (0..1) of the way through the turn.
    pub fn frame_at(&self, u: f32) -> usize {
        let n = self.frames.len();
        ((u.clamp(0.0, 1.0) * n as f32) as usize).min(n.saturating_sub(1))
    }
}

/// One frame, placed relative to the shelf face's top-left corner (in shelf pixels).
pub struct TurnFace {
    pub face: CartFace,
    pub x: i32,
    pub y: i32,
}

/// Every frame of `cart`'s turn, with `art` (the fitted label, as the shelf face uses) on it.
/// A cart that ends the turn the other way up (`flip`) also gets the face it goes in with.
pub struct TurnFaces {
    pub frames: Vec<TurnFace>,
    pub seated: Option<CartFace>,
}

pub fn faces(cart: &Cart, art: Option<Vec<u8>>) -> Option<TurnFaces> {
    let turn = of(cart.platform)?;
    let shell = shell_png::of(cart.platform)?;
    let (lw, lh) = (shell.lw, shell.lh);
    let label = match art {
        Some(px) if px.len() == (lw * lh * 4) as usize => px,
        _ => shell_png::plain_label_for(cart, lw, lh),
    };
    let mut frames = Vec::with_capacity(turn.frames.len());
    for f in turn.frames {
        let face = frame(f, &label, lw, lh, shell.backlit)?;
        frames.push(TurnFace {
            face,
            x: f.x,
            y: f.y,
        });
    }
    let seated = if turn.flip {
        frames.last().map(|last| crop(last, shell.w, shell.h))
    } else {
        None
    };
    Some(TurnFaces { frames, seated })
}

fn frame(f: &TurnFrame, label: &[u8], lw: u32, lh: u32, backlit: bool) -> Option<CartFace> {
    let (mut rgba, w, h) = decode8(f.shell)?;
    let (map, mw, mh) = decode8(f.map)?;
    if (w, h) != (f.w, f.h) || (mw, mh) != (w, h) {
        return None;
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

fn decode8(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
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
        let secs = seconds(Platform::Gw);
        assert!((secs - GW_SECONDS).abs() < 1e-6);
        // 1.6× the authored 0.6 s, in the 0.95–1.0 s range.
        assert!((secs - 0.96).abs() < 1e-6, "{secs}");
        assert_eq!(ease_gw(0.0), 0.0);
        assert_eq!(ease_gw(1.0), 1.0);
        assert!((ease_gw(0.5) - 0.5).abs() < 1e-6);
        assert!(ease_gw(0.25) < 0.25, "starts slowly");
        assert!(ease_gw(0.75) > 0.75, "settles slowly");
    }

    #[test]
    fn only_the_new_shells_turn_on_the_way_in() {
        for p in Platform::ALL {
            let turns = of(p).is_some();
            let new = matches!(
                p.def().shell,
                Some("md" | "sms" | "gg" | "arcade" | "pce" | "a2600" | "intv" | "lynx" | "a7800")
            );
            assert_eq!(turns, new, "{p:?}");
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
            let shelf = shell_png::cart_face(&cart(p), Some(art)).expect("face");
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
            assert!(
                turn.seconds() > 0.3 && turn.seconds() < 1.0,
                "{p:?} turn length"
            );
            for (i, f) in turn.frames.iter().enumerate() {
                let (_, w, h) = decode8(f.shell).unwrap_or_else(|| panic!("{p:?} frame {i}"));
                assert_eq!((w, h), (f.w, f.h), "{p:?} frame {i}");
                let (_, mw, mh) = decode8(f.map).unwrap_or_else(|| panic!("{p:?} map {i}"));
                assert_eq!((mw, mh), (w, h), "{p:?} map {i}");
            }
        }
    }
}
