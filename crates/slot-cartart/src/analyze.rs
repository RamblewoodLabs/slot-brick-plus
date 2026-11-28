//! Finding the thing in a picture: its background (transparent, or a flat colour round the
//! edge), the object's outline, how far it is turned, and where a label sits on a cart.
//! All of it runs on a shrunk copy (at most `PROBE` pixels across), so it costs about the
//! same for a 600 px scan as for a 3000 px one.

use crate::image::{resize, Image, Rect};

/// The longest side of the copy analysis runs on.
pub const PROBE: u32 = 256;

/// A foreground mask (1 = object) at probe size, and how probe pixels map back.
pub struct Mask {
    pub w: u32,
    pub h: u32,
    pub on: Vec<u8>,
    /// Source pixels per probe pixel.
    pub scale: f32,
    /// The colour the background was taken to be (None: it was transparency).
    pub bg: Option<[u8; 3]>,
    /// The source picture's size.
    pub src: (u32, u32),
}

impl Mask {
    #[inline]
    pub fn at(&self, x: u32, y: u32) -> bool {
        self.on[(y * self.w + x) as usize] != 0
    }

    pub fn coverage(&self) -> f32 {
        self.on.iter().filter(|v| **v != 0).count() as f32 / self.on.len().max(1) as f32
    }

    /// The bounding box of the object, in source pixels; None when there is no object.
    pub fn bbox(&self) -> Option<Rect> {
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        // A row or column counts once 1.5% of it is object: lone specks and shadows don't.
        let min_row = (self.w as f32 * 0.015).ceil() as u32;
        let min_col = (self.h as f32 * 0.015).ceil() as u32;
        for y in 0..self.h {
            let n = (0..self.w).filter(|&x| self.at(x, y)).count() as u32;
            if n > min_row {
                y0 = y0.min(y);
                y1 = y1.max(y + 1);
            }
        }
        for x in 0..self.w {
            let n = (0..self.h).filter(|&y| self.at(x, y)).count() as u32;
            if n > min_col {
                x0 = x0.min(x);
                x1 = x1.max(x + 1);
            }
        }
        if x0 >= x1 || y0 >= y1 {
            return None;
        }
        let s = self.scale;
        Some(
            Rect::new(
                (x0 as f32 * s).floor() as u32,
                (y0 as f32 * s).floor() as u32,
                ((x1 as f32 * s).ceil() as u32).min(self.src.0),
                ((y1 as f32 * s).ceil() as u32).min(self.src.1),
            )
            .clamp(self.src.0, self.src.1),
        )
    }
}

pub fn probe(img: &Image) -> (Image, f32) {
    let long = img.w.max(img.h);
    if long <= PROBE {
        return (img.clone(), 1.0);
    }
    let s = long as f32 / PROBE as f32;
    let small = resize(
        img,
        ((img.w as f32 / s).round() as u32).max(1),
        ((img.h as f32 / s).round() as u32).max(1),
    );
    let scale = img.w as f32 / small.w as f32;
    (small, scale)
}

fn dist(a: [u8; 4], b: [u8; 3]) -> i32 {
    let dr = a[0] as i32 - b[0] as i32;
    let dg = a[1] as i32 - b[1] as i32;
    let db = a[2] as i32 - b[2] as i32;
    // Weighted roughly as the eye weighs them.
    (2 * dr.abs() + 4 * dg.abs() + db.abs()) / 7
}

/// The median colour of the picture's outermost two-pixel ring, and how much it varies
/// (mean distance from that median). A flat backdrop varies little.
pub fn border_colour(img: &Image) -> ([u8; 3], f32) {
    let mut rs = Vec::new();
    let mut gs = Vec::new();
    let mut bs = Vec::new();
    let ring = 2.min(img.w / 4).min(img.h / 4).max(1);
    for y in 0..img.h {
        for x in 0..img.w {
            if x >= ring && y >= ring && x + ring < img.w && y + ring < img.h {
                continue;
            }
            let p = img.get(x, y);
            rs.push(p[0]);
            gs.push(p[1]);
            bs.push(p[2]);
        }
    }
    let med = |v: &mut Vec<u8>| {
        v.sort_unstable();
        v[v.len() / 2]
    };
    let m = [med(&mut rs), med(&mut gs), med(&mut bs)];
    let mut spread = 0i64;
    for i in 0..rs.len() {
        spread += dist([rs[i], gs[i], bs[i], 255], m) as i64;
    }
    (m, spread as f32 / rs.len().max(1) as f32)
}

/// The object in a picture: everything not transparent, or (an opaque scan on a flat
/// backdrop) everything not reached by flooding the backdrop colour in from the edges.
pub fn foreground(img: &Image) -> Mask {
    let (small, scale) = probe(img);
    let (w, h) = (small.w, small.h);
    let transparent = small.px.chunks_exact(4).filter(|p| p[3] < 128).count();
    if transparent as f32 > 0.02 * (w * h) as f32 {
        let on = small
            .px
            .chunks_exact(4)
            .map(|p| (p[3] >= 128) as u8)
            .collect();
        return Mask {
            w,
            h,
            on,
            scale,
            bg: None,
            src: (img.w, img.h),
        };
    }
    let (bg, spread) = border_colour(&small);
    let mut on = vec![1u8; (w * h) as usize];
    if spread > 22.0 {
        // A busy edge: the picture runs to its border. Nothing to take off.
        return Mask {
            w,
            h,
            on,
            scale,
            bg: None,
            src: (img.w, img.h),
        };
    }
    let tol = (spread * 2.5).clamp(14.0, 40.0) as i32;
    // Flood from every border pixel that is close to the backdrop colour.
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for x in 0..w {
        stack.push((x, 0));
        stack.push((x, h - 1));
    }
    for y in 0..h {
        stack.push((0, y));
        stack.push((w - 1, y));
    }
    while let Some((x, y)) = stack.pop() {
        let i = (y * w + x) as usize;
        if on[i] == 0 || dist(small.get(x, y), bg) > tol {
            continue;
        }
        on[i] = 0;
        if x > 0 {
            stack.push((x - 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
    let flooded = Mask {
        w,
        h,
        on,
        scale,
        bg: Some(bg),
        src: (img.w, img.h),
    };
    // A backdrop is a margin round the object. When the flood also ran far inside the object's
    // box (a red label with white letters on a red edge), that colour is the print itself:
    // keep the picture whole.
    if let Some(bb) = flooded.bbox() {
        let s = flooded.scale;
        let (x0, y0) = ((bb.x0 as f32 / s) as u32, (bb.y0 as f32 / s) as u32);
        let (x1, y1) = (
            ((bb.x1 as f32 / s) as u32).min(w),
            ((bb.y1 as f32 / s) as u32).min(h),
        );
        let area = ((x1 - x0) * (y1 - y0)).max(1);
        let inside = (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .filter(|&(x, y)| !flooded.at(x, y))
            .count() as u32;
        if inside as f32 > 0.12 * area as f32 {
            return Mask {
                w,
                h,
                on: vec![1u8; (w * h) as usize],
                scale,
                bg: None,
                src: (img.w, img.h),
            };
        }
    }
    flooded
}

/// Least-squares slope of points (t, s): ds/dt.
fn slope(pts: &[(f32, f32)]) -> Option<f32> {
    if pts.len() < 8 {
        return None;
    }
    let n = pts.len() as f32;
    let (st, ss) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.0, a.1 + p.1));
    let (mt, ms) = (st / n, ss / n);
    let (mut num, mut den) = (0.0, 0.0);
    for p in pts {
        num += (p.0 - mt) * (p.1 - ms);
        den += (p.0 - mt) * (p.0 - mt);
    }
    (den > 1e-3).then(|| num / den)
}

/// Fit with the worst quarter of points dropped (a notch, a rounded corner, a tab).
fn robust_slope(mut pts: Vec<(f32, f32)>) -> Option<f32> {
    let k = slope(&pts)?;
    let n = pts.len() as f32;
    let mt = pts.iter().map(|p| p.0).sum::<f32>() / n;
    let ms = pts.iter().map(|p| p.1).sum::<f32>() / n;
    pts.sort_by(|a, b| {
        let ra = (a.1 - (ms + k * (a.0 - mt))).abs();
        let rb = (b.1 - (ms + k * (b.0 - mt))).abs();
        ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
    });
    pts.truncate((pts.len() * 3 / 4).max(8));
    slope(&pts)
}

/// How many degrees clockwise the object is turned, from its four straight edges (each
/// edge's middle 70%), or None when it is square to the picture or has no straight edges.
/// Edges that disagree (a cart's tapered sides) are ignored: only the two that agree best count.
pub fn skew_degrees(m: &Mask) -> Option<f32> {
    let bb = {
        let r = m.bbox()?;
        let s = m.scale;
        (
            (r.x0 as f32 / s) as u32,
            (r.y0 as f32 / s) as u32,
            ((r.x1 as f32 / s) as u32).min(m.w),
            ((r.y1 as f32 / s) as u32).min(m.h),
        )
    };
    let (x0, y0, x1, y1) = bb;
    if x1 - x0 < 24 || y1 - y0 < 24 {
        return None;
    }
    let (ya, yb) = (y0 + (y1 - y0) * 15 / 100, y1 - (y1 - y0) * 15 / 100);
    let (xa, xb) = (x0 + (x1 - x0) * 15 / 100, x1 - (x1 - x0) * 15 / 100);
    let mut est = Vec::new();
    // Left and right edges: x as a function of y. A clockwise turn moves the left edge
    // left going down (dx/dy < 0) — angle = -atan(dx/dy).
    let left: Vec<_> = (ya..yb)
        .filter_map(|y| (x0..x1).find(|&x| m.at(x, y)).map(|x| (y as f32, x as f32)))
        .collect();
    let right: Vec<_> = (ya..yb)
        .filter_map(|y| {
            (x0..x1)
                .rev()
                .find(|&x| m.at(x, y))
                .map(|x| (y as f32, x as f32))
        })
        .collect();
    let top: Vec<_> = (xa..xb)
        .filter_map(|x| (y0..y1).find(|&y| m.at(x, y)).map(|y| (x as f32, y as f32)))
        .collect();
    let bottom: Vec<_> = (xa..xb)
        .filter_map(|x| {
            (y0..y1)
                .rev()
                .find(|&y| m.at(x, y))
                .map(|y| (x as f32, y as f32))
        })
        .collect();
    if let Some(k) = robust_slope(left) {
        est.push(-k.atan().to_degrees());
    }
    if let Some(k) = robust_slope(right) {
        est.push(-k.atan().to_degrees());
    }
    if let Some(k) = robust_slope(top) {
        est.push(k.atan().to_degrees());
    }
    if let Some(k) = robust_slope(bottom) {
        est.push(k.atan().to_degrees());
    }
    if est.len() < 2 {
        return None;
    }
    // The pair of edges that agree best.
    let mut best: Option<(f32, f32)> = None;
    for i in 0..est.len() {
        for j in i + 1..est.len() {
            let d = (est[i] - est[j]).abs();
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, (est[i] + est[j]) / 2.0));
            }
        }
    }
    let (spread, angle) = best?;
    // A turned rectangle turns all four of its edges: at least three must agree (a cart's
    // tapered or notched side may not).
    let agree = est.iter().filter(|e| (*e - angle).abs() <= 0.8).count();
    // Too small to see, too large to be a scan that slipped, or the edges don't agree.
    if spread > 0.8 || agree < 3 || angle.abs() < 0.3 || angle.abs() > 8.0 {
        return None;
    }
    Some(angle)
}

/// Mean squared gradient (detail) of a probe-sized grey picture over a rectangle.
pub fn grey(img: &Image) -> (Vec<u8>, u32, u32) {
    let g = img
        .px
        .chunks_exact(4)
        .map(|p| ((p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8) as u8)
        .collect();
    (g, img.w, img.h)
}

/// Gradient-energy integral image, for asking "how much detail is in this box" in O(1).
pub struct Energy {
    w: u32,
    h: u32,
    sum: Vec<u64>,
}

impl Energy {
    pub fn of(img: &Image) -> Energy {
        let (g, w, h) = grey(img);
        let mut sum = vec![0u64; ((w + 1) * (h + 1)) as usize];
        for y in 0..h {
            let mut row = 0u64;
            for x in 0..w {
                let c = g[(y * w + x) as usize] as i32;
                let r = if x + 1 < w {
                    g[(y * w + x + 1) as usize] as i32
                } else {
                    c
                };
                let d = if y + 1 < h {
                    g[((y + 1) * w + x) as usize] as i32
                } else {
                    c
                };
                row += ((c - r).abs() + (c - d).abs()) as u64;
                sum[((y + 1) * (w + 1) + x + 1) as usize] =
                    sum[(y * (w + 1) + x + 1) as usize] + row;
            }
        }
        Energy { w, h, sum }
    }

    /// Mean energy in `x0..x1, y0..y1` (probe pixels).
    pub fn mean(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> f32 {
        let (x1, y1) = (x1.min(self.w), y1.min(self.h));
        if x1 <= x0 || y1 <= y0 {
            return 0.0;
        }
        let at = |x: u32, y: u32| self.sum[(y * (self.w + 1) + x) as usize] as i64;
        let s = at(x1, y1) - at(x0, y1) - at(x1, y0) + at(x0, y0);
        s as f32 / ((x1 - x0) * (y1 - y0)) as f32
    }
}

/// Fine-tune where a label sits on a cart photo: starting from `guess` (source pixels), each
/// edge moves (by at most `slack` of the label's size) to where the plastic stops and the
/// printed label starts. The plastic's colour comes from `plastic` (source pixels). An edge
/// with no clear step stays where it was guessed.
pub fn refine_label(img: &Image, guess: Rect, plastic: Rect, slack: f32) -> Rect {
    let (small, s) = probe(img);
    let to = |v: u32| ((v as f32 / s).round() as u32).min(small.w.max(small.h));
    let g =
        Rect::new(to(guess.x0), to(guess.y0), to(guess.x1), to(guess.y1)).clamp(small.w, small.h);
    let pr = Rect::new(
        to(plastic.x0),
        to(plastic.y0),
        to(plastic.x1),
        to(plastic.y1),
    )
    .clamp(small.w, small.h);
    // The plastic: median of the sample area.
    let mut cols: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for y in pr.y0..pr.y1 {
        for x in pr.x0..pr.x1 {
            let p = small.get(x, y);
            if p[3] < 200 {
                continue;
            }
            for c in 0..3 {
                cols[c].push(p[c]);
            }
        }
    }
    if cols[0].len() < 6 {
        return guess;
    }
    let plastic_c = [0, 1, 2].map(|c| {
        cols[c].sort_unstable();
        cols[c][cols[c].len() / 2]
    });
    let is_label = |x: u32, y: u32| {
        let p = small.get(x, y);
        p[3] >= 200 && dist(p, plastic_c) > 26
    };
    let (gw, gh) = (g.w() as f32, g.h() as f32);
    let mx = (gw * slack).ceil() as i32;
    let my = (gh * slack).ceil() as i32;
    // Fraction of label-coloured pixels along a column (over the guessed rows' middle) or a row.
    let col_frac = |x: u32| {
        let (a, b) = (g.y0 + g.h() / 5, g.y1 - g.h() / 5);
        (a..b).filter(|&y| is_label(x, y)).count() as f32 / (b - a).max(1) as f32
    };
    let row_frac = |y: u32| {
        let (a, b) = (g.x0 + g.w() / 5, g.x1 - g.w() / 5);
        (a..b).filter(|&x| is_label(x, y)).count() as f32 / (b - a).max(1) as f32
    };
    // Walk from outside the guess inward; the edge is the first line that is mostly label.
    let edge = |from: i32, step: i32, n: i32, limit: u32, f: &dyn Fn(u32) -> f32| -> Option<i32> {
        let start = from - step * n;
        let mut prev = None;
        for k in 0..=2 * n {
            let v = start + step * k;
            if v < 0 || v >= limit as i32 {
                continue;
            }
            let frac = f(v as u32);
            if frac > 0.55 {
                // Only a real step counts: the line before it must have been mostly plastic.
                return match prev {
                    Some(p) if p < 0.35 => Some(v),
                    _ => None,
                };
            }
            prev = Some(frac);
        }
        None
    };
    let x0 = edge(g.x0 as i32, 1, mx, small.w, &col_frac).unwrap_or(g.x0 as i32);
    let x1 = edge(g.x1 as i32 - 1, -1, mx, small.w, &col_frac)
        .map(|v| v + 1)
        .unwrap_or(g.x1 as i32);
    let y0 = edge(g.y0 as i32, 1, my, small.h, &row_frac).unwrap_or(g.y0 as i32);
    let y1 = edge(g.y1 as i32 - 1, -1, my, small.h, &row_frac)
        .map(|v| v + 1)
        .unwrap_or(g.y1 as i32);
    if x1 - x0 < (gw * 0.6) as i32 || y1 - y0 < (gh * 0.6) as i32 {
        return guess;
    }
    let back = |v: i32| (v.max(0) as f32 * s).round() as u32;
    Rect::new(back(x0), back(y0), back(x1), back(y1)).clamp(img.w, img.h)
}

/// The luminance at the `lo` and `hi` fractions of the histogram.
pub fn percentiles(img: &Image, lo: f32, hi: f32) -> (u8, u8) {
    let mut hist = [0u32; 256];
    let mut n = 0u32;
    // Every 3rd pixel is plenty for a histogram.
    for p in img.px.chunks_exact(4).step_by(3) {
        if p[3] < 128 {
            continue;
        }
        let l = (p[0] as u32 * 77 + p[1] as u32 * 150 + p[2] as u32 * 29) >> 8;
        hist[l as usize] += 1;
        n += 1;
    }
    if n == 0 {
        return (0, 255);
    }
    let (tl, th) = ((n as f32 * lo) as u32, (n as f32 * hi) as u32);
    let (mut acc, mut a, mut b) = (0u32, 0u8, 255u8);
    let mut found_a = false;
    for (i, c) in hist.iter().enumerate() {
        acc += c;
        if !found_a && acc > tl {
            a = i as u8;
            found_a = true;
        }
        if acc >= th {
            b = i as u8;
            break;
        }
    }
    (a, b)
}
