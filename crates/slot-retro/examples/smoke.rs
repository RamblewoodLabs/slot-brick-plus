//! Loads a game in a core and runs it headless: how long frames take, whether a picture
//! comes out, and the last frame as a PPM. For trying a core on a PC or the device.
//!
//!     smoke CORE.so GAME SYSTEM_DIR FRAMES W H OUT.ppm [--gles] [--stick] [key=value ...]
use slot_retro::{ButtonMask, LibretroCore, RetroCore};
use std::path::Path;
use std::time::Instant;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 8 {
        eprintln!("usage: smoke CORE GAME SYSTEM_DIR FRAMES W H OUT.ppm [--gles] [--stick] [key=value ...]");
        std::process::exit(2);
    }
    let frames: u32 = a[4].parse().expect("frames");
    let (w, h): (u32, u32) = (a[5].parse().expect("w"), a[6].parse().expect("h"));
    let rest = &a[8..];
    if rest.iter().any(|s| s == "--gles") && !slot_retro::preload_gles() {
        eprintln!("no GLES");
        std::process::exit(1);
    }
    let saves = std::env::temp_dir().join("smoke-saves");
    std::fs::create_dir_all(&saves).ok();
    let mut core = LibretroCore::open_with(Path::new(&a[1]), Path::new(&a[3]), &saves).expect("open core");
    core.set_canvas(w, h);
    core.set_dpad_stick(rest.iter().any(|s| s == "--stick"), None);
    for kv in rest.iter().filter_map(|s| s.split_once('=')) {
        core.set_option(kv.0, kv.1);
    }
    let t = Instant::now();
    core.load(Path::new(&a[2])).expect("load game");
    let loaded = t.elapsed();
    let start = 1u16 << 3;
    let mut times = Vec::new();
    for i in 0..frames {
        // Start pressed now and then, to get past title screens.
        let press = if i % 120 > 100 { start } else { 0 };
        let t = Instant::now();
        core.run_frame(ButtonMask(press));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let (cw, ch) = core.canvas();
    let px = core.video_xrgb8888();
    let lit = px.chunks(4).filter(|p| p[0] as u32 + p[1] as u32 + p[2] as u32 > 30).count();
    let mut ppm = format!("P6\n{cw} {ch}\n255\n").into_bytes();
    for p in px.chunks(4).take((cw * ch) as usize) {
        ppm.extend_from_slice(&[p[2], p[1], p[0]]);
    }
    std::fs::write(&a[7], ppm).expect("write ppm");
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let tail = &times[times.len() / 4..];
    let mean = tail.iter().sum::<f64>() / tail.len() as f64;
    println!(
        "loaded in {:.2}s; frames after the first quarter: mean {:.2} ms, p95 {:.2} ms; canvas {cw}x{ch}, {:.0}% lit",
        loaded.as_secs_f64(),
        mean,
        times[times.len() * 95 / 100],
        100.0 * lit as f64 / (cw * ch) as f64
    );
}
