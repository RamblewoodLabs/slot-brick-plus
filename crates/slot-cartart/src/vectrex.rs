//! Cut the coloured overlay illustration out of a Vectrex box cover.
//!
//! The Vectrex shelf is the console itself, and the label window is its screen. A Vectrex
//! box already pictures that overlay under the VECTREX wordmark, so laying the whole cover
//! on the screen showed two images (the box, and the smaller overlay inside it). This finds
//! the overlay panel on the characteristic light graph-paper and returns just that.
//!
//! Pictures that are not a Vectrex box (a logo on a dark field, an overlay already cut out)
//! are left alone.

use crate::image::{Image, Rect};

/// Luma of an sRGB pixel, 0..=255.
fn luma(p: &[u8]) -> f32 {
    0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32
}

fn chroma(p: &[u8]) -> u8 {
    p[..3].iter().max().copied().unwrap_or(0) - p[..3].iter().min().copied().unwrap_or(0)
}

fn sat(p: &[u8]) -> f32 {
    let mx = *p[..3].iter().max().unwrap() as f32;
    if mx <= 0.0 {
        0.0
    } else {
        chroma(p) as f32 / mx
    }
}

/// The light, nearly grey graph-paper of a Vectrex box cover.
fn is_paper(p: &[u8]) -> bool {
    p[3] > 16 && luma(p) >= 185.0 && chroma(p) <= 18 && sat(p) < 0.12
}

/// Bounding box of the graph-paper, when there is enough of it to be a Vectrex box.
fn paper_bbox(px: &[u8], w: u32, h: u32) -> Option<Rect> {
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0u32, 0u32);
    let mut n = 0u32;
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            if is_paper(&px[i..i + 4]) {
                n += 1;
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
    }
    let area = w.saturating_mul(h);
    if n == 0 || area == 0 {
        return None;
    }
    let frac = n as f32 / area as f32;
    let (bw, bh) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
    (frac >= 0.06 && bw as f32 >= w as f32 * 0.35 && bh as f32 >= h as f32 * 0.35)
        .then_some(Rect::new(x0, y0, x1, y1))
}

fn median_colour(samples: &[[u8; 3]]) -> [u8; 3] {
    let mut out = [0u8; 3];
    for c in 0..3 {
        let mut v: Vec<u8> = samples.iter().map(|s| s[c]).collect();
        v.sort_unstable();
        out[c] = v[v.len() / 2];
    }
    out
}

/// Background colour of a crop: the median of its edge pixels, preferring light paper.
fn edge_bg(px: &[u8], w: u32, h: u32) -> [u8; 3] {
    let mut samples = Vec::with_capacity((w + h) as usize * 2);
    let mut push = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        samples.push([px[i], px[i + 1], px[i + 2]]);
    };
    for x in 0..w {
        push(x, 0);
        if h > 1 {
            push(x, h - 1);
        }
    }
    for y in 0..h {
        push(0, y);
        if w > 1 {
            push(w - 1, y);
        }
    }
    let light: Vec<[u8; 3]> = samples
        .iter()
        .copied()
        .filter(|s| luma(s) >= 185.0)
        .collect();
    let use_samples = if light.len() >= 8usize.max(samples.len() / 5) {
        &light
    } else {
        &samples
    };
    if use_samples.is_empty() {
        [221, 221, 221]
    } else {
        median_colour(use_samples)
    }
}

fn dilate(mask: &mut [u8], w: u32, h: u32, radius: i32) {
    let src = mask.to_vec();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut m = 0u8;
            'nb: for ny in (y - radius)..=(y + radius) {
                if ny < 0 || ny >= h as i32 {
                    continue;
                }
                for nx in (x - radius)..=(x + radius) {
                    if nx < 0 || nx >= w as i32 {
                        continue;
                    }
                    if src[(ny as u32 * w + nx as u32) as usize] >= 128 {
                        m = 255;
                        break 'nb;
                    }
                }
            }
            mask[(y as u32 * w + x as u32) as usize] = m;
        }
    }
}

fn erode(mask: &mut [u8], w: u32, h: u32, radius: i32) {
    let src = mask.to_vec();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut m = 255u8;
            'nb: for ny in (y - radius)..=(y + radius) {
                if ny < 0 || ny >= h as i32 {
                    m = 0;
                    break 'nb;
                }
                for nx in (x - radius)..=(x + radius) {
                    if nx < 0 || nx >= w as i32 || src[(ny as u32 * w + nx as u32) as usize] < 128 {
                        m = 0;
                        break 'nb;
                    }
                }
            }
            mask[(y as u32 * w + x as u32) as usize] = m;
        }
    }
}

/// Foreground mask: ink and colourful art on the paper, then closed so a panel stays one piece.
fn foreground_mask(px: &[u8], w: u32, h: u32, bg: [u8; 3]) -> Vec<u8> {
    let mut mask = vec![0u8; (w * h) as usize];
    let bgl = luma(&bg);
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let p = &px[i..i + 4];
            if p[3] < 16 {
                continue;
            }
            let far = (0..3).any(|c| (p[c] as i16 - bg[c] as i16).unsigned_abs() > 28);
            let colorful = sat(p) >= 0.18 && luma(p) < 230.0;
            let dark = luma(p) < 80.0;
            if far && (colorful || dark || luma(p) < bgl - 20.0) {
                mask[(y * w + x) as usize] = 255;
            }
        }
    }
    // Close small gaps (graph lines inside an overlay, rounded corners).
    dilate(&mut mask, w, h, 2);
    erode(&mut mask, w, h, 1);
    mask
}

struct Component {
    area: u32,
    fill: f32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

fn components(mask: &[u8], w: u32, h: u32, min_area: u32) -> Vec<Component> {
    let mut seen = vec![false; (w * h) as usize];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for y0 in 0..h {
        for x0 in 0..w {
            let s = (y0 * w + x0) as usize;
            if mask[s] < 128 || seen[s] {
                continue;
            }
            stack.clear();
            stack.push((x0, y0));
            seen[s] = true;
            let (mut x_min, mut x_max, mut y_min, mut y_max) = (x0, x0, y0, y0);
            let mut area = 0u32;
            while let Some((x, y)) = stack.pop() {
                area += 1;
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
                for (nx, ny) in [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ] {
                    if nx >= w || ny >= h {
                        continue;
                    }
                    let i = (ny * w + nx) as usize;
                    if !seen[i] && mask[i] >= 128 {
                        seen[i] = true;
                        stack.push((nx, ny));
                    }
                }
            }
            let bw = x_max - x_min + 1;
            let bh = y_max - y_min + 1;
            if area >= min_area && bw > 0 && bh > 0 {
                out.push(Component {
                    area,
                    fill: area as f32 / (bw * bh) as f32,
                    x0: x_min,
                    y0: y_min,
                    x1: x_max + 1,
                    y1: y_max + 1,
                });
            }
        }
    }
    out.sort_by(|a, b| b.area.cmp(&a.area));
    out
}

fn pick_overlay(comps: &[Component], tw: u32, th: u32) -> Option<&Component> {
    let mut best: Option<(f32, &Component)> = None;
    for c in comps {
        let bw = c.x1 - c.x0;
        let bh = c.y1 - c.y0;
        let aspect = bw as f32 / bh.max(1) as f32;
        let frac = c.area as f32 / (tw * th) as f32;
        let mut score = c.area as f32 * c.fill;
        if (0.55..=1.8).contains(&aspect) {
            score *= 1.3;
        }
        if (c.y0 + c.y1) as f32 / 2.0 > th as f32 * 0.35 {
            score *= 1.15;
        }
        let tall = bh as f32 / th as f32 >= 0.28;
        if tall && c.fill >= 0.45 && (0.08..=0.85).contains(&frac) {
            if best.map(|(s, _)| score > s).unwrap_or(true) {
                best = Some((score, c));
            }
        }
    }
    if best.is_none() {
        for c in comps.iter().take(3) {
            let bh = c.y1 - c.y0;
            let frac = c.area as f32 / (tw * th) as f32;
            if c.fill >= 0.4 && bh as f32 / th as f32 >= 0.22 && frac <= 0.85 {
                return Some(c);
            }
        }
    }
    best.map(|(_, c)| c)
}

/// The overlay illustration on a Vectrex box cover, in the picture's pixels.
///
/// `None` when the picture is not a Vectrex box (no graph-paper, or no overlay panel):
/// the caller should use the picture as it is.
pub fn vectrex_overlay_bounds(px: &[u8], w: u32, h: u32) -> Option<Rect> {
    if w == 0 || h == 0 || px.len() < (w as usize) * (h as usize) * 4 {
        return None;
    }
    let paper = paper_bbox(px, w, h)?;
    let (tw, th) = (paper.w(), paper.h());
    if tw < 16 || th < 16 {
        return None;
    }
    // Work in the paper crop so pillarboxed scans (grey bars beside a tall box) have
    // light graph-paper at their new edges.
    let mut crop = Vec::with_capacity((tw * th * 4) as usize);
    for y in paper.y0..paper.y1 {
        let a = ((y * w + paper.x0) * 4) as usize;
        crop.extend_from_slice(&px[a..a + (tw * 4) as usize]);
    }
    let bg = edge_bg(&crop, tw, th);
    if luma(&bg) < 160.0 {
        return None;
    }
    let mask = foreground_mask(&crop, tw, th, bg);
    let comps = components(&mask, tw, th, ((tw * th) as f32 * 0.04) as u32);
    let c = pick_overlay(&comps, tw, th)?;
    let pad = 2u32.max((0.01 * tw.min(th) as f32).round() as u32);
    let x0 = c.x0.saturating_sub(pad);
    let y0 = c.y0.saturating_sub(pad);
    let x1 = (c.x1 + pad).min(tw);
    let y1 = (c.y1 + pad).min(th);
    Some(Rect::new(
        paper.x0 + x0,
        paper.y0 + y0,
        paper.x0 + x1,
        paper.y0 + y1,
    ))
}

/// The overlay cropped from `img`, when one is found.
pub fn vectrex_overlay(img: &Image) -> Option<Image> {
    let r = vectrex_overlay_bounds(&img.px, img.w, img.h)?;
    let cropped = img.crop(r);
    (cropped.w >= 16 && cropped.h >= 16).then_some(cropped)
}
