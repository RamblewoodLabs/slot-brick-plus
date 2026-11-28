//! Prints what the analysis sees in one picture: size, backdrop, object box, skew.
//!     cargo run --release -p slot-cartart --example probe -- picture.png...
use slot_cartart::analyze::{border_colour, foreground, skew_degrees};

fn main() {
    for path in std::env::args().skip(1) {
        let Some(img) = slot_cartart::decode_png(&std::fs::read(&path).unwrap()) else {
            println!("{path}: not a PNG");
            continue;
        };
        let m = foreground(&img);
        let bb = m.bbox();
        let (edge, spread) = border_colour(&img);
        println!(
            "{path}\n  {}x{} alpha={:.0} edge={edge:?} spread={spread:.1} bg={:?} cover={:.2} bbox={bb:?} aspect={:.3} skew={:?}",
            img.w,
            img.h,
            img.mean_alpha(),
            m.bg,
            m.coverage(),
            bb.map(|b| b.w() as f32 / b.h() as f32).unwrap_or(0.0),
            skew_degrees(&m)
        );
    }
}
