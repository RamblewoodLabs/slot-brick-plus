//! The real vecx core, if a build of it is named by SLOT_VECX_CORE: its frame comes out of the
//! frontend as one picture of white vectors, not two squashed magenta copies (vecx draws in
//! 0RGB1555, which the frontend once read as XRGB8888).
use slot_retro::{ButtonMask, LibretroCore, RetroCore};
use std::path::PathBuf;

#[test]
fn vecx_draws_one_white_picture() {
    let Some(core) = std::env::var_os("SLOT_VECX_CORE") else {
        eprintln!("SLOT_VECX_CORE is not set, skipping");
        return;
    };
    let mut core = LibretroCore::open(std::path::Path::new(&core)).expect("vecx would not open");
    core.set_canvas(330, 410);
    // Not a cartridge: the BIOS runs its own Mine Storm.
    let rom: PathBuf = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("none.bin");
    std::fs::write(&rom, vec![0u8; 0x1000]).unwrap();
    core.load(&rom).expect("vecx would not load");
    for _ in 0..300 {
        core.run_frame(ButtonMask::default());
    }
    let (cw, ch) = core.canvas();
    let video = core.video_xrgb8888();
    // SLOT_VECX_PPM names a file to keep the frame in, as a binary PPM.
    if let Some(path) = std::env::var_os("SLOT_VECX_PPM") {
        let mut out = format!("P6\n{cw} {ch}\n255\n").into_bytes();
        for p in video.chunks(4) {
            out.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        std::fs::write(path, out).unwrap();
    }
    let lit: Vec<&[u8]> = video
        .chunks(4)
        .filter(|p| p[0] | p[1] | p[2] != 0)
        .collect();
    assert!(
        lit.len() > 200,
        "vecx drew almost nothing: {} pixels",
        lit.len()
    );
    let grey = lit.iter().filter(|p| p[0] == p[1] && p[1] == p[2]).count();
    assert!(
        grey * 100 >= lit.len() * 95,
        "vecx's white vectors are not white: {grey} of {} pixels grey",
        lit.len()
    );
}
