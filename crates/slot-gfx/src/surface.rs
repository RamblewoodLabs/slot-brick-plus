use std::ffi::c_void;
use std::fmt;

// 4:3 logical layout; source emulator framebuffer remains 240x160.
pub const OUT_W: u32 = if cfg!(feature = "brick") { 640 } else { 720 };
pub const OUT_H: u32 = 480;

#[derive(Debug)]
pub enum GfxError {
    Context(String),
    Shader(String),
    Framebuffer(u32),
}

impl fmt::Display for GfxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GfxError::Context(m) => write!(f, "gl context: {m}"),
            GfxError::Shader(m) => write!(f, "shader: {m}"),
            GfxError::Framebuffer(s) => write!(f, "framebuffer incomplete: 0x{s:x}"),
        }
    }
}

impl std::error::Error for GfxError {}

pub trait Surface {
    fn make_current(&mut self) -> Result<(), GfxError>;
    fn window_size(&self) -> (u32, u32);
    fn swap(&mut self) -> Result<(), GfxError>;
    fn proc_address(&self, name: &str) -> *const c_void;
}

pub fn fit_scale(win_w: u32, win_h: u32) -> u32 {
    (win_w / OUT_W).min(win_h / OUT_H).max(1)
}

pub fn fit_rect(win_w: u32, win_h: u32) -> (i32, i32, i32, i32) {
    let s = fit_scale(win_w, win_h) as i32;
    let (w, h) = (OUT_W as i32 * s, OUT_H as i32 * s);
    ((win_w as i32 - w) / 2, (win_h as i32 - h) / 2, w, h)
}

pub fn blit_rect_fit(panel: (u32, u32), shake: f32) -> (i32, i32, i32, i32) {
    let scale = (panel.0 as f32 / OUT_W as f32).min(panel.1 as f32 / OUT_H as f32);
    let w = (OUT_W as f32 * scale).round() as i32;
    let h = (OUT_H as f32 * scale).round() as i32;
    let dx = (shake * scale).round() as i32;
    (
        (panel.0 as i32 - w) / 2 + dx,
        (panel.1 as i32 - h) / 2,
        w,
        h,
    )
}

pub fn blit_rect(window: (u32, u32), shake: f32) -> (i32, i32, i32, i32) {
    if crate::bezel::framed(window) {
        let (x, y, w, h) = crate::bezel::screen_rect(window);
        let dx = shake * w as f32 / OUT_W as f32;
        return (x + dx.round() as i32, window.1 as i32 - y - h, w, h);
    }
    if cfg!(feature = "rg280v") {
        let (x, y, w, h, _) = crate::letterbox::letterbox_rect(window, crate::letterbox::orientation());
        let dx = (shake * 0.5).round() as i32;
        return (x + dx, y, w, h);
    }
    if cfg!(feature = "brick") || window.0 < OUT_W || window.1 < OUT_H {
        return blit_rect_fit(window, shake);
    }
    let (x, y, w, h) = fit_rect(window.0, window.1);
    let dx = shake * fit_scale(window.0, window.1) as f32;
    (x + dx.round() as i32, y, w, h)
}

#[cfg(all(test, feature = "brick"))]
mod brick_tests {
    #[test]
    fn brick_uses_full_panel_with_native_four_three_layout() {
        assert!(!crate::bezel::framed((1024, 768)));
        assert_eq!(super::blit_rect((1024, 768), 0.0), (0, 0, 1024, 768));
    }
}

/// Where a logical layout lands on a panel, centred, keeping its shape.
#[cfg(test)]
fn layout_on_panel(layout: (u32, u32), panel: (u32, u32)) -> (i32, i32, i32, i32) {
    let scale = (panel.0 as f32 / layout.0 as f32).min(panel.1 as f32 / layout.1 as f32);
    let w = (layout.0 as f32 * scale).round() as i32;
    let h = (layout.1 as f32 * scale).round() as i32;
    ((panel.0 as i32 - w) / 2, (panel.1 as i32 - h) / 2, w, h)
}

#[cfg(test)]
mod panel_fit {
    use super::layout_on_panel;

    #[test]
    fn the_sp_shows_the_brick_layout_at_one_to_one() {
        assert_eq!(layout_on_panel((640, 480), (640, 480)), (0, 0, 640, 480));
    }

    #[test]
    fn the_brick_scales_that_layout_to_the_whole_panel() {
        assert_eq!(layout_on_panel((640, 480), (1024, 768)), (0, 0, 1024, 768));
    }

    #[test]
    fn a_320x480_panel_fits_an_integer_half_of_the_640x480_layout() {
        // A "contain" fit on a 320×480 panel is already 320×240. A 480×320
        // panel would scale by 2/3 if we only fitted; the RG280V path instead
        // forces the integer half (see letterbox.rs).
        assert_eq!(layout_on_panel((640, 480), (320, 480)), (0, 120, 320, 240));
        assert_eq!(
            crate::letterbox::letterbox_rect((480, 320), crate::letterbox::Orientation::Landscape),
            (80, 40, 320, 240, 0)
        );
    }
}
