use slot_gfx::{Draw, TexId, OUT_H, OUT_W};
use slot_power::Battery;

use crate::battery::{draw_gauge, draw_gauge_styled, GAUGE_H};
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
    paint_corner(FOOTER_Y, wifi, update, clock, out);
}

/// The shelf's status row, at the top of the system menu: battery and percent on the left,
/// Wi-Fi and the clock on the right. The gauge is yellow below 20% and red at 10%.
pub fn draw_menu_header(
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    wifi: Option<(TexId, u32, u32)>,
    clock: Printed,
    out: &mut Vec<Draw>,
) {
    let y = 8.0 + (HINT_H as f32 - GAUGE_H) / 2.0;
    let colour = battery
        .map(|b| battery_colour(b.percent))
        .unwrap_or(crate::battery::ink());
    // The shelf's charging mark. It is taller than the battery outline, and only while charging.
    draw_gauge_styled(
        FOOTER_MARGIN,
        y,
        battery,
        percent,
        bolt,
        colour,
        crate::battery::BOLT_H,
        out,
    );
    paint_corner(8.0, wifi, None, clock, out);
}

fn paint_corner(
    row_y: f32,
    wifi: Option<(TexId, u32, u32)>,
    update: Option<(TexId, u32, u32)>,
    clock: Printed,
    out: &mut Vec<Draw>,
) {
    let clock_x = OUT_W as f32 - FOOTER_MARGIN - clock.w as f32;
    draw_printed(clock_x, row_y, clock, out);
    let mut right = clock_x;
    for (tex, w, h) in [wifi, update].into_iter().flatten() {
        right -= WIFI_GAP + w as f32;
        out.push(Draw::Tex {
            x: right.round(),
            y: (row_y + (HINT_H as f32 - h as f32) / 2.0).round(),
            w: w as f32,
            h: h as f32,
            tex,
            alpha: 1.0,
        });
    }
}

/// How far a note sits from the battery's percent.
const NOTE_GAP: f32 = 22.0;

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
