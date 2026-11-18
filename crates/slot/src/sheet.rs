//! A full-screen menu drawn the way the Wi-Fi screen is: Slot's plate, a bright bar
//! on the selected row, and the label font.
use slot_gfx::{OUT_H, OUT_W};
use slot_ui::{text, UndoFace};

const INK: [u8; 3] = [0xf6, 0xf4, 0xef];
const DIM: [u8; 3] = [0x9a, 0x9a, 0xa4];

pub struct Sheet {
    rgba: Vec<u8>,
    w: u32,
    h: u32,
}

impl Sheet {
    pub fn new() -> Self {
        let (w, h) = (OUT_W, OUT_H);
        let mut sheet = Sheet {
            rgba: vec![0; (w * h * 4) as usize],
            w,
            h,
        };
        let bg = slot_ui::opening();
        sheet.rect(0, 0, w, h, bg);
        sheet
    }

    pub fn rect(&mut self, x: u32, y: u32, w: u32, h: u32, colour: [f32; 4]) {
        let c = colour.map(|v| (v * 255.0) as u8);
        let x1 = x.saturating_add(w).min(self.w);
        let y1 = y.saturating_add(h).min(self.h);
        for yy in y..y1 {
            for xx in x..x1 {
                let at = ((yy * self.w + xx) * 4) as usize;
                self.rgba[at..at + 4].copy_from_slice(&c);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn text(&mut self, words: &str, x: u32, y: u32, w: u32, h: u32, px: f32, dim: bool) {
        self.text_case(words, x, y, w, h, px, dim, true);
    }

    /// `upper` is how the rest of Slot prints its menus. A typed search keeps its case.
    #[allow(clippy::too_many_arguments)]
    pub fn text_case(
        &mut self,
        words: &str,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
        px: f32,
        dim: bool,
        upper: bool,
    ) {
        let Some(font) = text::label_font() else {
            return;
        };
        if w == 0 || h == 0 || x >= self.w || y >= self.h {
            return;
        }
        let mut words = if upper {
            words.to_uppercase()
        } else {
            words.to_string()
        };
        // The HOME menu's type is 30px. Long lines shrink toward 18px before any letters are cut.
        let mut size = px;
        let min = 18.0_f32.min(px);
        while size > min && text::line_width(font, &words, size, 0.0) > w as f32 {
            size -= 1.0;
        }
        if text::line_width(font, &words, size, 0.0) > w as f32 {
            // Drop whole characters. A byte-length check would cut a multibyte glyph in half,
            // and the ellipsis itself has to fit, including when the last glyph is wide.
            let mut chars: Vec<char> = words.chars().collect();
            let mut fitted = String::new();
            while chars.len() > 1 {
                chars.pop();
                fitted = chars.iter().collect::<String>();
                fitted.push('…');
                if text::line_width(font, &fitted, size, 0.0) <= w as f32 {
                    break;
                }
                fitted.clear();
            }
            if fitted.is_empty() {
                words = chars.into_iter().collect();
            } else {
                words = fitted;
            }
        }
        if words.is_empty() {
            return;
        }
        let layout = text::Layout {
            lines: vec![words],
            px: size,
            tracking: 0.0,
        };
        let mut face = vec![0u8; (w * h * 4) as usize];
        text::draw_centred(&mut face, w, h, &layout, if dim { DIM } else { INK });
        self.blit(x, y, w, h, &face);
    }

    pub fn image(&mut self, x: u32, y: u32, w: u32, h: u32, rgba: &[u8]) {
        self.blit(x, y, w, h, rgba);
    }

    fn blit(&mut self, x: u32, y: u32, w: u32, h: u32, face: &[u8]) {
        if w == 0 || h == 0 || x >= self.w || y >= self.h || face.len() < 4 {
            return;
        }
        let rows = h.min(self.h.saturating_sub(y));
        let cols = w.min(self.w.saturating_sub(x));
        // A 56-wide picture asked for at 64 used to stride by 64 and read past the
        // buffer (7168 bytes, index 7171). The row width is whatever actually fits.
        let stride = Self::source_stride(face.len(), w, h);
        for yy in 0..rows {
            let row = (yy as usize).saturating_mul(stride);
            if row >= face.len() {
                break;
            }
            let avail = (face.len() - row) / 4;
            let cols = cols.min(avail as u32);
            for xx in 0..cols {
                let src = row + (xx as usize) * 4;
                if src + 3 >= face.len() {
                    break;
                }
                let dst = (((y + yy) * self.w + x + xx) * 4) as usize;
                if dst + 3 >= self.rgba.len() {
                    break;
                }
                let alpha = face[src + 3] as u32;
                if alpha == 0 {
                    continue;
                }
                for ch in 0..3 {
                    self.rgba[dst + ch] = ((face[src + ch] as u32 * alpha
                        + self.rgba[dst + ch] as u32 * (255 - alpha))
                        / 255) as u8;
                }
                self.rgba[dst + 3] = 255;
            }
        }
    }

    /// Bytes from one source row to the next. A full `w` by `h` picture uses `w`.
    /// A shorter buffer (56 pixels wide, drawn as 64) uses the width that divides it.
    fn source_stride(len: usize, w: u32, h: u32) -> usize {
        let pixels = len / 4;
        let asked = (w as usize).saturating_mul(h as usize);
        if w > 0 && pixels >= asked {
            return (w as usize).saturating_mul(4);
        }
        if h > 0 {
            let row = pixels / h as usize;
            if row > 0 {
                return row.saturating_mul(4);
            }
        }
        (w as usize).saturating_mul(4).max(4)
    }

    pub fn finish(self) -> UndoFace {
        UndoFace {
            rgba: self.rgba,
            w: self.w,
            h: self.h,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_56_wide_picture_drawn_as_64_does_not_read_past_byte_7168() {
        // The channel row asked for 64×32 of a 56×32 thumbnail. 56*32*4 = 7168,
        // and striding by 64 put the next row's alpha at index 7171.
        let mut sheet = Sheet::new();
        let px = vec![255u8; 56 * 32 * 4];
        assert_eq!(px.len(), 7168);
        let src = ((28 * 64 + 0) * 4) as usize;
        assert_eq!(src + 3, 7171);
        assert!(src + 3 >= px.len());
        sheet.image(12, 100, 64, 32, &px);
        let face = sheet.finish();
        assert_eq!(face.rgba.len(), (OUT_W * OUT_H * 4) as usize);
        assert!(face.rgba.iter().any(|b| *b == 255));
    }

    #[test]
    fn a_short_last_row_and_a_wide_glyph_are_clipped() {
        let mut sheet = Sheet::new();
        // 7 pixels, not a whole row of the width we ask for.
        sheet.image(0, 0, 4, 3, &[255u8; 7 * 4]);
        sheet.text(
            "動画のタイトルがとても長いです：éééééééééééééééééééééééééééééé",
            16,
            40,
            48,
            40,
            30.0,
            false,
        );
        let face = sheet.finish();
        assert_eq!(face.w, OUT_W);
    }
}
