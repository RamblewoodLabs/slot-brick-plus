//! SDL2 + OpenGL ES 2 window for OpenDingux (KMS / Etnaviv).
//!
//! The Slot compositor needs a GLES2 context. If one cannot be created, we
//! return an error; a software-only UI is not wired up for the full shell.
//! The hello OPK still tests the software-renderer fallback on its own.

use crate::surface::{GfxError, Surface};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_WINDOW_FULLSCREEN: u32 = 0x0000_0001;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_ES: c_int = 0x0004;
const SDL_GL_DOUBLEBUFFER: c_int = 5;
const SDL_WINDOWPOS_UNDEFINED: c_int = 0x1FFF_0000;

extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_GetDesktopDisplayMode(index: c_int, mode: *mut SDL_DisplayMode) -> c_int;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut c_void;
    fn SDL_DestroyWindow(window: *mut c_void);
    fn SDL_GL_SetAttribute(attr: c_int, value: c_int) -> c_int;
    fn SDL_GL_CreateContext(window: *mut c_void) -> *mut c_void;
    fn SDL_GL_DeleteContext(ctx: *mut c_void);
    fn SDL_GL_SwapWindow(window: *mut c_void);
    fn SDL_GL_GetProcAddress(name: *const c_char) -> *mut c_void;
    fn SDL_GL_SetSwapInterval(interval: c_int) -> c_int;
    fn SDL_GetWindowSize(window: *mut c_void, w: *mut c_int, h: *mut c_int);
    fn SDL_ShowCursor(toggle: c_int) -> c_int;
}

#[repr(C)]
struct SDL_DisplayMode {
    format: u32,
    w: c_int,
    h: c_int,
    refresh_rate: c_int,
    driverdata: *mut c_void,
}

fn sdl_error() -> String {
    unsafe {
        let p = SDL_GetError();
        if p.is_null() {
            "unknown SDL error".into()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}

pub struct SdlSurface {
    window: *mut c_void,
    context: *mut c_void,
    size: (u32, u32),
}

// The window and context are used from the thread that created them, same as FbdevSurface.
unsafe impl Send for SdlSurface {}

impl SdlSurface {
    pub fn new() -> Result<Self, GfxError> {
        unsafe {
            if SDL_Init(SDL_INIT_VIDEO) != 0 {
                return Err(GfxError::Context(format!("SDL_Init: {}", sdl_error())));
            }
            let _ = SDL_ShowCursor(0);
            SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_ES);
            SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 2);
            SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 0);
            SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

            let mut mode = SDL_DisplayMode {
                format: 0,
                w: 320,
                h: 480,
                refresh_rate: 60,
                driverdata: ptr::null_mut(),
            };
            if SDL_GetDesktopDisplayMode(0, &mut mode) != 0 || mode.w <= 0 || mode.h <= 0 {
                mode.w = 320;
                mode.h = 480;
            }
            eprintln!(
                "slotted: display {}x{} @ {} Hz",
                mode.w, mode.h, mode.refresh_rate
            );

            let title = CString::new("Slotted").unwrap();
            let flags = SDL_WINDOW_FULLSCREEN | SDL_WINDOW_OPENGL | SDL_WINDOW_SHOWN;
            let window = SDL_CreateWindow(
                title.as_ptr(),
                SDL_WINDOWPOS_UNDEFINED,
                SDL_WINDOWPOS_UNDEFINED,
                mode.w,
                mode.h,
                flags,
            );
            if window.is_null() {
                let err = sdl_error();
                SDL_Quit();
                return Err(GfxError::Context(format!("SDL_CreateWindow: {err}")));
            }
            let context = SDL_GL_CreateContext(window);
            if context.is_null() {
                let err = sdl_error();
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err(GfxError::Context(format!(
                    "SDL_GL_CreateContext (GLES2): {err}. The full Slotted shell needs OpenGL ES 2 (KMS/Etnaviv). The hello OPK still tests the software fallback."
                )));
            }
            let _ = SDL_GL_SetSwapInterval(1);
            let mut w = 0;
            let mut h = 0;
            SDL_GetWindowSize(window, &mut w, &mut h);
            let size = (w.max(1) as u32, h.max(1) as u32);
            eprintln!("slotted: GLES2 window {}x{}", size.0, size.1);
            Ok(SdlSurface {
                window,
                context,
                size,
            })
        }
    }

    /// Drop the window and context so a standalone emulator can take KMS.
    pub fn release(self) {
        // Drop runs the same teardown.
        drop(self);
    }
}

impl Drop for SdlSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.context.is_null() {
                SDL_GL_DeleteContext(self.context);
                self.context = ptr::null_mut();
            }
            if !self.window.is_null() {
                SDL_DestroyWindow(self.window);
                self.window = ptr::null_mut();
            }
            SDL_Quit();
        }
    }
}

impl Surface for SdlSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        Ok(())
    }

    fn window_size(&self) -> (u32, u32) {
        self.size
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        unsafe { SDL_GL_SwapWindow(self.window) };
        Ok(())
    }

    fn proc_address(&self, name: &str) -> *const c_void {
        let Ok(c) = CString::new(name) else {
            return ptr::null();
        };
        unsafe { SDL_GL_GetProcAddress(c.as_ptr()) as *const c_void }
    }
}
