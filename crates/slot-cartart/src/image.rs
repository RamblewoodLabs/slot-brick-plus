//! A small RGBA8 picture and the operations the fitter needs: decode, crop, filtered
//! resize, rotate, blur and encode. Everything is integer- or f32-only and allocation-light,
//! so it runs in a few tens of milliseconds on a Cortex-A53.

/// Straight (not premultiplied) RGBA8, row-major.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

impl Image {
    pub fn new(w: u32, h: u32, fill: [u8; 4]) -> Self {
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..w * h {
            px.extend_from_slice(&fill);
        }
        Image { w, h, px }
    }

    pub fn from_rgba(w: u32, h: u32, px: Vec<u8>) -> Option<Self> {
        (w > 0 && h > 0 && px.len() == (w as usize) * (h as usize) * 4).then_some(Image {
            w,
            h,
            px,
        })
    }

    #[inline]
    pub fn get(&self, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * self.w + x) * 4) as usize;
        [self.px[i], self.px[i + 1], self.px[i + 2], self.px[i + 3]]
    }

    #[inline]
    pub fn put(&mut self, x: u32, y: u32, p: [u8; 4]) {
        let i = ((y * self.w + x) * 4) as usize;
        self.px[i..i + 4].copy_from_slice(&p);
    }

    pub fn aspect(&self) -> f32 {
        self.w as f32 / self.h as f32
    }

    /// `x0, y0, x1, y1` in pixels, clamped to the picture.
    pub fn crop(&self, r: Rect) -> Image {
        let r = r.clamp(self.w, self.h);
        let (w, h) = (r.w().max(1), r.h().max(1));
        let mut px = Vec::with_capacity((w * h * 4) as usize);
        for y in r.y0..r.y0 + h {
            let a = ((y * self.w + r.x0) * 4) as usize;
            px.extend_from_slice(&self.px[a..a + (w * 4) as usize]);
        }
        Image { w, h, px }
    }

    /// The picture laid on a solid colour: no transparency left.
    pub fn flatten(&self, bg: [u8; 3]) -> Image {
        let mut out = self.clone();
        for p in out.px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            if a == 255 {
                continue;
            }
            for c in 0..3 {
                p[c] = ((p[c] as u32 * a + bg[c] as u32 * (255 - a) + 127) / 255) as u8;
            }
            p[3] = 255;
        }
        out
    }

    pub fn has_alpha(&self) -> bool {
        self.px.chunks_exact(4).any(|p| p[3] < 250)
    }

    /// Mean alpha, 0..=255.
    pub fn mean_alpha(&self) -> f32 {
        let s: u64 = self.px.chunks_exact(4).map(|p| p[3] as u64).sum();
        s as f32 / (self.w * self.h) as f32
    }

    /// Paste `src` (straight alpha, composited over) with its top-left at `x, y`.
    pub fn blit(&mut self, src: &Image, x: i32, y: i32) {
        for sy in 0..src.h {
            let dy = y + sy as i32;
            if dy < 0 || dy >= self.h as i32 {
                continue;
            }
            for sx in 0..src.w {
                let dx = x + sx as i32;
                if dx < 0 || dx >= self.w as i32 {
                    continue;
                }
                let s = src.get(sx, sy);
                let a = s[3] as u32;
                if a == 0 {
                    continue;
                }
                let d = self.get(dx as u32, dy as u32);
                let out = if a == 255 {
                    s
                } else {
                    let mut o = [0u8; 4];
                    for c in 0..3 {
                        o[c] = ((s[c] as u32 * a + d[c] as u32 * (255 - a) + 127) / 255) as u8;
                    }
                    o[3] = (a + d[3] as u32 * (255 - a) / 255).min(255) as u8;
                    o
                };
                self.put(dx as u32, dy as u32, out);
            }
        }
    }
}

/// A rectangle in pixels, half-open: `x0..x1`, `y0..y1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Rect {
    pub fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Self {
        Rect { x0, y0, x1, y1 }
    }
    pub fn w(&self) -> u32 {
        self.x1.saturating_sub(self.x0)
    }
    pub fn h(&self) -> u32 {
        self.y1.saturating_sub(self.y0)
    }
    pub fn clamp(&self, w: u32, h: u32) -> Rect {
        let x0 = self.x0.min(w.saturating_sub(1));
        let y0 = self.y0.min(h.saturating_sub(1));
        Rect {
            x0,
            y0,
            x1: self.x1.clamp(x0 + 1, w),
            y1: self.y1.clamp(y0 + 1, h),
        }
    }
    /// Fractions `[x0, y0, x1, y1]` of this rectangle, as a rectangle in the same pixels.
    pub fn sub(&self, f: [f32; 4]) -> Rect {
        let (w, h) = (self.w() as f32, self.h() as f32);
        Rect {
            x0: self.x0 + (f[0] * w).round().max(0.0) as u32,
            y0: self.y0 + (f[1] * h).round().max(0.0) as u32,
            x1: self.x0 + (f[2] * w).round().max(0.0) as u32,
            y1: self.y0 + (f[3] * h).round().max(0.0) as u32,
        }
    }
}

// ---------------------------------------------------------------- decode / encode

/// PNG bytes to RGBA8 (palette, grey and 16-bit are expanded). `None` for anything else.
pub fn decode_png(bytes: &[u8]) -> Option<Image> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    dec.set_limits(png::Limits { bytes: 96 << 20 });
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let src = &buf[..info.buffer_size()];
    let px: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => src
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => src
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Indexed => return None,
    };
    Image::from_rgba(info.width, info.height, px)
}

/// Opaque RGB PNG (a label has no transparency): smaller on the card than RGBA.
pub fn encode_png_rgb(img: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.w, img.h);
        enc.set_color(png::ColorType::Rgb);
        enc.set_depth(png::BitDepth::Eight);
        // Fast deflate with the Paeth filter: measured over the 74 test labels, 23 times
        // quicker to write than Default for files 4.6% larger (Best saves 0.6% at 50 times).
        enc.set_compression(png::Compression::Fast);
        enc.set_filter(png::FilterType::Paeth);
        enc.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);
        let rgb: Vec<u8> = img
            .px
            .chunks_exact(4)
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(&rgb).expect("png data");
    }
    out
}

pub fn encode_png_rgba(img: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.w, img.h);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        enc.set_filter(png::FilterType::Paeth);
        enc.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);
        let mut w = enc.write_header().expect("png header");
        w.write_image_data(&img.px).expect("png data");
    }
    out
}

// ---------------------------------------------------------------- resize

/// Catmull-Rom, the sharpest of the cubic filters that does not ring much.
fn cubic(x: f32) -> f32 {
    let x = x.abs();
    if x < 1.0 {
        1.5 * x * x * x - 2.5 * x * x + 1.0
    } else if x < 2.0 {
        -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
    } else {
        0.0
    }
}

/// For each output sample along one axis: the first source index and its weights (16.16 fixed).
fn weights(src: u32, dst: u32) -> Vec<(u32, Vec<i32>)> {
    let scale = dst as f32 / src as f32;
    // Shrinking widens the filter so every source pixel counts (no aliasing).
    let support = if scale < 1.0 { 2.0 / scale } else { 2.0 };
    let fscale = if scale < 1.0 { scale } else { 1.0 };
    (0..dst)
        .map(|o| {
            let centre = (o as f32 + 0.5) / scale;
            let a = ((centre - support).floor().max(0.0)) as u32;
            let b = ((centre + support).ceil() as u32).min(src);
            let mut ws: Vec<f32> = (a..b)
                .map(|i| cubic((i as f32 + 0.5 - centre) * fscale))
                .collect();
            let sum: f32 = ws.iter().sum();
            if sum.abs() > 1e-6 {
                ws.iter_mut().for_each(|w| *w /= sum);
            }
            let mut fixed: Vec<i32> = ws.iter().map(|w| (w * 65536.0).round() as i32).collect();
            // Exact unit gain, so flat colour stays flat.
            let err = 65536 - fixed.iter().sum::<i32>();
            if let Some(m) = fixed.iter_mut().max_by_key(|v| **v) {
                *m += err;
            }
            (a, fixed)
        })
        .collect()
}

/// Filtered resize (premultiplied, separable Catmull-Rom). Good both ways: shrinking averages
/// every pixel, enlarging stays crisp without the blocks of nearest or the blur of bilinear.
pub fn resize(img: &Image, w: u32, h: u32) -> Image {
    let (w, h) = (w.max(1), h.max(1));
    if (w, h) == (img.w, img.h) {
        return img.clone();
    }
    // Premultiply into i32 so transparent pixels don't bleed their colour.
    let n = (img.w * img.h) as usize;
    let mut pre = vec![0i32; n * 4];
    for (i, p) in img.px.chunks_exact(4).enumerate() {
        let a = p[3] as i32;
        pre[i * 4] = p[0] as i32 * a / 255;
        pre[i * 4 + 1] = p[1] as i32 * a / 255;
        pre[i * 4 + 2] = p[2] as i32 * a / 255;
        pre[i * 4 + 3] = a;
    }
    // Horizontal: img.w -> w, rows stay img.h.
    let wx = weights(img.w, w);
    let mut mid = vec![0i32; (w * img.h * 4) as usize];
    for y in 0..img.h as usize {
        let row = &pre[y * img.w as usize * 4..(y + 1) * img.w as usize * 4];
        let out = &mut mid[y * w as usize * 4..(y + 1) * w as usize * 4];
        for (x, (a, ws)) in wx.iter().enumerate() {
            let mut acc = [0i64; 4];
            for (k, wt) in ws.iter().enumerate() {
                let s = (*a as usize + k) * 4;
                let wt = *wt as i64;
                acc[0] += row[s] as i64 * wt;
                acc[1] += row[s + 1] as i64 * wt;
                acc[2] += row[s + 2] as i64 * wt;
                acc[3] += row[s + 3] as i64 * wt;
            }
            for c in 0..4 {
                out[x * 4 + c] = ((acc[c] + 32768) >> 16) as i32;
            }
        }
    }
    // Vertical: img.h -> h.
    let wy = weights(img.h, h);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let stride = (w * 4) as usize;
    let mut acc = vec![0i64; stride];
    for (y, (a, ws)) in wy.iter().enumerate() {
        acc.iter_mut().for_each(|v| *v = 0);
        for (k, wt) in ws.iter().enumerate() {
            let row = &mid[(*a as usize + k) * stride..(*a as usize + k + 1) * stride];
            let wt = *wt as i64;
            for (d, s) in acc.iter_mut().zip(row) {
                *d += *s as i64 * wt;
            }
        }
        let out = &mut px[y * stride..(y + 1) * stride];
        for x in 0..w as usize {
            let al = ((acc[x * 4 + 3] + 32768) >> 16).clamp(0, 255);
            out[x * 4 + 3] = al as u8;
            for c in 0..3 {
                let v = (acc[x * 4 + c] + 32768) >> 16;
                let v = if al > 0 { v * 255 / al } else { 0 };
                out[x * 4 + c] = v.clamp(0, 255) as u8;
            }
        }
    }
    Image { w, h, px }
}

// ---------------------------------------------------------------- rotate / blur / sharpen

/// Rotated `deg` clockwise about the centre, same size, bilinear; corners come in transparent.
pub fn rotate(img: &Image, deg: f32) -> Image {
    let (s, c) = deg.to_radians().sin_cos();
    let (cx, cy) = (img.w as f32 / 2.0, img.h as f32 / 2.0);
    let mut out = Image::new(img.w, img.h, [0, 0, 0, 0]);
    for y in 0..img.h {
        for x in 0..img.w {
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            // Inverse map: rotate the output pixel back by -deg.
            let sx = c * dx + s * dy + cx - 0.5;
            let sy = -s * dx + c * dy + cy - 0.5;
            out.put(x, y, bilinear(img, sx, sy));
        }
    }
    out
}

pub fn bilinear(img: &Image, x: f32, y: f32) -> [u8; 4] {
    if x < -1.0 || y < -1.0 || x > img.w as f32 || y > img.h as f32 {
        return [0, 0, 0, 0];
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let mut acc = [0f32; 4];
    for (ox, oy, wt) in [
        (0, 0, (1.0 - fx) * (1.0 - fy)),
        (1, 0, fx * (1.0 - fy)),
        (0, 1, (1.0 - fx) * fy),
        (1, 1, fx * fy),
    ] {
        let (px, py) = (x0 as i32 + ox, y0 as i32 + oy);
        if px < 0 || py < 0 || px >= img.w as i32 || py >= img.h as i32 {
            continue;
        }
        let p = img.get(px as u32, py as u32);
        let a = p[3] as f32 * wt;
        acc[0] += p[0] as f32 * a;
        acc[1] += p[1] as f32 * a;
        acc[2] += p[2] as f32 * a;
        acc[3] += a;
    }
    if acc[3] <= 0.0 {
        return [0, 0, 0, 0];
    }
    [
        (acc[0] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[1] / acc[3]).round().clamp(0.0, 255.0) as u8,
        (acc[2] / acc[3]).round().clamp(0.0, 255.0) as u8,
        acc[3].round().clamp(0.0, 255.0) as u8,
    ]
}

/// A soft blur, cheaply: shrink a lot, then grow back with the cubic filter.
pub fn soft_blur(img: &Image, factor: u32) -> Image {
    let f = factor.max(2);
    let small = resize(img, (img.w / f).max(2), (img.h / f).max(2));
    let small = resize(&small, (small.w / 2).max(2), (small.h / 2).max(2));
    resize(&small, img.w, img.h)
}

/// Unsharp mask on an opaque picture: `amount` of the difference from a 3x3 blur is added,
/// ignoring differences below `threshold` (so flat colour and JPEG noise stay put).
pub fn sharpen(img: &mut Image, amount: f32, threshold: i32) {
    if amount <= 0.0 || img.w < 3 || img.h < 3 {
        return;
    }
    let src = img.px.clone();
    let (w, h) = (img.w as usize, img.h as usize);
    let k = (amount * 256.0) as i32;
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            for c in 0..3 {
                let at = |xx: usize, yy: usize| src[(yy * w + xx) * 4 + c] as i32;
                let centre = at(x, y);
                let blur = (at(x - 1, y - 1)
                    + 2 * at(x, y - 1)
                    + at(x + 1, y - 1)
                    + 2 * at(x - 1, y)
                    + 4 * centre
                    + 2 * at(x + 1, y)
                    + at(x - 1, y + 1)
                    + 2 * at(x, y + 1)
                    + at(x + 1, y + 1))
                    / 16;
                let d = centre - blur;
                if d.abs() < threshold {
                    continue;
                }
                img.px[(y * w + x) * 4 + c] = (centre + ((d * k) >> 8)).clamp(0, 255) as u8;
            }
        }
    }
}
