use std::path::Path;

use slot_gfx::WHOLE_TEXTURE;
#[cfg(all(test, feature = "brick"))]
use slot_gfx::{SRC_H, SRC_W};
use slot_store::{ini, Platform};

pub const VIDEO_MODE_FILE: &str = if cfg!(feature = "brick") {
    "Config/brick_video_mode_v3.ini"
} else {
    "Config/video_mode.ini"
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VideoMode {
    /// Square pixels, fitted inside the screen. Saved as `actual` or `native`.
    Actual,
    /// The picture fills the screen. Saved as `stretch` or `full`.
    Stretch,
    /// The picture is shown at 4:3. On this 4:3 screen that fills it, the same as Full.
    FourThree,
    /// The largest whole-number scale that fits, with bars around it.
    Integer,
}

impl Default for VideoMode {
    fn default() -> Self { Self::Actual }
}

impl VideoMode {
    pub const CHOICES: [VideoMode; 4] = [
        VideoMode::Actual,
        VideoMode::FourThree,
        VideoMode::Stretch,
        VideoMode::Integer,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            VideoMode::Actual => "actual",
            VideoMode::Stretch => "stretch",
            VideoMode::FourThree => "4:3",
            VideoMode::Integer => "integer",
        }
    }

    /// What the menus print.
    pub fn label(self) -> &'static str {
        match self {
            VideoMode::Actual => "Native",
            VideoMode::FourThree => "4:3",
            VideoMode::Stretch => "Full",
            VideoMode::Integer => "Integer",
        }
    }

    pub fn index(self) -> usize {
        Self::CHOICES.iter().position(|mode| *mode == self).unwrap_or(0)
    }

    pub fn step(self, right: bool) -> VideoMode {
        let n = Self::CHOICES.len();
        let at = self.index();
        Self::CHOICES[if right { (at + 1) % n } else { (at + n - 1) % n }]
    }

    pub fn parse(s: &str) -> Option<VideoMode> {
        match s.trim().to_ascii_lowercase().as_str() {
            "actual" | "native" | "original" => Some(VideoMode::Actual),
            "stretch" | "full" => Some(VideoMode::Stretch),
            "4:3" | "43" | "fourthree" => Some(VideoMode::FourThree),
            "integer" => Some(VideoMode::Integer),
            _ => None,
        }
    }
}

pub fn video_mode_for(root: &Path, stem: &str) -> VideoMode {
    video_mode_for_platform(root, stem, Platform::Gba)
}

pub fn default_for_platform(_platform: Platform) -> VideoMode {
    VideoMode::Actual
}

pub fn video_mode_for_platform(root: &Path, stem: &str, platform: Platform) -> VideoMode {
    ini::value(root, VIDEO_MODE_FILE, stem)
        .as_deref()
        .and_then(VideoMode::parse)
        .unwrap_or_else(|| default_for_platform(platform))
}

pub fn write_video_mode(root: &Path, stem: &str, mode: VideoMode) -> std::io::Result<()> {
    ini::write(root, VIDEO_MODE_FILE, stem, mode.as_str())
}

fn global_key() -> &'static str {
    "default"
}

fn system_key(platform: Platform) -> String {
    format!("system_{}", platform.dir_name())
}

/// The picture for this game: its own saved choice, else the system's, else the global one.
pub fn effective(root: &Path, stem: &str, platform: Platform) -> VideoMode {
    if let Some(mode) = ini::value(root, VIDEO_MODE_FILE, stem).as_deref().and_then(VideoMode::parse) {
        return mode;
    }
    system_mode(root, platform).unwrap_or_else(|| global_mode(root))
}

pub fn global_mode(root: &Path) -> VideoMode {
    ini::value(root, VIDEO_MODE_FILE, global_key())
        .as_deref()
        .and_then(VideoMode::parse)
        .unwrap_or(VideoMode::Actual)
}

pub fn system_mode(root: &Path, platform: Platform) -> Option<VideoMode> {
    ini::value(root, VIDEO_MODE_FILE, &system_key(platform))
        .as_deref()
        .and_then(VideoMode::parse)
}

pub fn write_global(root: &Path, mode: VideoMode) -> std::io::Result<()> {
    ini::write(root, VIDEO_MODE_FILE, global_key(), mode.as_str())
}

pub fn write_system(root: &Path, platform: Platform, mode: Option<VideoMode>) -> std::io::Result<()> {
    write_opt(root, &system_key(platform), mode.map(VideoMode::as_str))
}

pub fn clear_game(root: &Path, stem: &str) -> std::io::Result<()> {
    write_opt(root, stem, None)
}

fn write_opt(root: &Path, key: &str, value: Option<&str>) -> std::io::Result<()> {
    let path = root.join(VIDEO_MODE_FILE);
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let out = ini::set(&existing, key, value)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    slot_store::atomic_write(&path, out.as_bytes())
}

/// Largest whole-number scale of the picture that fits the screen, centred.
fn integer_rect(platform: Platform) -> [f32; 4] {
    let (pw, ph) = platform.picture();
    let (cw, ch) = platform.canvas();
    let scale = (slot_gfx::OUT_W / pw).min(slot_gfx::OUT_H / ph).max(1);
    let vw = slot_gfx::OUT_W as f32 / scale as f32;
    let vh = slot_gfx::OUT_H as f32 / scale as f32;
    let px = (cw - pw) as f32 / 2.0;
    let py = (ch - ph) as f32 / 2.0;
    let x = px + pw as f32 / 2.0 - vw / 2.0;
    let y = py + ph as f32 / 2.0 - vh / 2.0;
    [x / cw as f32, y / ch as f32, vw / cw as f32, vh / ch as f32]
}

pub fn source_rect(platform: Platform, mode: VideoMode) -> [f32; 4] {
    if mode == VideoMode::Integer {
        return integer_rect(platform);
    }
    // 4:3 on this 4:3 panel is the picture filling the screen.
    let mode = if mode == VideoMode::FourThree { VideoMode::Stretch } else { mode };
    let (w, h) = platform.picture();
    // In canvas units: a NES frame arrives on a 256x240 canvas, the Game Boys' on 240x160.
    let (src_w, src_h) = platform.canvas();
    #[cfg(feature = "brick")]
    if mode == VideoMode::Actual {
        // Expand the source rectangle to 4:3; the shader paints the region
        // outside the active picture black. Stretch mode below has no bars.
        let aspect = slot_gfx::OUT_W as f32 / slot_gfx::OUT_H as f32;
        let (vw, vh) = if w as f32 / h as f32 > aspect {
            (w as f32, w as f32 / aspect)
        } else {
            (h as f32 * aspect, h as f32)
        };
        return [
            (src_w as f32 - vw) / (2.0 * src_w as f32),
            (src_h as f32 - vh) / (2.0 * src_h as f32),
            vw / src_w as f32,
            vh / src_h as f32,
        ];
    }
    if mode == VideoMode::Actual || (w, h) == (src_w, src_h) {
        return WHOLE_TEXTURE;
    }
    let x = src_w.saturating_sub(w) / 2;
    let y = src_h.saturating_sub(h) / 2;
    [
        x as f32 / src_w as f32,
        y as f32 / src_h as f32,
        w as f32 / src_w as f32,
        h as f32 / src_h as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_follows_its_own_choice_then_its_system_then_the_global_one() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::Actual);
        write_global(root.path(), VideoMode::Integer).unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::Integer);
        write_system(root.path(), Platform::Nes, Some(VideoMode::Stretch)).unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::Stretch);
        assert_eq!(effective(root.path(), "Emerald", Platform::Gba), VideoMode::Integer);
        write_video_mode(root.path(), "Emerald", VideoMode::FourThree).unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::FourThree);
        clear_game(root.path(), "Emerald").unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::Stretch);
        write_system(root.path(), Platform::Nes, None).unwrap();
        assert_eq!(effective(root.path(), "Emerald", Platform::Nes), VideoMode::Integer);
    }
}

#[cfg(all(test, feature = "brick"))]
mod brick_tests {
    use super::*;
    #[test]
    fn default_fills_screen_without_cropping_game_pixels() {
        assert_eq!(VideoMode::default(), VideoMode::Actual);
        assert_eq!((slot_gfx::OUT_W, slot_gfx::OUT_H), (640, 480));
        assert_eq!((SRC_W, SRC_H), (240, 160));
        for platform in [Platform::Gb, Platform::Gbc, Platform::Gba] {
            let [x, y, w, h] = source_rect(platform, VideoMode::Stretch);
            let (pw, ph) = platform.picture();
            assert!((w * SRC_W as f32 - pw as f32).abs() < 0.001);
            assert!((h * SRC_H as f32 - ph as f32).abs() < 0.001);
            assert!((2.0 * x + w - 1.0).abs() < 0.001);
            assert!((2.0 * y + h - 1.0).abs() < 0.001);
        }
    }
    #[test]
    fn defaults_are_platform_specific_and_saved_choices_work() {
        let root = tempfile::tempdir().unwrap();
        for platform in [Platform::Gb, Platform::Gbc] {
            assert_eq!(
                video_mode_for_platform(root.path(), "unsaved", platform),
                VideoMode::Actual
            );
            let [_, _, w, h] = source_rect(platform, default_for_platform(platform));
            let (pw, ph) = platform.picture();
            let ratio =
                (4.0 / 3.0) * (pw as f32 / (w * SRC_W as f32)) / (ph as f32 / (h * SRC_H as f32));
            assert!((ratio - 10.0 / 9.0).abs() < 0.001);
        }
        assert_eq!(
            video_mode_for_platform(root.path(), "unsaved", Platform::Gba),
            VideoMode::Actual
        );
        assert_eq!(
            source_rect(Platform::Gba, default_for_platform(Platform::Gba)),
            [0.0, -0.0625, 1.0, 1.125]
        );
        write_video_mode(root.path(), "choice", VideoMode::Actual).unwrap();
        assert_eq!(
            video_mode_for_platform(root.path(), "choice", Platform::Gba),
            VideoMode::Actual
        );
    }

    #[test]
    fn a_nes_picture_sits_in_its_own_canvas() {
        let (cw, ch) = Platform::Nes.canvas();
        let [x, y, w, h] = source_rect(Platform::Nes, VideoMode::Stretch);
        assert_eq!((w * cw as f32, h * ch as f32), (256.0, 224.0), "the overscan-cropped picture");
        assert_eq!((x, y * ch as f32), (0.0, 8.0), "centred, 8 lines down");
        let [_, _, w, h] = source_rect(Platform::Nes, VideoMode::Actual);
        assert!((w * cw as f32 / (h * ch as f32) - 4.0 / 3.0).abs() < 0.001, "padded to 4:3");
        assert!((h * ch as f32 - 224.0).abs() < 0.001, "no line cropped");
    }

    #[test]
    fn integer_scale_is_a_whole_number_of_screen_pixels() {
        let (pw, ph) = Platform::Nes.picture();
        let scale = (slot_gfx::OUT_W / pw).min(slot_gfx::OUT_H / ph);
        let [x, y, w, h] = source_rect(Platform::Nes, VideoMode::Integer);
        let (cw, ch) = Platform::Nes.canvas();
        let screen_w = w * cw as f32 * scale as f32;
        let screen_h = h * ch as f32 * scale as f32;
        assert!((screen_w - slot_gfx::OUT_W as f32).abs() < 0.01);
        assert!((screen_h - slot_gfx::OUT_H as f32).abs() < 0.01);
        let picture_x = (cw - pw) as f32 / 2.0;
        let shown = (picture_x / cw as f32 - x) / w * slot_gfx::OUT_W as f32;
        assert!((shown - (slot_gfx::OUT_W as f32 - pw as f32 * scale as f32) / 2.0).abs() < 0.5);
        let _ = (y, ph);
    }

    #[test]
    fn actual_mode_preserves_original_aspect() {
        for platform in [Platform::Gb, Platform::Gbc, Platform::Gba] {
            let [_, _, w, h] = source_rect(platform, VideoMode::Actual);
            assert!((w * SRC_W as f32 / (h * SRC_H as f32) - 4.0 / 3.0).abs() < 0.001);
        }
    }
}
