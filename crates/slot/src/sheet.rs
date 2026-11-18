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
            while text::line_width(font, &words, size, 0.0) > w as f32 && words.len() > 1 {
                words.pop();
            }
            if !words.is_empty() {
                words.pop();
                words.push('…');
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
        let rows = h.min(self.h.saturating_sub(y));
        let cols = w.min(self.w.saturating_sub(x));
        for yy in 0..rows {
            for xx in 0..cols {
                let src = ((yy * w + xx) * 4) as usize;
                let dst = (((y + yy) * self.w + x + xx) * 4) as usize;
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

    pub fn finish(self) -> UndoFace {
        UndoFace {
            rgba: self.rgba,
            w: self.w,
            h: self.h,
        }
    }
}
