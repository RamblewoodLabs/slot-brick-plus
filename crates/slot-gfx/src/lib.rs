mod bezel;
mod draw;
mod fbdev;
mod fbo;
mod gl;
mod grade;
#[cfg(target_os = "macos")]
mod headless;
#[cfg(feature = "host")]
mod host;
mod letterbox;
mod lcd3x;
mod pipeline;
mod power;
mod quad;
#[cfg(feature = "rg280v")]
mod sdl;
mod shaders;
mod surface;

pub use bezel::{framed, lifted_rect, screen_rect};
pub use draw::{Draw, TexId};
pub use fbdev::{egl_error, panel_mode, panel_size, FbdevSurface};
#[cfg(feature = "rg280v")]
pub use sdl::SdlSurface;
pub use fbo::{canvas_size, Compositor, BACKDROP};
pub use grade::{blue_light_gain, BLUE_LIGHT_MAX};
#[cfg(target_os = "macos")]
pub use headless::HeadlessSurface;
#[cfg(feature = "host")]
pub use host::HostSurface;
pub use lcd3x::{lcd3x_factors, lcd3x_mask, ScreenEffect};
pub use letterbox::{letterbox_rect, load_from as load_orientation, orientation, set_orientation, Orientation, CONTENT_H, CONTENT_W};
pub use pipeline::{SCALE, SRC_H, SRC_W, WHOLE_TEXTURE};
pub use power::{screen_brightness, screen_scale, screen_width};
pub use surface::{blit_rect, blit_rect_fit, fit_rect, fit_scale, GfxError, Surface, OUT_H, OUT_W};
