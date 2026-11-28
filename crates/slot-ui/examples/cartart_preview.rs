//! Runs the cart-art engine over a folder of ScreenScraper downloads and draws each result on
//! its real cart, for checking by eye and timing (test tooling; not on the card).
//!
//!     cargo run --release -p slot-ui --example cartart_preview -- ROOT [SCALE]
//!
//! ROOT/<DIR>/raw/<game>/<media type>.png in; ROOT/<DIR>/fitted/<game>.label.png (the label),
//! <game>.cart.png (on the shelf's cart) and <game>.plate.png (the cart with no art, as now)
//! out, plus ROOT/results.tsv.
use std::path::{Path, PathBuf};
use std::time::Instant;

use slot_cartart::{Kind, Source};
use slot_store::{Cart, Platform};

fn kind_of(name: &str) -> Option<(Kind, u8)> {
    // (kind, preference within kind: lower wins)
    Some(match name {
        "support-texture" => (Kind::Texture, 0),
        "support-2D" => (Kind::Photo, 0),
        "box-2D" => (Kind::Box, 0),
        "wheel-hd" => (Kind::Logo, 0),
        "wheel" => (Kind::Logo, 1),
        "screenmarquee" => (Kind::Marquee, 1),
        "marquee" => (Kind::Marquee, 0),
        _ => return None,
    })
}

fn save(path: &Path, rgba: &[u8], w: u32, h: u32) {
    let img = slot_cartart::Image::from_rgba(w, h, rgba.to_vec()).expect("size");
    std::fs::write(path, slot_cartart::image::encode_png_rgba(&img)).expect("write");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let scale: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(2);
    let only: Option<Vec<String>> = args
        .get(3)
        .map(|s| s.split(',').map(String::from).collect());
    let mut tiers = String::new();
    let mut report = String::from("system\tgame\ttier\thow\twindow\tout\tdecode_ms\tfit_ms\tencode_ms\tlabel_bytes\traw_bytes\n");
    for platform in Platform::ALL {
        let dir = platform.dir_name();
        if only.as_ref().is_some_and(|o| !o.iter().any(|d| d == dir)) {
            continue;
        }
        let raw = root.join(dir).join("raw");
        let Ok(games) = std::fs::read_dir(&raw) else {
            continue;
        };
        let mut games: Vec<PathBuf> = games
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        games.sort();
        let out = root.join(dir).join("fitted");
        std::fs::create_dir_all(&out).expect("fitted dir");
        for game in games {
            let stem = game.file_name().unwrap().to_string_lossy().to_string();
            let cart = Cart {
                platform,
                stem: stem.clone(),
                rom: game.join("rom"),
                label: None,
                famicom_label: None,
                title: stem.clone(),
                code: String::new(),
                shell: None,
            };
            let (lw, lh) = slot_ui::label_size(&cart);
            // Read and decode every picture the engine can use (one per kind, the preferred).
            let mut files: Vec<(Kind, u8, PathBuf)> = std::fs::read_dir(&game)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    let name = p.file_stem()?.to_string_lossy().to_string();
                    let (k, pref) = kind_of(&name)?;
                    Some((k, pref, p))
                })
                .collect();
            files.sort_by_key(|f| f.1);
            let mut raw_bytes = 0usize;
            let t = Instant::now();
            let mut sources: Vec<Source> = Vec::new();
            for (kind, _, path) in &files {
                if sources.iter().any(|s| s.kind == *kind) {
                    continue;
                }
                let bytes = std::fs::read(path).expect("read");
                raw_bytes += bytes.len();
                if let Some(img) = slot_cartart::decode_png(&bytes) {
                    sources.push(Source { kind: *kind, img });
                }
            }
            let decode_ms = t.elapsed().as_secs_f64() * 1000.0;
            let Some(fitted) = slot_cartart::fit(dir, lw, lh, scale, &sources) else {
                report.push_str(&format!("{dir}\t{stem}\tnone\tno usable art\t{lw}x{lh}\t-\t{decode_ms:.1}\t-\t-\t0\t{raw_bytes}\n"));
                println!("{dir}\t{stem}\tnone");
                continue;
            };
            let t = Instant::now();
            let png = fitted.png();
            let encode_ms = t.elapsed().as_secs_f64() * 1000.0;
            std::fs::write(out.join(format!("{stem}.label.png")), &png).expect("write label");
            // On the cart, exactly as the shelf would draw it from that file.
            let art = slot_ui::fitted_label_rgba(
                &cart,
                &fitted.img.px,
                fitted.img.w,
                fitted.img.h,
                lw,
                lh,
            );
            let face = slot_ui::cart_face_with(&cart, art);
            save(
                &out.join(format!("{stem}.cart.png")),
                &face.rgba,
                face.w,
                face.h,
            );
            let plate = slot_ui::cart_face_with(&cart, None);
            save(
                &out.join(format!("{stem}.plate.png")),
                &plate.rgba,
                plate.w,
                plate.h,
            );
            // Every tier on its own too, so the fallbacks can be judged on every system.
            for &kind in slot_cartart::profile(dir).order {
                match slot_cartart::fit_tier(dir, lw, lh, scale, &sources, kind) {
                    Some(f) => {
                        let art =
                            slot_ui::fitted_label_rgba(&cart, &f.img.px, f.img.w, f.img.h, lw, lh);
                        let face = slot_ui::cart_face_with(&cart, art);
                        save(
                            &out.join(format!("{stem}.tier-{}.png", kind.name())),
                            &face.rgba,
                            face.w,
                            face.h,
                        );
                        tiers.push_str(&format!("{dir}\t{stem}\t{}\t{}\n", kind.name(), f.how));
                    }
                    None => {
                        let _ = std::fs::remove_file(
                            out.join(format!("{stem}.tier-{}.png", kind.name())),
                        );
                    }
                }
            }
            let fit_ms = fitted.micros as f64 / 1000.0;
            println!(
                "{dir}\t{stem}\t{}\t{}\t{fit_ms:.1}ms",
                fitted.tier.name(),
                fitted.how
            );
            report.push_str(&format!(
                "{dir}\t{stem}\t{}\t{}\t{lw}x{lh}\t{}x{}\t{decode_ms:.1}\t{fit_ms:.1}\t{encode_ms:.1}\t{}\t{raw_bytes}\n",
                fitted.tier.name(),
                fitted.how,
                fitted.img.w,
                fitted.img.h,
                png.len()
            ));
        }
    }
    let name = if only.is_some() {
        "results-partial.tsv"
    } else {
        "results.tsv"
    };
    std::fs::write(root.join(name), report).expect("report");
    std::fs::write(root.join(name.replace("results", "tiers")), tiers).expect("tiers");
}
