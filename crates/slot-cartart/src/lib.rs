//! Cart art: ScreenScraper's pictures of a game turned into the label its cart wears on the
//! shelf, on the device, once, when the art is fetched (never per frame).
//!
//! Slot draws every cart itself (a modelled shell per system, `docs/cart-shells.md`), with a
//! label window the art is laid into. So what this makes is that label, shaped to the window:
//!
//! 1. **texture**: ScreenScraper's `support-texture`, a flat scan of the cart's own label.
//!    The backdrop is flooded away, the scan straightened and trimmed to the label's edge.
//! 2. **photo**: the label cut out of `support-2D`, a picture of the whole cart. Where the
//!    label sits is known for each system's carts and fine-tuned to where the plastic stops.
//! 3. **box**: the box cover (`box-2D`) cut to the window, from the top, where its title is.
//! 4. **logo**: the game's logo (`wheel`) whole, on a colour taken from its art.
//!
//! Arcade's label is its marquee, so a marquee comes first there. Each label is then
//! straightened, cut to the window's shape (only ever cut a little: a picture far from the
//! window's shape is kept whole instead, its edge carried out or on a soft blow-up of
//! itself; nothing is squeezed), resized with a proper filter, given a gentle levels lift and
//! a little sharpening, and written as an opaque PNG for the shelf to load.
//!
//! No network, no allocation per frame, no dependencies beyond `png`.

pub mod analyze;
pub mod image;
mod profile;
mod vectrex;

use std::time::Instant;

pub use image::{decode_png, encode_png_rgb, Image, Rect};
pub use profile::{profile, Profile, SYSTEMS};
pub use vectrex::{vectrex_overlay, vectrex_overlay_bounds};

use analyze::{border_colour, foreground, percentiles, refine_label, skew_degrees, Energy};
use image::{resize, rotate, sharpen, soft_blur};

/// What a ScreenScraper picture is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `support-texture`: the cart's label, scanned flat.
    Texture,
    /// `support-2D`: the whole cart (or disc), photographed or rendered from the front.
    Photo,
    /// `box-2D`: the front of the box.
    Box,
    /// `wheel` / `wheel-hd`: the logo, on transparency.
    Logo,
    /// `screenmarquee` / `marquee`: an arcade cabinet's marquee.
    Marquee,
}

impl Kind {
    /// ScreenScraper's media type names, best first.
    pub fn media_types(self) -> &'static [&'static str] {
        match self {
            Kind::Texture => &["support-texture"],
            Kind::Photo => &["support-2D"],
            Kind::Box => &["box-2D"],
            Kind::Logo => &["wheel-hd", "wheel"],
            Kind::Marquee => &["marquee", "screenmarquee"],
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Texture => "texture",
            Kind::Photo => "photo",
            Kind::Box => "box",
            Kind::Logo => "logo",
            Kind::Marquee => "marquee",
        }
    }
}

/// A decoded picture and what it is.
pub struct Source {
    pub kind: Kind,
    pub img: Image,
}

/// The label made, the kind of picture it came from, and how it was fitted (for the log).
pub struct Fitted {
    pub img: Image,
    pub tier: Kind,
    pub how: String,
    /// Microseconds spent fitting (decoding and encoding not included).
    pub micros: u128,
    /// The label keeps transparency (a Game & Watch handheld): write it with `encode_png_rgba`.
    pub transparent: bool,
}

impl Fitted {
    /// The label as the PNG the shelf loads: opaque RGB, or RGBA when it has an outline.
    pub fn png(&self) -> Vec<u8> {
        if self.transparent {
            image::encode_png_rgba(&self.img)
        } else {
            encode_png_rgb(&self.img)
        }
    }
}

/// Window shapes this close are filled by a stretch nobody can see, so no edge is lost.
const STRETCH: f32 = 0.06; // ln(1.06)
/// Up to this far apart, the picture is cut to the window's shape.
const NEAR: f32 = 0.262; // ln(1.3)

fn miss(a: f32, b: f32) -> f32 {
    (a / b).ln().abs()
}

/// Make the label for a cart of system `dir` (the card folder: "GB", "MD", "PSP"...) whose
/// label window is `w` by `h`, at `scale` times that size, from whatever pictures there are.
pub fn fit(dir: &str, w: u32, h: u32, scale: u32, sources: &[Source]) -> Option<Fitted> {
    fit_in_order(dir, w, h, scale, sources, None)
}

/// `fit` with only one kind of picture allowed to make the label (the others still lend a
/// logo its colour): for checking each fallback on its own.
pub fn fit_tier(
    dir: &str,
    w: u32,
    h: u32,
    scale: u32,
    sources: &[Source],
    kind: Kind,
) -> Option<Fitted> {
    fit_in_order(dir, w, h, scale, sources, Some(&[kind]))
}

fn fit_in_order(
    dir: &str,
    w: u32,
    h: u32,
    scale: u32,
    sources: &[Source],
    order: Option<&[Kind]>,
) -> Option<Fitted> {
    let t = Instant::now();
    let p = profile(dir);
    let (ow, oh) = (w * scale.max(1), h * scale.max(1));
    let window = w as f32 / h as f32;
    let field = field_colour(sources);
    let mut notes = Vec::new();
    for kind in order.unwrap_or(p.order) {
        for src in sources.iter().filter(|s| s.kind == *kind) {
            if p.whole_object && matches!(kind, Kind::Photo | Kind::Texture) {
                if let Some((obj, _, _)) = isolate(&src.img, false) {
                    let mut img = stand(&obj, ow, oh);
                    sharpen(&mut img, 0.35, 3);
                    let how = format!("{} whole, standing on its outline", kind.name());
                    return Some(Fitted {
                        img,
                        tier: *kind,
                        how,
                        micros: t.elapsed().as_micros(),
                        transparent: true,
                    });
                }
            }
            let made = match kind {
                Kind::Texture => texture_label(&p, &src.img, window)
                    .map(|(l, n)| (fill(&l, ow, oh, Cut::Middle, p.label_cut), n)),
                Kind::Photo => photo_label(&p, &src.img, window)
                    .map(|(l, n)| (fill(&l, ow, oh, Cut::Middle, p.label_cut), n)),
                Kind::Box => box_label(&src.img).map(|l| {
                    let (l, note) = if dir == "VECTREX" {
                        match crate::vectrex::vectrex_overlay(&l) {
                            Some(overlay) => (overlay, "overlay cut from the box"),
                            None => (l, ""),
                        }
                    } else {
                        (l, "")
                    };
                    (
                        fill(
                            &l,
                            ow,
                            oh,
                            if p.box_top { Cut::Top } else { Cut::Detail },
                            NEAR,
                        ),
                        note.to_string(),
                    )
                }),
                Kind::Marquee => marquee_label(&src.img)
                    .map(|l| (fill(&l, ow, oh, Cut::Detail, NEAR), String::new())),
                Kind::Logo => Some((
                    (
                        logo_label(
                            &src.img,
                            ow,
                            oh,
                            if p.black_field { [0, 0, 0] } else { field },
                        ),
                        "whole",
                    ),
                    String::new(),
                )),
            };
            match made {
                Some(((mut img, cut), note)) => {
                    let photo = *kind == Kind::Photo;
                    polish(&mut img, photo, *kind != Kind::Logo);
                    let mut how = format!("{} {cut}", kind.name());
                    if !note.is_empty() {
                        how.push_str(&format!(" ({note})"));
                    }
                    if !notes.is_empty() {
                        how.push_str(&format!("; passed over: {}", notes.join(", ")));
                    }
                    return Some(Fitted {
                        img,
                        tier: *kind,
                        how,
                        micros: t.elapsed().as_micros(),
                        transparent: false,
                    });
                }
                None => notes.push(kind.name().to_string()),
            }
        }
    }
    None
}

/// `fit` from PNG bytes, to PNG bytes: what the fetcher calls.
pub fn fit_png(
    dir: &str,
    w: u32,
    h: u32,
    scale: u32,
    pictures: &[(Kind, &[u8])],
) -> Option<(Vec<u8>, Fitted)> {
    let sources: Vec<Source> = pictures
        .iter()
        .filter_map(|(kind, bytes)| {
            Some(Source {
                kind: *kind,
                img: decode_png(bytes)?,
            })
        })
        .collect();
    let fitted = fit(dir, w, h, scale, &sources)?;
    Some((fitted.png(), fitted))
}

// ---------------------------------------------------------------- sources to labels

/// The object (a label scan, a cart) found, straightened and cut out: none of the backdrop.
fn isolate(img: &Image, deskew: bool) -> Option<(Image, Option<[u8; 3]>, f32)> {
    let m = foreground(img);
    if m.coverage() < 0.08 {
        return None;
    }
    let bg = m.bg;
    let fill = bg.map(|c| [c[0], c[1], c[2], 255]).unwrap_or([0, 0, 0, 0]);
    let mut angle = 0.0;
    let skew = if deskew { skew_degrees(&m) } else { None };
    let straight = match skew {
        Some(a) => {
            angle = a;
            // Turn back, on a canvas the backdrop colour, then find the object again.
            let mut canvas = img.clone();
            if bg.is_some() {
                for p in canvas.px.chunks_exact_mut(4) {
                    if p[3] < 255 {
                        p.copy_from_slice(&fill);
                    }
                }
            }
            let mut r = rotate(&canvas, -a);
            if bg.is_some() {
                for p in r.px.chunks_exact_mut(4) {
                    if p[3] < 128 {
                        p.copy_from_slice(&fill);
                    } else {
                        p[3] = 255;
                    }
                }
            }
            r
        }
        None => img.clone(),
    };
    let m2 = if angle != 0.0 {
        foreground(&straight)
    } else {
        m
    };
    let bb = m2.bbox()?;
    Some((straight.crop(bb), bg, angle))
}

fn texture_label(p: &Profile, img: &Image, window: f32) -> Option<(Image, String)> {
    let (obj, bg, angle) = isolate(img, p.texture_deskew)?;
    let mut label = match p.texture {
        Some(f) => obj.crop(Rect::new(0, 0, obj.w, obj.h).sub(f)),
        None => obj,
    };
    // In from the edge: a scan's rounded corners and the last pixels of backdrop.
    let ix = (label.w as f32 * p.texture_inset).round() as u32;
    let iy = (label.h as f32 * p.texture_inset).round() as u32;
    label = label.crop(Rect::new(ix, iy, label.w - ix, label.h - iy));
    if label.w < 48 || label.h < 32 {
        return None;
    }
    // A texture far from the label's shape is another kind of picture (a wrap of the whole cart).
    let want = p.texture_aspect.unwrap_or(window);
    if miss(label.aspect(), want) > p.texture_tolerance {
        return None;
    }
    let label = label.flatten(bg.unwrap_or([255, 255, 255]));
    let mut note = Vec::new();
    if angle != 0.0 {
        note.push(format!("straightened {angle:+.1}°"));
    }
    if bg.is_some() {
        note.push("backdrop removed".into());
    }
    Some((label, note.join(", ")))
}

fn photo_label(p: &Profile, img: &Image, window: f32) -> Option<(Image, String)> {
    let first = p.photo.as_ref()?;
    let (obj, _bg, angle) = isolate(img, first.deskew)?;
    // Another cart (a Japanese one, a CD case) has another shape: its label isn't where we'd
    // cut. A system with two carts (NES and Famicom) knows both; the label window's shape
    // says which one the shelf is drawing.
    let ph = [Some(first), p.photo_alt.as_ref()]
        .into_iter()
        .flatten()
        .filter(|ph| miss(obj.aspect(), ph.aspect) <= ph.tolerance)
        .min_by(|a, b| miss(a.label_aspect(), window).total_cmp(&miss(b.label_aspect(), window)))?;
    let whole = Rect::new(0, 0, obj.w, obj.h);
    let guess = whole.sub(ph.label);
    let r = if ph.refine {
        refine_label(&obj, guess, whole.sub(ph.plastic), 0.06)
    } else {
        guess
    };
    let label = obj.crop(r).flatten([0, 0, 0]);
    if label.w < 40 || label.h < 24 {
        return None;
    }
    let mut note = Vec::new();
    if angle != 0.0 {
        note.push(format!("straightened {angle:+.1}°"));
    }
    if r != guess {
        note.push("edges found".to_string());
    }
    Some((label, note.join(", ")))
}

fn box_label(img: &Image) -> Option<Image> {
    let (obj, bg, _) = isolate(img, true)?;
    Some(obj.flatten(bg.unwrap_or([0, 0, 0])))
}

fn marquee_label(img: &Image) -> Option<Image> {
    let (obj, bg, _) = isolate(img, true)?;
    // A marquee that is mostly see-through is a logo, not a marquee.
    if obj.mean_alpha() < 200.0 {
        return None;
    }
    Some(obj.flatten(bg.unwrap_or([0, 0, 0])))
}

/// The logo whole and centred on `field`.
fn logo_label(img: &Image, w: u32, h: u32, field: [u8; 3]) -> Image {
    // A logo is drawn on transparency: its ink's own bounds, however thin a wordmark it is.
    let obj = match ink_bounds(img) {
        Some(r) => img.crop(r),
        None => isolate(img, false)
            .map(|o| o.0)
            .unwrap_or_else(|| img.clone()),
    };
    let mut out = Image::new(w, h, [field[0], field[1], field[2], 255]);
    let (rw, rh) = (w as f32 * 0.86, h as f32 * 0.74);
    let k = (rw / obj.w as f32).min(rh / obj.h as f32);
    let (lw, lh) = (
        ((obj.w as f32 * k).round() as u32).max(1),
        ((obj.h as f32 * k).round() as u32).max(1),
    );
    let logo = resize(&obj, lw, lh);
    out.blit(&logo, ((w - lw) / 2) as i32, ((h - lh) / 2) as i32);
    out
}

/// Where a picture with transparency has ink (alpha over 24), when it has any transparency.
fn ink_bounds(img: &Image) -> Option<Rect> {
    if !img.has_alpha() {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for y in 0..img.h {
        let row = &img.px[(y * img.w * 4) as usize..((y + 1) * img.w * 4) as usize];
        for (x, p) in row.chunks_exact(4).enumerate() {
            if p[3] > 24 {
                let x = x as u32;
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
            }
        }
    }
    (x1 > x0 + 8 && y1 > y0 + 4).then(|| Rect::new(x0, y0, x1, y1))
}

/// A dark, rich colour from the game's own art (its box, else its label), for a logo to sit
/// on; a deep blue-grey when there is none.
fn field_colour(sources: &[Source]) -> [u8; 3] {
    let pick = [Kind::Box, Kind::Texture, Kind::Photo, Kind::Marquee]
        .iter()
        .find_map(|k| sources.iter().find(|s| s.kind == *k));
    let Some(src) = pick else { return [24, 26, 38] };
    let small = resize(&src.img.flatten([0, 0, 0]), 24, 24);
    // Median hue by way of the median of each channel, then made dark and saturated.
    let mut ch: [Vec<u8>; 3] = Default::default();
    for p in small.px.chunks_exact(4) {
        for c in 0..3 {
            ch[c].push(p[c]);
        }
    }
    let m = [0, 1, 2].map(|c| {
        ch[c].sort_unstable();
        ch[c][ch[c].len() / 2] as f32
    });
    let max = m[0].max(m[1]).max(m[2]).max(1.0);
    let min = m[0].min(m[1]).min(m[2]);
    // Push away from grey, then scale so the brightest channel is ~110.
    let sat = |v: f32| min + (v - min) * 1.6;
    let k = 110.0 / sat(max).max(1.0);
    [0, 1, 2].map(|c| (sat(m[c]) * k).clamp(8.0, 140.0) as u8)
}

// ---------------------------------------------------------------- fitting to the window

#[derive(Clone, Copy, PartialEq)]
enum Cut {
    /// Keep the middle (a label: its art is centred).
    Middle,
    /// Keep the top (a box cover: its title is at the top).
    Top,
    /// Keep where the detail is, a little in favour of the middle.
    Detail,
}

/// The picture shaped to `w` by `h`: stretched invisibly, cut a little, or kept whole.
fn fill(src: &Image, w: u32, h: u32, cut: Cut, cut_up_to: f32) -> (Image, &'static str) {
    let (a, want) = (src.aspect(), w as f32 / h as f32);
    let m = miss(a, want);
    if m <= STRETCH {
        return (resize(src, w, h), "fitted");
    }
    // A tall box cover on a wide window: its top (the title and the art under it) is the
    // label; on a very wide one (an end label, a disk's sticker) the top band is its title.
    let tall_box = cut == Cut::Top && a < want && (m <= 0.75 || want > 2.5);
    if m <= cut_up_to || tall_box {
        let r = cut_box(src, want, cut);
        return (resize(&src.crop(r), w, h), "cut to shape");
    }
    let (edge, spread) = border_colour(src);
    if spread < 14.0 {
        return (extend(src, w, h, edge), "whole, edge carried out");
    }
    (backdrop(src, w, h), "whole on its own backdrop")
}

/// The window-shaped box to cut: where `cut` says.
fn cut_box(src: &Image, want: f32, cut: Cut) -> Rect {
    let (sw, sh) = (src.w, src.h);
    let (cw, ch) = if src.aspect() > want {
        (((sh as f32) * want).round() as u32, sh)
    } else {
        (sw, ((sw as f32) / want).round() as u32)
    };
    let (cw, ch) = (cw.clamp(1, sw), ch.clamp(1, sh));
    let wide = cw < sw;
    let span = if wide { sw - cw } else { sh - ch };
    let off = match cut {
        Cut::Middle => span / 2,
        Cut::Top if !wide => 0,
        Cut::Top | Cut::Detail => {
            // Slide along and keep the most detailed place, a little in favour of the middle.
            let (small, s) = analyze::probe(src);
            let e = Energy::of(&small);
            let (pw, ph) = (((cw as f32) / s) as u32, ((ch as f32) / s) as u32);
            let steps = 24;
            let mut best = (span / 2, f32::MIN);
            for i in 0..=steps {
                let off = span * i / steps;
                let po = (off as f32 / s) as u32;
                let energy = if wide {
                    e.mean(po, 0, po + pw, ph)
                } else {
                    e.mean(0, po, pw, po + ph)
                };
                let u = i as f32 / steps as f32;
                let score = energy * (1.0 - 0.25 * (u - 0.5).abs() * 2.0);
                if score > best.1 {
                    best = (off, score);
                }
            }
            best.0
        }
    };
    if wide {
        Rect::new(off, 0, off + cw, ch)
    } else {
        Rect::new(0, off, cw, off + ch)
    }
}

/// The object whole, on transparency, as large as fits, standing on the bottom edge.
fn stand(obj: &Image, w: u32, h: u32) -> Image {
    let mut out = Image::new(w, h, [0, 0, 0, 0]);
    let k = (w as f32 / obj.w as f32).min(h as f32 / obj.h as f32);
    let (pw, ph) = (
        ((obj.w as f32 * k).round() as u32).clamp(1, w),
        ((obj.h as f32 * k).round() as u32).clamp(1, h),
    );
    let pic = resize(obj, pw, ph);
    let (x, y) = ((w - pw) / 2, h - ph);
    for py in 0..ph {
        for px in 0..pw {
            out.put(x + px, y + py, pic.get(px, py));
        }
    }
    out
}

/// The whole picture, its plain edge colour carried out to the window.
fn extend(src: &Image, w: u32, h: u32, edge: [u8; 3]) -> Image {
    let mut out = Image::new(w, h, [edge[0], edge[1], edge[2], 255]);
    let k = (w as f32 / src.w as f32).min(h as f32 / src.h as f32);
    let (pw, ph) = (
        ((src.w as f32 * k).round() as u32).clamp(1, w),
        ((src.h as f32 * k).round() as u32).clamp(1, h),
    );
    out.blit(
        &resize(src, pw, ph),
        ((w - pw) / 2) as i32,
        ((h - ph) / 2) as i32,
    );
    out
}

/// The whole picture over a soft, dim blow-up of itself.
fn backdrop(src: &Image, w: u32, h: u32) -> Image {
    let r = cut_box(src, w as f32 / h as f32, Cut::Middle);
    let mut back = soft_blur(&resize(&src.crop(r), w, h), 10);
    for p in back.px.chunks_exact_mut(4) {
        for v in &mut p[..3] {
            *v = (*v as u32 * 140 / 255) as u8;
        }
        p[3] = 255;
    }
    let k = (w as f32 / src.w as f32).min(h as f32 / src.h as f32);
    let (pw, ph) = (
        ((src.w as f32 * k).round() as u32).clamp(1, w),
        ((src.h as f32 * k).round() as u32).clamp(1, h),
    );
    back.blit(
        &resize(src, pw, ph),
        ((w - pw) / 2) as i32,
        ((h - ph) / 2) as i32,
    );
    back
}

// ---------------------------------------------------------------- finishing

/// A gentle levels lift (a faded scan gets its blacks and whites back, a good one is left
/// alone), a touch more colour for a photo (its studio light washes the print out) and
/// a little sharpening.
fn polish(img: &mut Image, photo: bool, levels: bool) {
    if levels {
        let (lo, hi) = percentiles(img, 0.004, 0.996);
        if hi > lo + 48 {
            // Move only part of the way: keep the print's own character.
            let lo2 = lo as f32 * 0.65;
            let hi2 = hi as f32 + (255.0 - hi as f32) * 0.35;
            let gain = (255.0 / (hi2 - lo2)).min(1.3);
            if gain > 1.02 {
                let lut: Vec<u8> = (0..256)
                    .map(|v| ((v as f32 - lo2) * gain).round().clamp(0.0, 255.0) as u8)
                    .collect();
                for p in img.px.chunks_exact_mut(4) {
                    for c in 0..3 {
                        p[c] = lut[p[c] as usize];
                    }
                }
            }
        }
    }
    if photo {
        for p in img.px.chunks_exact_mut(4) {
            let l = (p[0] as i32 * 77 + p[1] as i32 * 150 + p[2] as i32 * 29) >> 8;
            for v in &mut p[..3] {
                *v = (l + (*v as i32 - l) * 9 / 8).clamp(0, 255) as u8;
            }
        }
    }
    sharpen(img, 0.45, 3);
    for p in img.px.chunks_exact_mut(4) {
        p[3] = 255;
    }
}
