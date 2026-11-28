//! The fitter on synthetic pictures: each step does what it says, on shapes whose answer is known.
use slot_cartart::analyze::{foreground, refine_label, skew_degrees};
use slot_cartart::image::{resize, rotate};
use slot_cartart::{fit, fit_tier, Image, Kind, Rect, Source};

/// A busy "print": stripes and a gradient, so it has detail and no flat edge.
fn print(w: u32, h: u32) -> Image {
    let mut img = Image::new(w, h, [0, 0, 0, 255]);
    for y in 0..h {
        for x in 0..w {
            let v = ((x * 7 + y * 3) % 64) as u8 * 3;
            img.put(x, y, [v, 255 - v, (x * 255 / w) as u8, 255]);
        }
    }
    img
}

/// `inner` laid on a canvas of `bg`, with `pad` all round.
fn on(inner: &Image, pad: u32, bg: [u8; 4]) -> Image {
    let mut c = Image::new(inner.w + 2 * pad, inner.h + 2 * pad, bg);
    c.blit(inner, pad as i32, pad as i32);
    c
}

#[test]
fn resize_keeps_flat_colour_flat_and_hits_the_size() {
    let img = Image::new(300, 200, [40, 120, 200, 255]);
    for (w, h) in [(150, 100), (601, 401), (37, 91)] {
        let r = resize(&img, w, h);
        assert_eq!((r.w, r.h), (w, h));
        assert!(
            r.px.chunks_exact(4).all(|p| p == [40, 120, 200, 255]),
            "{w}x{h}"
        );
    }
}

#[test]
fn resize_does_not_bleed_transparent_colour() {
    // Opaque red next to fully transparent green: the green must never show.
    let mut img = Image::new(40, 40, [0, 255, 0, 0]);
    for y in 0..40 {
        for x in 0..20 {
            img.put(x, y, [255, 0, 0, 255]);
        }
    }
    let r = resize(&img, 17, 17);
    for p in r.px.chunks_exact(4) {
        if p[3] > 0 {
            assert!(p[1] < 8, "green bled in: {p:?}");
        }
    }
}

#[test]
fn foreground_floods_a_flat_backdrop_away() {
    let img = on(&print(200, 120), 30, [250, 250, 250, 255]);
    let m = foreground(&img);
    assert!(m.bg.is_some());
    let bb = m.bbox().unwrap();
    for (got, want) in [(bb.x0, 30), (bb.y0, 30), (bb.x1, 230), (bb.y1, 150)] {
        assert!(got.abs_diff(want) <= 3, "{bb:?}");
    }
}

#[test]
fn foreground_uses_transparency_when_there_is_some() {
    let img = on(&print(100, 100), 20, [0, 0, 0, 0]);
    let m = foreground(&img);
    assert!(m.bg.is_none());
    let bb = m.bbox().unwrap();
    assert!(
        bb.x0.abs_diff(20) <= 2 && bb.x1.abs_diff(120) <= 2,
        "{bb:?}"
    );
}

#[test]
fn a_picture_that_runs_to_its_edge_is_left_whole() {
    let m = foreground(&print(300, 200));
    assert_eq!(m.bbox().unwrap(), Rect::new(0, 0, 300, 200));
}

#[test]
fn skew_is_measured_and_square_scans_are_left_alone() {
    let flat = on(
        &Image::new(300, 200, [200, 30, 30, 255]),
        60,
        [255, 255, 255, 0],
    );
    assert_eq!(skew_degrees(&foreground(&flat)), None);
    for deg in [2.5f32, -4.0] {
        let turned = rotate(&flat, deg);
        let got = skew_degrees(&foreground(&turned)).expect("a turn");
        assert!((got - deg).abs() < 0.6, "turned {deg}, measured {got}");
    }
}

#[test]
fn a_turned_label_scan_comes_out_straight() {
    // A red label with a white frame, turned 3 degrees on transparency.
    let mut label = Image::new(300, 200, [255, 255, 255, 255]);
    for y in 10..190 {
        for x in 10..290 {
            label.put(x, y, [200, 20, 20, 255]);
        }
    }
    let turned = rotate(&on(&label, 50, [0, 0, 0, 0]), 3.0);
    let src = [Source {
        kind: Kind::Texture,
        img: turned,
    }];
    let f = fit("MD", 150, 100, 2, &src).expect("fitted");
    assert!(f.how.contains("straightened"), "{}", f.how);
    // Straightened, the corners are label (white frame), not transparent black.
    let c = f.img.get(4, 4);
    assert!(c[0] > 150 && c[1] > 150, "corner {c:?}");
}

#[test]
fn the_label_edge_is_found_on_a_cart_photo() {
    // Grey cart, a blue label a little off from where it is expected.
    let mut cart = Image::new(400, 400, [128, 128, 128, 255]);
    for y in 60..330 {
        for x in 70..330 {
            cart.put(x, y, [20, 40, 220, 255]);
        }
    }
    let guess = Rect::new(60, 50, 340, 340);
    let r = refine_label(&cart, guess, Rect::new(5, 100, 30, 300), 0.06);
    for (got, want) in [(r.x0, 70), (r.y0, 60), (r.x1, 330), (r.y1, 330)] {
        assert!(got.abs_diff(want) <= 4, "{r:?}");
    }
}

#[test]
fn every_tier_fills_the_window_exactly() {
    let sources = vec![
        Source {
            kind: Kind::Texture,
            img: on(&print(240, 196), 12, [255, 255, 255, 255]),
        },
        Source {
            kind: Kind::Photo,
            img: on(&print(360, 245), 0, [0, 0, 0, 0]),
        },
        Source {
            kind: Kind::Box,
            img: print(240, 340),
        },
        Source {
            kind: Kind::Logo,
            img: on(&print(200, 80), 20, [0, 0, 0, 0]),
        },
        Source {
            kind: Kind::Marquee,
            img: print(400, 120),
        },
    ];
    for dir in slot_cartart::SYSTEMS {
        for kind in [
            Kind::Texture,
            Kind::Photo,
            Kind::Box,
            Kind::Logo,
            Kind::Marquee,
        ] {
            if let Some(f) = fit_tier(dir, 120, 98, 2, &sources, kind) {
                assert_eq!((f.img.w, f.img.h), (240, 196), "{dir} {kind:?}");
                // Only a whole-object label (a Game & Watch handheld) keeps transparency.
                if !f.transparent {
                    assert!(
                        f.img.px.chunks_exact(4).all(|p| p[3] == 255),
                        "{dir} {kind:?} not opaque"
                    );
                }
                assert_eq!(
                    f.transparent,
                    slot_cartart::profile(dir).whole_object
                        && matches!(kind, Kind::Photo | Kind::Texture)
                );
            }
        }
    }
}

#[test]
fn the_best_picture_wins_and_a_missing_one_falls_through() {
    let tex = Source {
        kind: Kind::Texture,
        img: print(240, 196),
    };
    let bx = Source {
        kind: Kind::Box,
        img: print(240, 340),
    };
    let logo = Source {
        kind: Kind::Logo,
        img: on(&print(200, 80), 20, [0, 0, 0, 0]),
    };
    assert_eq!(
        fit("MD", 240, 196, 1, &[bx, tex]).unwrap().tier,
        Kind::Texture
    );
    let bx = Source {
        kind: Kind::Box,
        img: print(240, 340),
    };
    assert_eq!(fit("MD", 240, 196, 1, &[logo, bx]).unwrap().tier, Kind::Box);
    let logo = Source {
        kind: Kind::Logo,
        img: on(&print(200, 80), 20, [0, 0, 0, 0]),
    };
    assert_eq!(fit("MD", 240, 196, 1, &[logo]).unwrap().tier, Kind::Logo);
    assert!(fit("MD", 240, 196, 1, &[]).is_none());
}

#[test]
fn a_label_scan_of_the_wrong_shape_is_passed_over() {
    // A long thin strip (an end label) is not the front label of a Genesis cart.
    let strip = Source {
        kind: Kind::Texture,
        img: print(600, 80),
    };
    let bx = Source {
        kind: Kind::Box,
        img: print(240, 340),
    };
    let f = fit("MD", 240, 196, 1, &[strip, bx]).unwrap();
    assert_eq!(f.tier, Kind::Box);
    assert!(f.how.contains("passed over: texture"), "{}", f.how);
}

#[test]
fn png_round_trip() {
    let img = print(64, 48);
    let png = slot_cartart::encode_png_rgb(&img);
    let back = slot_cartart::decode_png(&png).unwrap();
    assert_eq!(back, img);
}
