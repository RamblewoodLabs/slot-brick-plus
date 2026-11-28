//! Times the cart-art engine the way the card runs it (test tooling; not on the card).
//!
//!     cargo run --release -p slot-ui --example cartart_bench -- ROOT [RUNS]
//!
//! For every game in ROOT/<DIR>/raw/<game>/, walks the system's tiers in order like
//! `cart_art_ss::label_for`: decode the next picture (first shrunk to at most 1000 px, as
//! ScreenScraper sends it with maxwidth/maxheight=1000), try that tier, stop at the first label,
//! encode it at the card's size (twice the window, at most 512 px across). One thread; the median
//! of RUNS runs. Writes ROOT/bench.tsv and prints a summary.
use std::path::PathBuf;
use std::time::Instant;

use slot_cartart::{Image, Kind, Source};
use slot_store::{Cart, Platform};

fn file_for(game: &std::path::Path, kind: Kind) -> Option<PathBuf> {
    let names: &[&str] = match kind {
        Kind::Texture => &["support-texture"],
        Kind::Photo => &["support-2D"],
        Kind::Box => &["box-2D"],
        Kind::Logo => &["wheel-hd", "wheel"],
        Kind::Marquee => &["marquee", "screenmarquee"],
    };
    names
        .iter()
        .map(|n| game.join(format!("{n}.png")))
        .find(|p| p.exists())
}

/// As the server sends it: no side over 1000 px, re-encoded.
fn as_sent(bytes: &[u8]) -> Vec<u8> {
    let Some(img) = slot_cartart::decode_png(bytes) else {
        return bytes.to_vec();
    };
    let k = 1000.0 / img.w.max(img.h) as f32;
    if k >= 1.0 {
        return bytes.to_vec();
    }
    let small: Image = slot_cartart::image::resize(
        &img,
        (img.w as f32 * k).round() as u32,
        (img.h as f32 * k).round() as u32,
    );
    slot_cartart::image::encode_png_rgba(&small)
}

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = PathBuf::from(&args[1]);
    let runs: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(7);
    let mut out = String::from("system\tgame\ttier\ttiers_tried\tdecode_ms\tfit_ms\tencode_ms\ttotal_ms\tlabel\tbytes\tbytes_1x\n");
    let mut totals = Vec::new();
    let mut sizes = Vec::new();
    for platform in Platform::ALL {
        let dir = platform.dir_name();
        let Ok(games) = std::fs::read_dir(root.join(dir).join("raw")) else {
            continue;
        };
        let mut games: Vec<PathBuf> = games
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        games.sort();
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
            let k = (512.0 / lw.max(lh).max(1) as f32).min(2.0);
            let (ow, oh) = (
                ((lw as f32 * k).round() as u32).max(1),
                ((lh as f32 * k).round() as u32).max(1),
            );
            let pictures: Vec<(Kind, Vec<u8>)> = slot_cartart::profile(dir)
                .order
                .iter()
                .filter_map(|&kind| {
                    Some((kind, as_sent(&std::fs::read(file_for(&game, kind)?).ok()?)))
                })
                .collect();
            let (mut dec, mut fit, mut enc) = (Vec::new(), Vec::new(), Vec::new());
            let mut result = None;
            for _ in 0..runs {
                let (mut d, mut f) = (0.0, 0.0);
                let mut sources: Vec<Source> = Vec::new();
                let mut got = None;
                for (kind, png) in &pictures {
                    let t = Instant::now();
                    let Some(img) = slot_cartart::decode_png(png) else {
                        continue;
                    };
                    d += t.elapsed().as_secs_f64() * 1e3;
                    sources.push(Source { kind: *kind, img });
                    let t = Instant::now();
                    let fitted = slot_cartart::fit_tier(dir, ow, oh, 1, &sources, *kind);
                    f += t.elapsed().as_secs_f64() * 1e3;
                    if let Some(fitted) = fitted {
                        got = Some((fitted, sources.len()));
                        break;
                    }
                }
                let mut e = 0.0;
                if let Some((fitted, _)) = &got {
                    let t = Instant::now();
                    let png = fitted.png();
                    e = t.elapsed().as_secs_f64() * 1e3;
                    std::hint::black_box(png);
                }
                dec.push(d);
                fit.push(f);
                enc.push(e);
                result = got;
            }
            let (d, f, e) = (median(&mut dec), median(&mut fit), median(&mut enc));
            let Some((fitted, tried)) = result else {
                out.push_str(&format!(
                    "{dir}\t{stem}\tnone\t{}\t{d:.1}\t{f:.1}\t-\t{:.1}\t-\t0\t0\n",
                    pictures.len(),
                    d + f
                ));
                continue;
            };
            let bytes = fitted.png().len();
            let small = slot_cartart::image::resize(&fitted.img, lw, lh);
            let one = if fitted.transparent {
                slot_cartart::image::encode_png_rgba(&small)
            } else {
                slot_cartart::encode_png_rgb(&small)
            }
            .len();
            let total = d + f + e;
            totals.push(total);
            sizes.push(bytes as f64);
            out.push_str(&format!(
                "{dir}\t{stem}\t{}\t{tried}\t{d:.1}\t{f:.1}\t{e:.1}\t{total:.1}\t{}x{}\t{bytes}\t{one}\n",
                fitted.tier.name(),
                fitted.img.w,
                fitted.img.h
            ));
        }
    }
    std::fs::write(root.join("bench.tsv"), &out).expect("bench.tsv");
    let n = totals.len();
    let mut t = totals.clone();
    let p50 = median(&mut t);
    let max = t.last().copied().unwrap_or(0.0);
    let p90 = t[(n * 9 / 10).min(n - 1)];
    let mean_kb = sizes.iter().sum::<f64>() / n as f64 / 1024.0;
    println!("{n} labels: total per game median {p50:.1} ms, p90 {p90:.1} ms, max {max:.1} ms; label mean {mean_kb:.0} KB");
}
