use slot_ui::tape_face;

#[test]
fn a_thumbnail_fills_the_label_window() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("thumb.png");
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, 8, 8);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let red = [220u8, 20, 20, 255].repeat(64);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&red)
        .unwrap();
    std::fs::write(&path, bytes).unwrap();

    let bare = tape_face(None);
    let with = tape_face(Some(&path));
    assert_eq!((with.w, with.h), (slot_ui::TAPE_W, slot_ui::TAPE_H));
    let reds = |face: &slot_ui::UndoFace| {
        face.rgba
            .chunks(4)
            .filter(|p| p[0] > 180 && p[1] < 60 && p[2] < 60)
            .count()
    };
    assert!(reds(&with) > reds(&bare), "the picture is on the label");
    if let Ok(dir) = std::env::var("SLOT_FEATURE_PREVIEW") {
        let path = std::path::PathBuf::from(dir).join("video-tape.png");
        let mut file = std::fs::File::create(&path).unwrap();
        let mut encoder = png::Encoder::new(&mut file, with.w, with.h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&with.rgba)
            .unwrap();
    }
}
