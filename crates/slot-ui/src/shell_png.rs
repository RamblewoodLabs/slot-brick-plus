//! A cartridge drawn from a pre-rendered shell. The label from `Labels/<sys>/<stem>.png`
//! is scaled into the rectangle in `meta.toml`, rotated if asked, (when `mask.png`
//! is there) clipped to that mask, and lit: multiplied by `shade.png` (the light a paper
//! label catches on that cart), or brightened when the shell is a `backlit` marquee.
//! See `docs/cart-shells.md`.
use slot_store::{Cart, Platform};

use crate::art;
use crate::cart::{label_colour, label_text, CartFace};
use crate::text;

include!(concat!(env!("OUT_DIR"), "/shells.rs"));

pub fn of(platform: Platform) -> Option<&'static ShellPng> {
    by_dir(platform.def().shell?)
}

pub fn cart_face(cart: &Cart, art: Option<Vec<u8>>) -> Option<CartFace> {
    let shell = of(cart.platform)?;
    let (mut rgba, w, h) = decode(shell.shell)?;
    if rgba.len() != (w * h * 4) as usize || w != shell.w || h != shell.h {
        return None;
    }
    let label = match art {
        Some(pixels) if pixels.len() == (shell.lw * shell.lh * 4) as usize => pixels,
        Some(_) => plain_label(shell.lw, shell.lh, &label_text(cart)),
        None => plain_label(shell.lw, shell.lh, &label_text(cart)),
    };
    let label = rotate_label(&label, shell.lw, shell.lh, shell.rotation);
    let mask = shell.mask.and_then(|bytes| decode(bytes));
    let shade = shell.shade.and_then(|bytes| decode(bytes));
    let light = Light {
        shade: shade.as_ref(),
        backlit: shell.backlit,
    };
    if let Some(quad) = shell.quad {
        paste_quad(
            &mut rgba,
            w,
            h,
            &label,
            shell.lw,
            shell.lh,
            quad,
            mask.as_ref(),
            light,
        );
    } else {
        paste_rect(&mut rgba, w, h, &label, shell, mask.as_ref(), light);
    }
    Some(CartFace { rgba, w, h })
}

/// The shell with the label window clear, so a picture behind it shows through.
pub fn cutout(cart: &Cart) -> Option<(CartFace, (u32, u32, u32, u32))> {
    let shell = of(cart.platform)?;
    let mut face = cart_face(cart, Some(vec![0; (shell.lw * shell.lh * 4) as usize]))?;
    let (x, y, lw, lh) = (shell.x, shell.y, shell.lw, shell.lh);
    for py in y..(y + lh).min(face.h) {
        for px in x..(x + lw).min(face.w) {
            face.rgba[((py * face.w + px) * 4 + 3) as usize] = 0;
        }
    }
    Some((face, (x, y, lw, lh)))
}

pub fn shadow(platform: Platform) -> Option<CartFace> {
    let shell = of(platform)?;
    let (rgba, w, h) = decode(shell.shell)?;
    let mut out = vec![0u8; rgba.len()];
    for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        dst[3] = src[3];
    }
    Some(CartFace { rgba: out, w, h })
}

/// The label a cart with no art gets: its title on its own colour.
pub(crate) fn plain_label_for(cart: &Cart, w: u32, h: u32) -> Vec<u8> {
    plain_label(w, h, &label_text(cart))
}

/// One label pixel lit as `lit` does (for the turn frames).
pub(crate) fn lit_px(px: [u8; 4], shade: u8, backlit: bool) -> [u8; 4] {
    lit(px, shade, backlit)
}

fn plain_label(w: u32, h: u32, title: &str) -> Vec<u8> {
    let bg = label_colour(title);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..w * h {
        rgba.extend_from_slice(&[bg[0], bg[1], bg[2], 255]);
    }
    if let Some(font) = text::label_font() {
        let layout = text::fit_box(
            font,
            title,
            (w.saturating_sub(16)) as f32,
            h as f32,
            3,
            h as f32 / 4.0,
            10.0,
        );
        text::draw_centred(&mut rgba, w, h, &layout, [0x14, 0x12, 0x10]);
    }
    rgba
}

fn rotate_label(src: &[u8], w: u32, h: u32, degrees: f32) -> Vec<u8> {
    if degrees.abs() < 0.01 || w == 0 || h == 0 {
        return src.to_vec();
    }
    let rad = degrees.to_radians();
    let (sin, cos) = (rad.sin(), rad.cos());
    let mut out = vec![0u8; src.len()];
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    for y in 0..h {
        for x in 0..w {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let sx = cos * dx + sin * dy + cx;
            let sy = -sin * dx + cos * dy + cy;
            let (ix, iy) = (sx.floor() as i32, sy.floor() as i32);
            if ix < 0 || iy < 0 || ix >= w as i32 || iy >= h as i32 {
                continue;
            }
            let s = ((iy as u32 * w + ix as u32) * 4) as usize;
            let d = ((y * w + x) * 4) as usize;
            out[d..d + 4].copy_from_slice(&src[s..s + 4]);
        }
    }
    out
}

fn paste_rect(
    dst: &mut [u8],
    dw: u32,
    dh: u32,
    label: &[u8],
    shell: &ShellPng,
    mask: Option<&(Vec<u8>, u32, u32)>,
    light: Light,
) {
    for y in 0..shell.lh {
        for x in 0..shell.lw {
            let px = shell.x + x;
            let py = shell.y + y;
            if px >= dw || py >= dh {
                continue;
            }
            let cover = mask_at(mask, px, py);
            if let Some(src) = label_px(label, shell.lw, x, y) {
                blit(dst, dw, px, py, light.on(src, px, py), cover);
            }
        }
    }
}

fn paste_quad(
    dst: &mut [u8],
    dw: u32,
    dh: u32,
    label: &[u8],
    lw: u32,
    lh: u32,
    quad: [[f32; 2]; 4],
    mask: Option<&(Vec<u8>, u32, u32)>,
    light: Light,
) {
    let (min_x, min_y, max_x, max_y) = bounds(quad, dw, dh);
    for py in min_y..max_y {
        for px in min_x..max_x {
            let Some((u, v)) = invert_quad(quad, px as f32 + 0.5, py as f32 + 0.5) else {
                continue;
            };
            if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
                continue;
            }
            let x = (u * lw as f32).clamp(0.0, lw.saturating_sub(1) as f32) as u32;
            let y = (v * lh as f32).clamp(0.0, lh.saturating_sub(1) as f32) as u32;
            let cover = mask_at(mask, px, py);
            if let Some(src) = label_px(label, lw, x, y) {
                blit(dst, dw, px, py, light.on(src, px, py), cover);
            }
        }
    }
}

fn bounds(quad: [[f32; 2]; 4], dw: u32, dh: u32) -> (u32, u32, u32, u32) {
    let min_x = quad
        .iter()
        .map(|p| p[0])
        .fold(f32::MAX, f32::min)
        .floor()
        .max(0.0) as u32;
    let min_y = quad
        .iter()
        .map(|p| p[1])
        .fold(f32::MAX, f32::min)
        .floor()
        .max(0.0) as u32;
    let max_x = quad
        .iter()
        .map(|p| p[0])
        .fold(0.0, f32::max)
        .ceil()
        .min(dw as f32) as u32;
    let max_y = quad
        .iter()
        .map(|p| p[1])
        .fold(0.0, f32::max)
        .ceil()
        .min(dh as f32) as u32;
    (min_x, min_y, max_x, max_y)
}

/// Bilinear inverse: UV in 0..1 for a point inside the quad (TL, TR, BR, BL), else None.
fn invert_quad(q: [[f32; 2]; 4], x: f32, y: f32) -> Option<(f32, f32)> {
    let (tl, tr, br, bl) = (q[0], q[1], q[2], q[3]);
    let mut u = 0.5f32;
    let mut v = 0.5f32;
    for _ in 0..8 {
        let (px, py) = bilinear(tl, tr, br, bl, u, v);
        let (dx, dy) = (x - px, y - py);
        if dx * dx + dy * dy < 0.05 {
            return Some((u, v));
        }
        let (pxu, pyu) = bilinear(tl, tr, br, bl, (u + 0.01).min(1.0), v);
        let (pxv, pyv) = bilinear(tl, tr, br, bl, u, (v + 0.01).min(1.0));
        let (jx, jy) = ((pxu - px) / 0.01, (pyu - py) / 0.01);
        let (kx, ky) = ((pxv - px) / 0.01, (pyv - py) / 0.01);
        let det = jx * ky - jy * kx;
        if det.abs() < 1e-4 {
            return None;
        }
        u = (u + (ky * dx - kx * dy) / det).clamp(0.0, 1.0);
        v = (v + (jx * dy - jy * dx) / det).clamp(0.0, 1.0);
    }
    let (px, py) = bilinear(tl, tr, br, bl, u, v);
    ((x - px).abs() < 1.5 && (y - py).abs() < 1.5).then_some((u, v))
}

fn bilinear(tl: [f32; 2], tr: [f32; 2], br: [f32; 2], bl: [f32; 2], u: f32, v: f32) -> (f32, f32) {
    let top = (tl[0] + (tr[0] - tl[0]) * u, tl[1] + (tr[1] - tl[1]) * u);
    let bot = (bl[0] + (br[0] - bl[0]) * u, bl[1] + (br[1] - bl[1]) * u);
    (top.0 + (bot.0 - top.0) * v, top.1 + (bot.1 - top.1) * v)
}

fn mask_at(mask: Option<&(Vec<u8>, u32, u32)>, x: u32, y: u32) -> u32 {
    let Some((rgba, w, _)) = mask else { return 255 };
    let i = ((y * w + x) * 4 + 3) as usize;
    rgba.get(i).copied().unwrap_or(0) as u32
}

/// How the label is lit on the shell: by the light a paper label catches there (`shade.png`,
/// white where the paper faces the light), or from behind (`backlit`, a marquee).
#[derive(Clone, Copy)]
struct Light<'a> {
    shade: Option<&'a (Vec<u8>, u32, u32)>,
    backlit: bool,
}

impl Light<'_> {
    fn on(self, px: [u8; 4], x: u32, y: u32) -> [u8; 4] {
        let shade = match self.shade {
            Some((rgba, w, _)) => rgba.get(((y * w + x) * 4) as usize).copied().unwrap_or(255),
            None => 255,
        };
        lit(px, shade, self.backlit)
    }
}

/// One label pixel under the shell's light. `shade` is 0..255 (255 leaves it as printed).
/// A backlit marquee ignores the shade: the light comes through it, lifting every colour.
fn lit(px: [u8; 4], shade: u8, backlit: bool) -> [u8; 4] {
    let mut out = px;
    for c in 0..3 {
        out[c] = if backlit {
            (px[c] as u32 * 110 / 100 + 12).min(255) as u8
        } else {
            ((px[c] as u32 * shade as u32 + 127) / 255) as u8
        };
    }
    out
}

fn label_px(src: &[u8], sw: u32, sx: u32, sy: u32) -> Option<[u8; 4]> {
    let s = ((sy * sw + sx) * 4) as usize;
    src.get(s..s + 4).map(|p| [p[0], p[1], p[2], p[3]])
}

fn blit(dst: &mut [u8], dw: u32, dx: u32, dy: u32, src: [u8; 4], cover: u32) {
    let a = src[3] as u32 * cover / 255;
    if a == 0 {
        return;
    }
    let d = ((dy * dw + dx) * 4) as usize;
    for c in 0..3 {
        dst[d + c] = ((src[c] as u32 * a + dst[d + c] as u32 * (255 - a) + 127) / 255) as u8;
    }
    dst[d + 3] = dst[d + 3].max(a as u8);
}

fn decode(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let dec = png::Decoder::new(bytes);
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let pixels = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for px in pixels.chunks_exact(3) {
                out.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
            out
        }
        _ => return None,
    };
    (rgba.len() == (w * h * 4) as usize).then_some((rgba, w, h))
}

pub fn fitted(cart: &Cart, bytes: &[u8]) -> Option<Vec<u8>> {
    let shell = of(cart.platform)?;
    let (src, sw, sh) = decode(bytes)?;
    art::cover_rgba(&src, sw, sh, shell.lw, shell.lh)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cart(platform: Platform) -> Cart {
        Cart {
            platform,
            stem: "Sonic".into(),
            rom: "Sonic.md".into(),
            label: None,
            famicom_label: None,
            title: String::new(),
            code: String::new(),
            shell: None,
        }
    }

    #[test]
    fn no_shell_stands_taller_than_the_nes_cart_on_the_shelf_or_seated() {
        let nes_seated = crate::cart::seated_box(Platform::Nes).1;
        for platform in Platform::ALL {
            let Some(shell) = of(platform) else { continue };
            assert!(shell.h <= crate::cart::NES_CART_H, "{platform:?} is {} tall", shell.h);
            let (sw, sh) = crate::cart::seated_box(platform);
            assert!(sh <= nes_seated && sw <= crate::cart::SEATED_W, "{platform:?} seats {sw}x{sh}");
        }
    }

    #[test]
    fn every_shell_asset_is_inside_its_png_and_keeps_its_aspect() {
        for platform in Platform::ALL {
            let Some(shell) = of(platform) else {
                assert!(
                    platform.def().shell.is_none(),
                    "{platform:?} names a shell that was not built"
                );
                continue;
            };
            assert!(
                shell.x + shell.lw <= shell.w && shell.y + shell.lh <= shell.h,
                "{platform:?}"
            );
            let aspect = shell.lw as f32 / shell.lh as f32;
            assert!(
                (aspect - shell.aspect).abs() < 0.02,
                "{platform:?} aspect {aspect} vs {}",
                shell.aspect
            );
            let face = cart_face(&cart(platform), None).unwrap();
            assert_eq!((face.w, face.h), (shell.w, shell.h));
        }
    }

    #[test]
    fn a_label_lands_in_the_window_and_a_mask_keeps_the_recess_corners() {
        let platform = Platform::Md;
        let shell = of(platform).unwrap();
        let mut red = vec![0u8; (shell.lw * shell.lh * 4) as usize];
        for px in red.chunks_exact_mut(4) {
            px.copy_from_slice(&[220, 20, 20, 255]);
        }
        let face = cart_face(&cart(platform), Some(red)).unwrap();
        let at = |x, y| {
            let i = ((y * face.w + x) * 4) as usize;
            [face.rgba[i], face.rgba[i + 1], face.rgba[i + 2]]
        };
        let mid = at(shell.x + shell.lw / 2, shell.y + shell.lh / 2);
        // Red, under the cart's light: darker than printed where the paper turns away.
        assert!(mid[0] > 150 && mid[1] < 80, "label centre {mid:?}");
        let outside = at(4, 4);
        assert!(outside[0] < 80, "shell corner took the label {outside:?}");

        // Wherever a mask is clear inside the window (rounded label corners, the marquee's
        // retainers), the shell is left exactly as rendered.
        let mut hidden = 0;
        for platform in Platform::ALL {
            let Some(shell) = of(platform) else { continue };
            let Some(mask) = shell.mask else { continue };
            let (bare, _, _) = decode(shell.shell).unwrap();
            let (mask, mw, _) = decode(mask).unwrap();
            let mut gold = vec![0u8; (shell.lw * shell.lh * 4) as usize];
            for px in gold.chunks_exact_mut(4) {
                px.copy_from_slice(&[240, 200, 40, 255]);
            }
            let face = cart_face(&cart(platform), Some(gold)).unwrap();
            for y in shell.y..shell.y + shell.lh {
                for x in shell.x..shell.x + shell.lw {
                    let i = ((y * mw + x) * 4) as usize;
                    if mask[i + 3] == 0 {
                        hidden += 1;
                        assert_eq!(
                            &face.rgba[i..i + 4],
                            &bare[i..i + 4],
                            "{platform:?}: masked pixel {x},{y} took the label"
                        );
                    }
                }
            }
        }
        assert!(hidden > 0, "no mask clears any corner of its window");
    }

    #[test]
    fn shade_multiplies_the_label_and_a_backlit_marquee_glows() {
        assert_eq!(lit([200, 100, 50, 255], 255, false), [200, 100, 50, 255]);
        assert_eq!(lit([200, 100, 50, 255], 128, false), [100, 50, 25, 255]);
        assert_eq!(lit([200, 100, 50, 77], 0, false), [0, 0, 0, 77]);
        let glow = lit([100, 100, 250, 255], 0, true);
        assert!(glow[0] > 100 && glow[2] == 255, "backlit ignores the shade and lifts: {glow:?}");
    }

    #[test]
    fn every_shade_and_mask_matches_its_shell_and_the_marquee_is_backlit() {
        for platform in Platform::ALL {
            let Some(shell) = of(platform) else { continue };
            for (what, png) in [("mask", shell.mask), ("shade", shell.shade)] {
                if let Some(bytes) = png {
                    let (_, w, h) = decode(bytes).unwrap_or_else(|| panic!("{platform:?} {what}.png"));
                    assert_eq!((w, h), (shell.w, shell.h), "{platform:?} {what}.png size");
                }
            }
            assert_eq!(shell.backlit, platform == Platform::Arcade, "{platform:?}");
        }
    }

    #[test]
    fn a_white_label_takes_the_shade_and_a_marquee_lights_up() {
        let md = of(Platform::Md).unwrap();
        let (shade, sw, _) = decode(md.shade.expect("md ships a shade")).unwrap();
        let white = vec![255u8; (md.lw * md.lh * 4) as usize];
        let face = cart_face(&cart(Platform::Md), Some(white)).unwrap();
        let (x, y) = (md.x + md.lw / 2, md.y + md.lh / 2);
        let got = face.rgba[((y * face.w + x) * 4) as usize];
        let want = shade[((y * sw + x) * 4) as usize];
        assert!(got.abs_diff(want) <= 2, "white label at the centre {got}, shade {want}");

        let arcade = of(Platform::Arcade).unwrap();
        let grey = vec![128u8; (arcade.lw * arcade.lh * 4) as usize]
            .chunks_exact(4)
            .flat_map(|_| [128u8, 128, 128, 255])
            .collect::<Vec<_>>();
        let face = cart_face(&cart(Platform::Arcade), Some(grey)).unwrap();
        let (x, y) = (arcade.x + arcade.lw / 2, arcade.y + arcade.lh / 2);
        let lit_centre = face.rgba[((y * face.w + x) * 4) as usize];
        assert!(lit_centre > 140, "the marquee should glow, centre {lit_centre}");
    }

    #[test]
    fn rotation_turns_a_split_label() {
        let shell = of(Platform::Gg).unwrap();
        let mut label = vec![0u8; (shell.lw * shell.lh * 4) as usize];
        for y in 0..shell.lh {
            for x in 0..shell.lw {
                let i = ((y * shell.lw + x) * 4) as usize;
                let c = if x < shell.lw / 2 { 200 } else { 20 };
                label[i] = c;
                label[i + 3] = 255;
            }
        }
        let turned = rotate_label(&label, shell.lw, shell.lh, 90.0);
        let left = turned[(shell.lh / 2 * shell.lw) as usize * 4];
        let top = turned[(shell.lw / 2 * 4) as usize];
        assert_ne!(left, top, "a quarter turn should move the red half");
    }
}
