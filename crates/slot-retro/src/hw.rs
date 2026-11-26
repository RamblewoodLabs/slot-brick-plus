//! A GLES context for cores that draw with the GPU (the N64's GLideN64, PPSSPP).
//!
//! The core gets its own pbuffer context on the thread that runs it, and an off-screen
//! framebuffer to draw into. Each finished frame is read back into the same XRGB8888 canvas
//! every software core fills, so the rest of Slot (shaders, rewind pictures, polaroids, the
//! shelf's screen) does not know the difference. The context is not shared with Slot's own.
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::sync::OnceLock;

use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};

/// retro_hw_context_type values Slot can give: GLES 2, GLES 3, and "GLES of this version".
pub const CONTEXT_OPENGLES2: c_uint = 2;
pub const CONTEXT_OPENGLES3: c_uint = 4;
pub const CONTEXT_OPENGLES_VERSION: c_uint = 5;

pub type ProcAddress = Option<unsafe extern "C" fn()>;

#[repr(C)]
pub struct HwRenderCallback {
    pub context_type: c_uint,
    pub context_reset: Option<unsafe extern "C" fn()>,
    pub get_current_framebuffer: Option<unsafe extern "C" fn() -> usize>,
    pub get_proc_address: Option<unsafe extern "C" fn(*const c_char) -> ProcAddress>,
    pub depth: bool,
    pub stencil: bool,
    pub bottom_left_origin: bool,
    pub version_major: c_uint,
    pub version_minor: c_uint,
    pub cache_context: bool,
    pub context_destroy: Option<unsafe extern "C" fn()>,
    pub debug_context: bool,
}

/// What the core asked for, kept from SET_HW_RENDER until the game is loaded.
#[derive(Clone, Copy)]
pub struct Request {
    pub gles3: bool,
    pub depth: bool,
    pub stencil: bool,
    pub bottom_left_origin: bool,
    pub context_reset: Option<unsafe extern "C" fn()>,
    pub context_destroy: Option<unsafe extern "C" fn()>,
}

impl Request {
    /// The request, if it is for a context Slot can make (GLES 2 or 3).
    pub fn from_callback(cb: &HwRenderCallback) -> Option<Request> {
        let gles3 = match cb.context_type {
            CONTEXT_OPENGLES2 => false,
            CONTEXT_OPENGLES3 => true,
            CONTEXT_OPENGLES_VERSION => cb.version_major >= 3,
            _ => return None,
        };
        Some(Request {
            gles3,
            depth: cb.depth,
            stencil: cb.stencil,
            bottom_left_origin: cb.bottom_left_origin,
            context_reset: cb.context_reset,
            context_destroy: cb.context_destroy,
        })
    }
}

struct GlLibs {
    egl: Library,
    gles: Library,
}

static LIBS: OnceLock<Option<GlLibs>> = OnceLock::new();

fn open_any(names: &[&str]) -> Option<Library> {
    names
        .iter()
        .find_map(|n| unsafe { Library::open(Some(*n), RTLD_NOW | RTLD_GLOBAL) }.ok())
}

/// The device's EGL and GLES, loaded globally and kept for good. A GPU core is built without
/// naming them (the vendor libraries' file names differ between firmwares), so this has to run
/// before such a core is opened: its GL calls bind to whatever is loaded here.
pub fn preload() -> bool {
    LIBS.get_or_init(|| {
        let egl = open_any(&["libEGL.so.1", "libEGL.so"])?;
        let gles = open_any(&["libGLESv2.so.2", "libGLESv2.so"])?;
        Some(GlLibs { egl, gles })
    })
    .is_some()
}

fn libs() -> Option<&'static GlLibs> {
    LIBS.get().and_then(|l| l.as_ref())
}

/// A GL or EGL entry point by name: EGL's lookup first, then the libraries themselves.
pub unsafe fn proc_address(name: &CStr) -> ProcAddress {
    let libs = libs()?;
    if let Ok(get) = libs
        .egl
        .get::<unsafe extern "C" fn(*const c_char) -> ProcAddress>(b"eglGetProcAddress\0")
    {
        if let Some(f) = get(name.as_ptr()) {
            return Some(f);
        }
    }
    let bytes = name.to_bytes_with_nul();
    for lib in [&libs.gles, &libs.egl] {
        if let Ok(sym) = lib.get::<unsafe extern "C" fn()>(bytes) {
            return Some(*sym);
        }
    }
    None
}

type EglDisplay = *mut c_void;
type EglConfig = *mut c_void;
type EglContext = *mut c_void;
type EglSurface = *mut c_void;

const EGL_NONE: c_int = 0x3038;
const EGL_RED_SIZE: c_int = 0x3024;
const EGL_GREEN_SIZE: c_int = 0x3023;
const EGL_BLUE_SIZE: c_int = 0x3022;
const EGL_ALPHA_SIZE: c_int = 0x3021;
const EGL_DEPTH_SIZE: c_int = 0x3025;
const EGL_STENCIL_SIZE: c_int = 0x3026;
const EGL_SURFACE_TYPE: c_int = 0x3033;
const EGL_RENDERABLE_TYPE: c_int = 0x3040;
const EGL_PBUFFER_BIT: c_int = 0x0001;
const EGL_OPENGL_ES2_BIT: c_int = 0x0004;
const EGL_OPENGL_ES3_BIT: c_int = 0x0040;
const EGL_WIDTH: c_int = 0x3057;
const EGL_HEIGHT: c_int = 0x3056;
const EGL_CONTEXT_CLIENT_VERSION: c_int = 0x3098;
const EGL_OPENGL_ES_API: c_uint = 0x30A0;

const GL_FRAMEBUFFER: c_uint = 0x8D40;
const GL_RENDERBUFFER: c_uint = 0x8D41;
const GL_TEXTURE_2D: c_uint = 0x0DE1;
const GL_RGBA: c_uint = 0x1908;
const GL_UNSIGNED_BYTE: c_uint = 0x1401;
const GL_COLOR_ATTACHMENT0: c_uint = 0x8CE0;
const GL_DEPTH_ATTACHMENT: c_uint = 0x8D00;
const GL_STENCIL_ATTACHMENT: c_uint = 0x8D20;
const GL_DEPTH24_STENCIL8: c_uint = 0x88F0;
const GL_DEPTH_COMPONENT16: c_uint = 0x81A5;
const GL_FRAMEBUFFER_BINDING: c_uint = 0x8CA6;
const GL_FRAMEBUFFER_COMPLETE: c_uint = 0x8CD5;
const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
const GL_NEAREST: c_int = 0x2600;
const GL_PACK_ALIGNMENT: c_uint = 0x0D05;

macro_rules! fns {
    ($($name:ident: fn($($arg:ty),*) $(-> $ret:ty)?;)*) => {
        #[allow(non_snake_case)]
        struct Fns { $($name: unsafe extern "C" fn($($arg),*) $(-> $ret)?,)* }
        impl Fns {
            unsafe fn load() -> Option<Fns> {
                Some(Fns { $($name: std::mem::transmute::<unsafe extern "C" fn(), unsafe extern "C" fn($($arg),*) $(-> $ret)?>(
                    proc_address(CStr::from_bytes_with_nul(concat!(stringify!($name), "\0").as_bytes()).ok()?)?
                ),)* })
            }
        }
    };
}

fns! {
    eglGetDisplay: fn(*mut c_void) -> EglDisplay;
    eglInitialize: fn(EglDisplay, *mut c_int, *mut c_int) -> c_uint;
    eglBindAPI: fn(c_uint) -> c_uint;
    eglChooseConfig: fn(EglDisplay, *const c_int, *mut EglConfig, c_int, *mut c_int) -> c_uint;
    eglCreatePbufferSurface: fn(EglDisplay, EglConfig, *const c_int) -> EglSurface;
    eglCreateContext: fn(EglDisplay, EglConfig, EglContext, *const c_int) -> EglContext;
    eglMakeCurrent: fn(EglDisplay, EglSurface, EglSurface, EglContext) -> c_uint;
    eglGetCurrentContext: fn() -> EglContext;
    eglDestroyContext: fn(EglDisplay, EglContext) -> c_uint;
    eglDestroySurface: fn(EglDisplay, EglSurface) -> c_uint;
    glGenFramebuffers: fn(c_int, *mut c_uint);
    glBindFramebuffer: fn(c_uint, c_uint);
    glFramebufferTexture2D: fn(c_uint, c_uint, c_uint, c_uint, c_int);
    glFramebufferRenderbuffer: fn(c_uint, c_uint, c_uint, c_uint);
    glCheckFramebufferStatus: fn(c_uint) -> c_uint;
    glGenTextures: fn(c_int, *mut c_uint);
    glBindTexture: fn(c_uint, c_uint);
    glTexImage2D: fn(c_uint, c_int, c_int, c_int, c_int, c_int, c_uint, c_uint, *const c_void);
    glTexParameteri: fn(c_uint, c_uint, c_int);
    glGenRenderbuffers: fn(c_int, *mut c_uint);
    glBindRenderbuffer: fn(c_uint, c_uint);
    glRenderbufferStorage: fn(c_uint, c_uint, c_int, c_int);
    glReadPixels: fn(c_int, c_int, c_int, c_int, c_uint, c_uint, *mut c_void);
    glGetIntegerv: fn(c_uint, *mut c_int);
    glPixelStorei: fn(c_uint, c_int);
    glDeleteFramebuffers: fn(c_int, *const c_uint);
    glDeleteTextures: fn(c_int, *const c_uint);
    glDeleteRenderbuffers: fn(c_int, *const c_uint);
}

/// The core's context, current on the emulation thread, and the framebuffer it draws into.
pub struct Gl {
    f: Fns,
    display: EglDisplay,
    surface: EglSurface,
    context: EglContext,
    pub request: Request,
    fbo: c_uint,
    tex: c_uint,
    rbs: [c_uint; 2],
    size: (u32, u32),
    rgba: Vec<u8>,
    pub frame: Vec<u8>,
}

// The handles are only used from the thread the core runs on: the context is made current
// there again before each frame (`current`), so moving the core with its context is sound.
unsafe impl Send for Gl {}

impl Gl {
    /// Makes the context current on this thread, if it is not already (the core may have been
    /// loaded on one thread and run on another).
    pub fn current(&self) -> bool {
        unsafe {
            (self.f.eglGetCurrentContext)() == self.context
                || (self.f.eglMakeCurrent)(self.display, self.surface, self.surface, self.context)
                    != 0
        }
    }

    /// A context of the kind asked for, made current on this thread. None if the device has no
    /// GLES or refuses it; the core is then told no, and most fall back or refuse the game.
    pub fn create(request: Request) -> Option<Gl> {
        if !preload() {
            return None;
        }
        unsafe {
            let f = Fns::load()?;
            let display = (f.eglGetDisplay)(std::ptr::null_mut());
            if display.is_null()
                || (f.eglInitialize)(display, std::ptr::null_mut(), std::ptr::null_mut()) == 0
            {
                return None;
            }
            (f.eglBindAPI)(EGL_OPENGL_ES_API);
            let bit = if request.gles3 {
                EGL_OPENGL_ES3_BIT
            } else {
                EGL_OPENGL_ES2_BIT
            };
            let attrs = [
                EGL_SURFACE_TYPE,
                EGL_PBUFFER_BIT,
                EGL_RENDERABLE_TYPE,
                bit,
                EGL_RED_SIZE,
                8,
                EGL_GREEN_SIZE,
                8,
                EGL_BLUE_SIZE,
                8,
                EGL_ALPHA_SIZE,
                8,
                EGL_DEPTH_SIZE,
                0,
                EGL_STENCIL_SIZE,
                0,
                EGL_NONE,
            ];
            let mut config: EglConfig = std::ptr::null_mut();
            let mut n = 0;
            if (f.eglChooseConfig)(display, attrs.as_ptr(), &mut config, 1, &mut n) == 0 || n < 1 {
                return None;
            }
            let pb = [EGL_WIDTH, 16, EGL_HEIGHT, 16, EGL_NONE];
            let surface = (f.eglCreatePbufferSurface)(display, config, pb.as_ptr());
            if surface.is_null() {
                return None;
            }
            // GLES 3 where the GPU has it, even for a core that asked for 2 (PPSSPP asks for 2
            // and uses 3's features when it finds them); 2 only if 3 is refused.
            let versions: &[c_int] = if request.gles3 { &[3] } else { &[3, 2] };
            let context = versions
                .iter()
                .map(|v| {
                    let ctx_attrs = [EGL_CONTEXT_CLIENT_VERSION, *v, EGL_NONE];
                    (f.eglCreateContext)(display, config, std::ptr::null_mut(), ctx_attrs.as_ptr())
                })
                .find(|c| !c.is_null())
                .unwrap_or(std::ptr::null_mut());
            if context.is_null() {
                (f.eglDestroySurface)(display, surface);
                return None;
            }
            if (f.eglMakeCurrent)(display, surface, surface, context) == 0 {
                (f.eglDestroyContext)(display, context);
                (f.eglDestroySurface)(display, surface);
                return None;
            }
            Some(Gl {
                f,
                display,
                surface,
                context,
                request,
                fbo: 0,
                tex: 0,
                rbs: [0; 2],
                size: (0, 0),
                rgba: Vec::new(),
                frame: Vec::new(),
            })
        }
    }

    /// The framebuffer the core draws into, at least `w` x `h`. Returns its GL name.
    pub fn framebuffer(&mut self, w: u32, h: u32) -> c_uint {
        if self.fbo != 0 && self.size.0 >= w && self.size.1 >= h {
            return self.fbo;
        }
        let (w, h) = (w.max(self.size.0).max(1), h.max(self.size.1).max(1));
        unsafe {
            self.free_framebuffer();
            let f = &self.f;
            (f.glGenTextures)(1, &mut self.tex);
            (f.glBindTexture)(GL_TEXTURE_2D, self.tex);
            (f.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            (f.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            (f.glTexImage2D)(
                GL_TEXTURE_2D,
                0,
                GL_RGBA as c_int,
                w as c_int,
                h as c_int,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                std::ptr::null(),
            );
            (f.glBindTexture)(GL_TEXTURE_2D, 0);
            (f.glGenFramebuffers)(1, &mut self.fbo);
            (f.glBindFramebuffer)(GL_FRAMEBUFFER, self.fbo);
            (f.glFramebufferTexture2D)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                self.tex,
                0,
            );
            if self.request.depth || self.request.stencil {
                (f.glGenRenderbuffers)(1, &mut self.rbs[0]);
                (f.glBindRenderbuffer)(GL_RENDERBUFFER, self.rbs[0]);
                if self.request.stencil {
                    // GLES 3 has packed depth-stencil; GLES 2 devices here all have
                    // OES_packed_depth_stencil, which is the same enum.
                    (f.glRenderbufferStorage)(
                        GL_RENDERBUFFER,
                        GL_DEPTH24_STENCIL8,
                        w as c_int,
                        h as c_int,
                    );
                    (f.glFramebufferRenderbuffer)(
                        GL_FRAMEBUFFER,
                        GL_DEPTH_ATTACHMENT,
                        GL_RENDERBUFFER,
                        self.rbs[0],
                    );
                    (f.glFramebufferRenderbuffer)(
                        GL_FRAMEBUFFER,
                        GL_STENCIL_ATTACHMENT,
                        GL_RENDERBUFFER,
                        self.rbs[0],
                    );
                } else {
                    (f.glRenderbufferStorage)(
                        GL_RENDERBUFFER,
                        GL_DEPTH_COMPONENT16,
                        w as c_int,
                        h as c_int,
                    );
                    (f.glFramebufferRenderbuffer)(
                        GL_FRAMEBUFFER,
                        GL_DEPTH_ATTACHMENT,
                        GL_RENDERBUFFER,
                        self.rbs[0],
                    );
                }
                (f.glBindRenderbuffer)(GL_RENDERBUFFER, 0);
            }
            if (f.glCheckFramebufferStatus)(GL_FRAMEBUFFER) != GL_FRAMEBUFFER_COMPLETE {
                eprintln!("slot-retro: the GPU core's {w}x{h} framebuffer is incomplete");
            }
        }
        self.size = (w, h);
        self.fbo
    }

    unsafe fn free_framebuffer(&mut self) {
        let f = &self.f;
        if self.fbo != 0 {
            (f.glDeleteFramebuffers)(1, &self.fbo);
        }
        if self.tex != 0 {
            (f.glDeleteTextures)(1, &self.tex);
        }
        for rb in self.rbs.iter().filter(|r| **r != 0) {
            (f.glDeleteRenderbuffers)(1, rb);
        }
        self.fbo = 0;
        self.tex = 0;
        self.rbs = [0; 2];
    }

    /// The core's finished `w` x `h` frame, read back into `self.frame` as XRGB8888, top row
    /// first. Returns the frame's pitch in bytes.
    pub fn read_frame(&mut self, w: u32, h: u32) -> usize {
        let (w, h) = (w.min(self.size.0) as usize, h.min(self.size.1) as usize);
        self.rgba.resize(w * h * 4, 0);
        unsafe {
            let f = &self.f;
            let mut bound = 0;
            (f.glGetIntegerv)(GL_FRAMEBUFFER_BINDING, &mut bound);
            (f.glBindFramebuffer)(GL_FRAMEBUFFER, self.fbo);
            (f.glPixelStorei)(GL_PACK_ALIGNMENT, 4);
            (f.glReadPixels)(
                0,
                0,
                w as c_int,
                h as c_int,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                self.rgba.as_mut_ptr() as *mut c_void,
            );
            (f.glBindFramebuffer)(GL_FRAMEBUFFER, bound as c_uint);
        }
        rgba_to_xrgb(
            &self.rgba,
            w,
            h,
            self.request.bottom_left_origin,
            &mut self.frame,
        );
        w * 4
    }

    /// Tells the core its context is going, while it is still current.
    pub fn destroy(&mut self) {
        if let Some(destroy) = self.request.context_destroy {
            unsafe { destroy() };
        }
    }
}

impl Drop for Gl {
    fn drop(&mut self) {
        unsafe {
            self.free_framebuffer();
            (self.f.eglMakeCurrent)(
                self.display,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            (self.f.eglDestroyContext)(self.display, self.context);
            (self.f.eglDestroySurface)(self.display, self.surface);
        }
    }
}

/// GL's RGBA rows (bottom row first when `bottom_up`) as XRGB8888 (B, G, R, X in memory), top
/// row first.
pub fn rgba_to_xrgb(rgba: &[u8], w: usize, h: usize, bottom_up: bool, out: &mut Vec<u8>) {
    out.resize(w * h * 4, 0);
    for y in 0..h {
        let from = if bottom_up { h - 1 - y } else { y };
        let src = &rgba[from * w * 4..(from + 1) * w * 4];
        let dst = &mut out[y * w * 4..(y + 1) * w * 4];
        for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
            d[0] = s[2];
            d[1] = s[1];
            d[2] = s[0];
            d[3] = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gl_frame_comes_out_top_row_first_in_slots_byte_order() {
        // 1x2: bottom row red, top row blue, as GL reads them (bottom first).
        let rgba = [255, 0, 0, 255, 0, 0, 255, 255];
        let mut out = Vec::new();
        rgba_to_xrgb(&rgba, 1, 2, true, &mut out);
        assert_eq!(
            out,
            [255, 0, 0, 0, 0, 0, 255, 0],
            "blue on top, then red, as B G R X"
        );
        rgba_to_xrgb(&rgba, 1, 2, false, &mut out);
        assert_eq!(
            out,
            [0, 0, 255, 0, 255, 0, 0, 0],
            "a top-left origin keeps its order"
        );
    }

    #[test]
    fn only_gles_contexts_are_accepted() {
        let mut cb = HwRenderCallback {
            context_type: CONTEXT_OPENGLES3,
            context_reset: None,
            get_current_framebuffer: None,
            get_proc_address: None,
            depth: true,
            stencil: false,
            bottom_left_origin: true,
            version_major: 3,
            version_minor: 0,
            cache_context: false,
            context_destroy: None,
            debug_context: false,
        };
        assert!(Request::from_callback(&cb).is_some_and(|r| r.gles3));
        cb.context_type = CONTEXT_OPENGLES2;
        assert!(Request::from_callback(&cb).is_some_and(|r| !r.gles3));
        cb.context_type = CONTEXT_OPENGLES_VERSION;
        cb.version_major = 3;
        assert!(Request::from_callback(&cb).is_some_and(|r| r.gles3));
        // libretro.h's numbers: NONE 0, OPENGL 1, OPENGLES2 2, OPENGL_CORE 3, OPENGLES3 4,
        // OPENGLES_VERSION 5, VULKAN 6, D3D11 7.
        assert_eq!((CONTEXT_OPENGLES2, CONTEXT_OPENGLES3, CONTEXT_OPENGLES_VERSION), (2, 4, 5));
        for desktop in [0, 1, 3, 6, 7] {
            cb.context_type = desktop; // none, OpenGL, OpenGL core, Vulkan, D3D11
            assert!(Request::from_callback(&cb).is_none());
        }
    }
}
