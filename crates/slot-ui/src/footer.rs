use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_power::Battery;

use crate::battery::{draw_gauge, GAUGE_H};
use crate::plate::HINT_H;
use crate::slot_chrome::MOUTH_H;

const FOOTER_Y: f32 = OUT_H as f32 - MOUTH_H + (MOUTH_H - HINT_H as f32) / 2.0;
const FOOTER_MARGIN: f32 = 24.0;

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Printed {
    pub face: Option<TexId>,
    pub w: u32,
}

impl Printed {
    pub fn new(face: TexId, w: u32) -> Self {
        Printed {
            face: Some(face),
            w,
        }
    }
}

/// How far the Wi-Fi symbol sits from the clock, and how big it is drawn.
const WIFI_GAP: f32 = 10.0;
pub const WIFI_PX: f32 = 18.0;

pub fn draw_footer(
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    wifi: Option<(TexId, u32, u32)>,
    update: Option<(TexId, u32, u32)>,
    clock: Printed,
    out: &mut Vec<Draw>,
) {
    let y = FOOTER_Y + (HINT_H as f32 - GAUGE_H) / 2.0;
    draw_gauge(FOOTER_MARGIN, y, battery, percent, bolt, out);
    let clock_x = OUT_W as f32 - FOOTER_MARGIN - clock.w as f32;
    printed(clock_x, clock, out);
    // To the left of the clock: the two together say what the Brick is doing right now. A
    // newer Slot, when there is one, is told to the left of those.
    let mut right = clock_x;
    for (tex, w, h) in [wifi, update].into_iter().flatten() {
        right -= WIFI_GAP + w as f32;
        out.push(Draw::Tex {
            x: right.round(),
            y: (FOOTER_Y + (HINT_H as f32 - h as f32) / 2.0).round(),
            w: w as f32,
            h: h as f32,
            tex,
            alpha: 1.0,
        });
    }
}

/// How far a note sits from the battery's percent.
const NOTE_GAP: f32 = 22.0;

/// Battery and Wi-Fi at the top right of the system menu. A missing reading draws nothing
/// for that half, and every coordinate stays inside the screen.
pub fn draw_menu_header(
    percent: Option<u8>,
    charging: bool,
    percent_face: Printed,
    bolt: Option<TexId>,
    wifi: Option<u8>,
    out: &mut Vec<Draw>,
) {
    let y = 8.0;
    let h = HINT_H as f32;
    let mut right = OUT_W as f32 - 16.0;
    if let Some(level) = wifi {
        right = draw_wifi(right, y, h, level, out);
        right -= 12.0;
    }
    if percent.is_none() {
        return;
    }
    if percent_face.w > 0 {
        right -= percent_face.w as f32;
        if right >= 8.0 {
            draw_printed(right, y, percent_face, out);
        }
        right -= 8.0;
    }
    let gauge_w = 34.0;
    let gauge_h = 16.0;
    right -= gauge_w + 4.0;
    if right < 8.0 {
        return;
    }
    let colour = battery_colour(percent.unwrap_or(100));
    let gy = y + (h - gauge_h) / 2.0;
    let rect = |x: f32, y: f32, w: f32, h: f32, out: &mut Vec<Draw>| {
        if w <= 0.0
            || h <= 0.0
            || x < 0.0
            || y < 0.0
            || x + w > OUT_W as f32
            || y + h > OUT_H as f32
        {
            return;
        }
        out.push(Draw::Rect { x, y, w, h, colour });
    };
    rect(right, gy, gauge_w, 2.0, out);
    rect(right, gy + gauge_h - 2.0, gauge_w, 2.0, out);
    rect(right, gy, 2.0, gauge_h, out);
    rect(right + gauge_w - 2.0, gy, 2.0, gauge_h, out);
    rect(right + gauge_w, gy + 5.0, 3.0, 6.0, out);
    let fill = (gauge_w - 8.0) * f32::from(percent.unwrap_or(0).min(100)) / 100.0;
    if fill > 0.0 {
        rect(right + 4.0, gy + 4.0, fill, gauge_h - 8.0, out);
    }
    if charging {
        if let Some(tex) = bolt {
            out.push(Draw::Tex {
                x: (right - 16.0).max(8.0),
                y,
                w: 14.0,
                h: 14.0,
                tex,
                alpha: 1.0,
            });
        }
    }
}

/// Above 20% stays the menu ink. Below 20% is yellow. 10% and under is red.
pub fn battery_colour(percent: u8) -> [f32; 4] {
    if percent <= 10 {
        [0.86, 0.22, 0.18, 1.0]
    } else if percent < 20 {
        [0.93, 0.76, 0.22, 1.0]
    } else {
        [
            crate::hud::HUD_INK[0] as f32 / 255.0,
            crate::hud::HUD_INK[1] as f32 / 255.0,
            crate::hud::HUD_INK[2] as f32 / 255.0,
            1.0,
        ]
    }
}

fn draw_wifi(right: f32, y: f32, h: f32, level: u8, out: &mut Vec<Draw>) -> f32 {
    let colour = [
        crate::hud::HUD_INK[0] as f32 / 255.0,
        crate::hud::HUD_INK[1] as f32 / 255.0,
        crate::hud::HUD_INK[2] as f32 / 255.0,
        1.0,
    ];
    let bars = 4u8;
    let bw = 4.0;
    let gap = 3.0;
    let total = bars as f32 * bw + (bars - 1) as f32 * gap;
    let x0 = right - total;
    if x0 < 8.0 {
        return right;
    }
    let base = y + h - 4.0;
    for i in 0..bars {
        let bh = 6.0 + i as f32 * 4.0;
        let on = level > 0 && i < level;
        out.push(Draw::Rect {
            x: x0 + i as f32 * (bw + gap),
            y: base - bh,
            w: bw,
            h: bh,
            colour: if on {
                colour
            } else {
                [colour[0], colour[1], colour[2], 0.28]
            },
        });
    }
    if level == 0 {
        out.push(Draw::Rect {
            x: x0,
            y: y + 4.0,
            w: total,
            h: 2.0,
            colour: [0.86, 0.22, 0.18, 1.0],
        });
    }
    x0
}

/// A note beside the battery's percent (the selected game's play time, on the shelf).
pub fn draw_footer_note(percent: Printed, note: Printed, out: &mut Vec<Draw>) {
    let x = crate::battery::gauge_end(FOOTER_MARGIN, percent) + NOTE_GAP;
    printed(x.round(), note, out);
}

#[cfg(test)]
mod tests {
    use super::battery_colour;

    #[test]
    fn the_battery_is_yellow_below_twenty_and_red_at_ten() {
        assert!(battery_colour(21)[0] > 0.8 && battery_colour(21)[1] > 0.8);
        assert!(battery_colour(20)[0] > 0.8 && battery_colour(20)[1] > 0.8);
        let yellow = battery_colour(19);
        assert!(yellow[0] > 0.8 && yellow[1] > 0.6 && yellow[2] < 0.4);
        let red = battery_colour(10);
        assert!(red[0] > 0.8 && red[1] < 0.4);
        assert!(battery_colour(0)[0] > red[1]);
    }
}

pub(crate) fn draw_printed(x: f32, y: f32, p: Printed, out: &mut Vec<Draw>) {
    if p.w == 0 {
        return;
    }
    let (w, h) = (p.w as f32, HINT_H as f32);
    out.push(match p.face {
        Some(tex) => Draw::Tex {
            x,
            y,
            w,
            h,
            tex,
            alpha: 1.0,
        },
        None => Draw::Rect {
            x,
            y,
            w,
            h,
            colour: [1.0, 1.0, 1.0, 0.08],
        },
    });
}

fn printed(x: f32, p: Printed, out: &mut Vec<Draw>) {
    draw_printed(x, FOOTER_Y, p, out);
}
