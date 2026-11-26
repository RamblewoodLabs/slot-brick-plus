//! The first six systems' carts (and the PlayStation disc) drawn from their Blender models
//! (`assets/carts/model-*`, one per cart shape) instead of flat outlines. Each render is framed
//! to the box `cart` draws that cart in, with its label window on the procedural label panel,
//! so label fitting, layout and shadows are as before. The plastic is rendered neutral grey and
//! recoloured here for each game (`shell::shell_for`): `tint.png` holds where the plastic is and
//! how lit it is (128 = a face square to the light). See `docs/cart-shells.md`.
use slot_store::Cart;

use crate::cart::{self, CartFace};
use crate::shell::{Finish, Shell};
use crate::shell_png::{self, ShellPng};

/// The model a cart is drawn from, when its shape has one.
pub fn of(cart: &Cart) -> Option<&'static ShellPng> {
    shell_png::by_dir(cart::model_dir(cart)?)
}

/// Light a disc's printed face (`DISC_PX` square) as the model's disc is lit, and lay the
/// model's clear hub, stacking ring and rim over it.
pub(crate) fn light_disc(face: &mut CartFace) {
    let Some(model) = shell_png::by_dir("model-ps1") else {
        return;
    };
    let (Some((shell, w, h)), Some((shade, _, _)), Some((mask, _, _))) = (
        crate::turn::decode8(model.shell),
        model.shade.and_then(crate::turn::decode8),
        model.mask.and_then(crate::turn::decode8),
    ) else {
        return;
    };
    if (w, h) != (face.w, face.h) || shade.len() != shell.len() || mask.len() != shell.len() {
        return;
    }
    for (i, px) in face.rgba.chunks_exact_mut(4).enumerate() {
        let o = i * 4;
        if mask[o + 3] > 0 {
            for c in 0..3 {
                px[c] = ((px[c] as u32 * shade[o] as u32 + 127) / 255) as u8;
            }
        }
        let a = shell[o + 3] as u32;
        if a > 0 && px[3] > 0 {
            for c in 0..3 {
                px[c] = ((shell[o + c] as u32 * a + px[c] as u32 * (255 - a) + 127) / 255) as u8;
            }
        }
    }
}

/// The colour the grey plastic takes.
pub(crate) enum Plastic<'a> {
    /// One plastic all over (a turning cart).
    Flat(&'a Shell),
    /// Pixel for pixel the flat-drawn shell's own colours (RGBA, the face's size): a clear
    /// pak's board showing through, a gold cart's sheen, as `cart` paints them, now lit.
    Face(&'a [u8]),
}

/// Recolour the grey plastic of a decoded shell (`rgba`, `w` x `h`) as `plastic`, from its
/// decoded tint map (grey = light, alpha = cover; same size).
pub(crate) fn tint(rgba: &mut [u8], w: u32, h: u32, map: &[u8], plastic: &Plastic) {
    if map.len() != rgba.len() {
        return;
    }
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let cover = map[i + 3] as u32;
            if cover == 0 {
                continue;
            }
            let c = match plastic {
                Plastic::Flat(shell) => plastic_px(shell, map[i], x, y, h),
                Plastic::Face(face) if face.len() == rgba.len() => {
                    light_px([face[i], face[i + 1], face[i + 2]], map[i])
                }
                Plastic::Face(_) => continue,
            };
            // a pixel's cover is never more than its own alpha: the rest of it is not plastic
            let a = rgba[i + 3].max(1) as u32;
            let t = (cover * 255 / a).min(255);
            for k in 0..3 {
                rgba[i + k] =
                    ((c[k] as u32 * t + rgba[i + k] as u32 * (255 - t) + 127) / 255) as u8;
            }
        }
    }
}

/// One pixel of `plastic` lit by `light` (128 = as lit as a flat face; less is shadow, more
/// a highlight that lifts it towards white).
pub(crate) fn plastic_px(plastic: &Shell, light: u8, x: u32, y: u32, h: u32) -> [u8; 3] {
    let base = match plastic.finish {
        Finish::Solid => plastic.colour,
        Finish::Sheen => cart::chrome_px(plastic.colour, y, h),
        // clear plastic: lighter, as the light comes through it
        Finish::Translucent => plastic
            .colour
            .map(|c| c + ((255 - c as u32) * 40 / 255) as u8),
        Finish::Glitter if cart::fleck_at(x, y) => [0xff; 3],
        Finish::Glitter => plastic
            .colour
            .map(|c| c + ((255 - c as u32) * 24 / 255) as u8),
    };
    light_px(base, light)
}

/// `base` lit by `light` (128 leaves it as it is).
fn light_px(base: [u8; 3], light: u8) -> [u8; 3] {
    let s = light as f32 / 128.0;
    base.map(|c| {
        let c = c as f32;
        let v = if s <= 1.0 {
            c * s
        } else {
            c + (255.0 - c) * (s - 1.0)
        };
        v.round().clamp(0.0, 255.0) as u8
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_light_keeps_the_plastic_colour_and_shadow_darkens_it() {
        let grey = Shell {
            colour: [0x9a, 0x97, 0x8f],
            finish: Finish::Solid,
        };
        assert_eq!(plastic_px(&grey, 128, 0, 0, 10), grey.colour);
        let dark = plastic_px(&grey, 64, 0, 0, 10);
        assert!(dark[0] < grey.colour[0] && dark[0] > 60);
        let lit = plastic_px(&grey, 200, 0, 0, 10);
        assert!(lit[0] > grey.colour[0]);
    }

    #[test]
    fn every_model_has_its_tint_and_a_label_window_inside_it() {
        for dir in [
            "model-gb",
            "model-gbc",
            "model-gba",
            "model-nes",
            "model-famicom",
            "model-snes",
            "model-snes-pal",
            "model-ps1",
        ] {
            let m = shell_png::by_dir(dir).unwrap_or_else(|| panic!("{dir} missing"));
            assert!(m.x + m.lw <= m.w && m.y + m.lh <= m.h, "{dir} label window");
            assert_eq!(m.tint.is_some(), dir != "model-ps1", "{dir} tint");
        }
    }
}
