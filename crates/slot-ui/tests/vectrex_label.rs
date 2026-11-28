//! The Vectrex shelf fits the overlay, not the whole box cover, into the console's screen.
use slot_cartart::Image;
use slot_store::{Cart, Platform};
use slot_ui::fitted_label_rgba;

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

fn vectrex_cart() -> Cart {
    Cart {
        platform: Platform::Vectrex,
        stem: "Berzerk".into(),
        rom: "Berzerk.vec".into(),
        label: None,
        famicom_label: None,
        title: "Berzerk".into(),
        code: String::new(),
        shell: None,
    }
}

#[test]
fn the_shelf_fits_the_overlay_not_the_whole_cover() {
    let img = box_cover(200, 240, 40, 80, 120, 140);
    let (lw, lh) = (170, 192);
    let out = fitted_label_rgba(&vectrex_cart(), &img.px, img.w, img.h, lw, lh).expect("fit");
    assert_eq!(out.len(), (lw * lh * 4) as usize);
    let mut paper = 0u32;
    let mut ink = 0u32;
    let mut color = 0u32;
    for p in out.chunks_exact(4) {
        let luma = 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
        if luma > 200.0 && (p[0] as i16 - p[1] as i16).unsigned_abs() < 12 {
            paper += 1;
        } else if luma < 40.0 {
            ink += 1;
        } else {
            color += 1;
        }
    }
    let n = lw * lh;
    assert!(
        color * 2 > n,
        "screen is not mostly the overlay: color={color} paper={paper} ink={ink} / {n}"
    );
    assert!(
        paper < n / 5,
        "too much graph paper on the screen: paper={paper} / {n}"
    );
    assert!(
        ink < n / 10,
        "wordmark still on the screen: ink={ink} / {n}"
    );
}
