//! Vectrex box covers yield the overlay illustration, not the whole cover.
use slot_cartart::{vectrex_overlay, vectrex_overlay_bounds, Image, Rect};

/// A light graph-paper field with a dark wordmark at the top and a colourful overlay panel.
fn box_cover(w: u32, h: u32, ox: u32, oy: u32, ow: u32, oh: u32) -> Image {
    let mut img = Image::new(w, h, [221, 221, 221, 255]);
    for y in 12..36 {
        for x in 20..w.saturating_sub(20) {
            img.put(x, y, [20, 20, 20, 255]);
        }
    }
    for y in oy..oy + oh {
        for x in ox..ox + ow {
            let p = if (x + y) % 2 == 0 {
                [200, 30, 30, 255]
            } else {
                [30, 40, 200, 255]
            };
            img.put(x, y, p);
        }
    }
    img
}

fn contains(r: Rect, x: u32, y: u32, w: u32, h: u32) {
    assert!(
        r.x0 <= x + 4 && r.y0 <= y + 4 && r.x1 + 4 >= x + w && r.y1 + 4 >= y + h,
        "overlay {w}x{h} at ({x},{y}) not inside {r:?}"
    );
    assert!(r.y0 > 36, "crop includes the wordmark: {r:?}");
}

#[test]
fn a_vectrex_box_yields_the_overlay_panel() {
    let (ox, oy, ow, oh) = (40, 80, 120, 140);
    let img = box_cover(200, 240, ox, oy, ow, oh);
    let r = vectrex_overlay_bounds(&img.px, img.w, img.h).expect("overlay");
    contains(r, ox, oy, ow, oh);
    let cut = vectrex_overlay(&img).expect("crop");
    assert!(cut.w >= ow - 4 && cut.h >= oh - 4);
    let paper = cut
        .px
        .chunks_exact(4)
        .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
        .count();
    assert!(
        paper < (cut.w * cut.h) as usize / 5,
        "crop is still mostly paper: {paper} / {}",
        cut.w * cut.h
    );
}

#[test]
fn a_pillarboxed_box_still_yields_the_overlay() {
    // Mid-grey bars beside a tall box, as on some unfitted Labels/VECTREX scans.
    let inner = box_cover(200, 240, 40, 80, 120, 140);
    let mut img = Image::new(280, 240, [122, 122, 122, 255]);
    img.blit(&inner, 40, 0);
    let r = vectrex_overlay_bounds(&img.px, img.w, img.h).expect("overlay");
    contains(r, 40 + 40, 80, 120, 140);
}

#[test]
fn an_already_cut_overlay_is_left_whole() {
    let mut img = Image::new(170, 192, [20, 30, 80, 255]);
    for y in 0..img.h {
        for x in 0..img.w {
            let p = if (x + y) % 3 == 0 {
                [200, 30, 30, 255]
            } else {
                [30, 40, 180, 255]
            };
            img.put(x, y, p);
        }
    }
    assert!(
        vectrex_overlay_bounds(&img.px, img.w, img.h).is_none(),
        "must not recrop an overlay that already fills the window"
    );
}

#[test]
fn a_dark_logo_plate_is_left_whole() {
    let mut img = Image::new(170, 192, [16, 18, 48, 255]);
    for y in 80..110 {
        for x in 30..140 {
            img.put(x, y, [240, 240, 250, 255]);
        }
    }
    assert!(vectrex_overlay_bounds(&img.px, img.w, img.h).is_none());
}
