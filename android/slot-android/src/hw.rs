//! GLES hardware-render support for libretro cores that draw on the GPU
//! (mupen64plus-next / GLideN64, PPSSPP).
//!
//! Unlike the handheld host, this does not create its own EGL pbuffer:
//! PlayActivity already has a GLES3 context current on the GL thread
//! where the core runs. We allocate an FBO for the core, hand back
//! proc addresses, and expose the color texture so the Java renderer
//! can composite it to the surface.

use libloading::os::unix::{Library, RTLD_GLOBAL, RTLD_NOW};
use std::ffi::{c_char, c_int, c_uint, c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

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

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Request {
    pub gles3: bool, // used when choosing the context flavor
    pub depth: bool,
    pub stencil: bool,
    pub bottom_left_origin: bool,
    pub context_reset: Option<unsafe extern "C" fn()>,
    pub context_destroy: Option<unsafe extern "C" fn()>,
}

impl Request {
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

/// The FBO id the core should draw into; read from the GET_CURRENT_FRAMEBUFFER
/// callback without borrowing the Host (cores may call it reentrantly).
pub static CURRENT_FBO: AtomicUsize = AtomicUsize::new(0);

fn open_any(names: &[&str]) -> Option<Library> {
    names
        .iter()
        .find_map(|n| unsafe { Library::open(Some(*n), RTLD_NOW | RTLD_GLOBAL) }.ok())
}

/// Load the device's EGL and GLES globally before a GPU core is dlopened,
/// so unresolved GL relocations bind to the system libraries.
pub fn preload() -> bool {
    LIBS.get_or_init(|| {
        let egl = open_any(&["libEGL.so", "libEGL.so.1"])?;
        let gles = open_any(&["libGLESv2.so", "libGLESv3.so", "libGLESv2.so.2"])?;
        Some(GlLibs { egl, gles })
    })
    .is_some()
}

fn libs() -> Option<&'static GlLibs> {
    LIBS.get().and_then(|l| l.as_ref())
}

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

pub unsafe extern "C" fn hw_get_current_framebuffer() -> usize {
    CURRENT_FBO.load(Ordering::Acquire)
}

pub unsafe extern "C" fn hw_get_proc_address(name: *const c_char) -> ProcAddress {
    if name.is_null() {
        return None;
    }
    proc_address(CStr::from_ptr(name))
}

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
const GL_FRAMEBUFFER_COMPLETE: c_uint = 0x8CD5;
const GL_TEXTURE_MIN_FILTER: c_uint = 0x2801;
const GL_TEXTURE_MAG_FILTER: c_uint = 0x2800;
const GL_NEAREST: c_int = 0x2600;

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
    glDeleteFramebuffers: fn(c_int, *const c_uint);
    glDeleteTextures: fn(c_int, *const c_uint);
    glDeleteRenderbuffers: fn(c_int, *const c_uint);
}

/// The core's off-screen framebuffer in the current GLSurfaceView context.
pub struct Gl {
    f: Fns,
    pub request: Request,
    fbo: c_uint,
    tex: c_uint,
    rbs: [c_uint; 2],
    size: (u32, u32),
}

impl Gl {
    pub fn attach(request: Request) -> Option<Gl> {
        if !preload() {
            return None;
        }
        let f = unsafe { Fns::load()? };
        Some(Gl {
            f,
            request,
            fbo: 0,
            tex: 0,
            rbs: [0, 0],
            size: (0, 0),
        })
    }

    pub fn color_texture(&self) -> c_uint {
        self.tex
    }

    pub fn framebuffer(&mut self, w: u32, h: u32) -> c_uint {
        let w = w.max(1);
        let h = h.max(1);
        if self.fbo != 0 && self.size == (w, h) {
            return self.fbo;
        }
        self.destroy_buffers();
        unsafe {
            let f = &self.f;
            let mut tex = 0;
            (f.glGenTextures)(1, &mut tex);
            (f.glBindTexture)(GL_TEXTURE_2D, tex);
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
            (f.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            (f.glTexParameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);

            let mut fbo = 0;
            (f.glGenFramebuffers)(1, &mut fbo);
            (f.glBindFramebuffer)(GL_FRAMEBUFFER, fbo);
            (f.glFramebufferTexture2D)(GL_FRAMEBUFFER, GL_COLOR_ATTACHMENT0, GL_TEXTURE_2D, tex, 0);

            let mut rbs = [0u32; 2];
            if self.request.depth || self.request.stencil {
                (f.glGenRenderbuffers)(1, &mut rbs[0]);
                (f.glBindRenderbuffer)(GL_RENDERBUFFER, rbs[0]);
                if self.request.stencil {
                    (f.glRenderbufferStorage)(GL_RENDERBUFFER, GL_DEPTH24_STENCIL8, w as c_int, h as c_int);
                    (f.glFramebufferRenderbuffer)(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, rbs[0]);
                    (f.glFramebufferRenderbuffer)(GL_FRAMEBUFFER, GL_STENCIL_ATTACHMENT, GL_RENDERBUFFER, rbs[0]);
                } else {
                    (f.glRenderbufferStorage)(GL_RENDERBUFFER, GL_DEPTH_COMPONENT16, w as c_int, h as c_int);
                    (f.glFramebufferRenderbuffer)(GL_FRAMEBUFFER, GL_DEPTH_ATTACHMENT, GL_RENDERBUFFER, rbs[0]);
                }
            }
            let status = (f.glCheckFramebufferStatus)(GL_FRAMEBUFFER);
            (f.glBindFramebuffer)(GL_FRAMEBUFFER, 0);
            (f.glBindTexture)(GL_TEXTURE_2D, 0);
            if status != GL_FRAMEBUFFER_COMPLETE {
                log::error!("libretro HW FBO incomplete: 0x{status:x}");
                if fbo != 0 {
                    (f.glDeleteFramebuffers)(1, &fbo);
                }
                if tex != 0 {
                    (f.glDeleteTextures)(1, &tex);
                }
                if rbs[0] != 0 {
                    (f.glDeleteRenderbuffers)(1, &rbs[0]);
                }
                CURRENT_FBO.store(0, Ordering::Release);
                return 0;
            }
            self.fbo = fbo;
            self.tex = tex;
            self.rbs = rbs;
            self.size = (w, h);
            CURRENT_FBO.store(fbo as usize, Ordering::Release);
            fbo
        }
    }

    pub fn unbind(&self) {
        unsafe { (self.f.glBindFramebuffer)(GL_FRAMEBUFFER, 0) };
    }

    fn destroy_buffers(&mut self) {
        unsafe {
            let f = &self.f;
            if self.fbo != 0 {
                (f.glDeleteFramebuffers)(1, &self.fbo);
                self.fbo = 0;
            }
            if self.tex != 0 {
                (f.glDeleteTextures)(1, &self.tex);
                self.tex = 0;
            }
            if self.rbs[0] != 0 {
                (f.glDeleteRenderbuffers)(1, &self.rbs[0]);
                self.rbs[0] = 0;
            }
        }
        CURRENT_FBO.store(0, Ordering::Release);
        self.size = (0, 0);
    }
}

impl Drop for Gl {
    fn drop(&mut self) {
        if let Some(destroy) = self.request.context_destroy {
            unsafe { destroy() };
        }
        self.destroy_buffers();
    }
}
