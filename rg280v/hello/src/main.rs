//! Slotted hello for the Anbernic RG280V.
//!
//! Tries SDL2 + OpenGL ES 2 first (KMS / Etnaviv on OpenDingux Beta). If the
//! GL context cannot be created, falls back to SDL's software renderer.
//! Face buttons, shoulders and the D-pad light up on screen. Start+Select quits.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;
use std::time::Instant;

const W: i32 = 320;
const H: i32 = 480;

const SDL_INIT_VIDEO: u32 = 0x0000_0020;
const SDL_INIT_EVENTS: u32 = 0x0000_4000;
const SDL_INIT_JOYSTICK: u32 = 0x0000_0200;
const SDL_WINDOW_FULLSCREEN: u32 = 0x0000_0001;
const SDL_WINDOW_OPENGL: u32 = 0x0000_0002;
const SDL_WINDOW_SHOWN: u32 = 0x0000_0004;
const SDL_GL_CONTEXT_MAJOR_VERSION: c_int = 17;
const SDL_GL_CONTEXT_MINOR_VERSION: c_int = 18;
const SDL_GL_CONTEXT_PROFILE_MASK: c_int = 21;
const SDL_GL_CONTEXT_PROFILE_ES: c_int = 0x0004;
const SDL_GL_DOUBLEBUFFER: c_int = 5;
const SDL_PIXELFORMAT_ABGR8888: u32 = 0x1676_2004;
const SDL_TEXTUREACCESS_STREAMING: c_int = 1;

const SDL_QUIT: u32 = 0x100;
const SDL_KEYDOWN: u32 = 0x300;
const SDL_KEYUP: u32 = 0x301;
const SDL_JOYBUTTONDOWN: u32 = 0x603;
const SDL_JOYBUTTONUP: u32 = 0x604;
const SDL_JOYHATMOTION: u32 = 0x602;

const SDLK_SCANCODE_MASK: i32 = 1 << 30;
const SDLK_ESCAPE: i32 = 27;
const SDLK_RETURN: i32 = 13;
const SDLK_SPACE: i32 = 32;
const SDLK_TAB: i32 = 9;
const SDLK_BACKSPACE: i32 = 8;
const SDLK_LCTRL: i32 = SDLK_SCANCODE_MASK | 224;
const SDLK_LSHIFT: i32 = SDLK_SCANCODE_MASK | 225;
const SDLK_LALT: i32 = SDLK_SCANCODE_MASK | 226;
const SDLK_RCTRL: i32 = SDLK_SCANCODE_MASK | 228;
const SDLK_RSHIFT: i32 = SDLK_SCANCODE_MASK | 229;
const SDLK_RIGHT: i32 = SDLK_SCANCODE_MASK | 79;
const SDLK_LEFT: i32 = SDLK_SCANCODE_MASK | 80;
const SDLK_DOWN: i32 = SDLK_SCANCODE_MASK | 81;
const SDLK_UP: i32 = SDLK_SCANCODE_MASK | 82;
const SDLK_HOME: i32 = SDLK_SCANCODE_MASK | 74;
const SDLK_PAGEUP: i32 = SDLK_SCANCODE_MASK | 75;
const SDLK_PAGEDOWN: i32 = SDLK_SCANCODE_MASK | 78;
const SDLK_POWER: i32 = SDLK_SCANCODE_MASK | 102;

const GL_COLOR_BUFFER_BIT: u32 = 0x0000_4000;
const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE0: u32 = 0x84C0;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_TRIANGLES: u32 = 0x0004;
const GL_FLOAT: u32 = 0x1406;
const GL_ARRAY_BUFFER: u32 = 0x8892;
const GL_STATIC_DRAW: u32 = 0x88E4;
const GL_FRAGMENT_SHADER: u32 = 0x8B30;
const GL_VERTEX_SHADER: u32 = 0x8B31;
const GL_COMPILE_STATUS: u32 = 0x8B81;
const GL_LINK_STATUS: u32 = 0x8B82;
const GL_NEAREST: i32 = 0x2600;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_FALSE: u8 = 0;

const VERT: &str = r#"attribute vec2 a_pos;
attribute vec2 a_uv;
varying vec2 v_uv;
void main() {
    v_uv = a_uv;
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const FRAG: &str = r#"precision mediump float;
varying vec2 v_uv;
uniform sampler2D u_tex;
void main() {
    gl_FragColor = texture2D(u_tex, v_uv);
}
"#;

#[repr(C)]
struct SDL_Rect {
    x: c_int,
    y: c_int,
    w: c_int,
    h: c_int,
}

#[link(name = "SDL2")]
extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
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
    fn SDL_CreateRenderer(window: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
    fn SDL_DestroyRenderer(renderer: *mut c_void);
    fn SDL_CreateTexture(
        renderer: *mut c_void,
        format: u32,
        access: c_int,
        w: c_int,
        h: c_int,
    ) -> *mut c_void;
    fn SDL_DestroyTexture(texture: *mut c_void);
    fn SDL_UpdateTexture(
        texture: *mut c_void,
        rect: *const SDL_Rect,
        pixels: *const c_void,
        pitch: c_int,
    ) -> c_int;
    fn SDL_RenderClear(renderer: *mut c_void) -> c_int;
    fn SDL_RenderCopy(
        renderer: *mut c_void,
        texture: *mut c_void,
        src: *const SDL_Rect,
        dst: *const SDL_Rect,
    ) -> c_int;
    fn SDL_RenderPresent(renderer: *mut c_void);
    fn SDL_PollEvent(event: *mut u8) -> c_int;
    fn SDL_Delay(ms: u32);
    fn SDL_NumJoysticks() -> c_int;
    fn SDL_JoystickOpen(index: c_int) -> *mut c_void;
    fn SDL_JoystickName(js: *mut c_void) -> *const c_char;
}

type GlClearColor = unsafe extern "C" fn(f32, f32, f32, f32);
type GlClear = unsafe extern "C" fn(u32);
type GlViewport = unsafe extern "C" fn(c_int, c_int, c_int, c_int);
type GlGenTextures = unsafe extern "C" fn(c_int, *mut u32);
type GlBindTexture = unsafe extern "C" fn(u32, u32);
type GlTexImage2D = unsafe extern "C" fn(u32, c_int, c_int, c_int, c_int, c_int, u32, u32, *const c_void);
type GlTexSubImage2D = unsafe extern "C" fn(u32, c_int, c_int, c_int, c_int, c_int, u32, u32, *const c_void);
type GlTexParameteri = unsafe extern "C" fn(u32, u32, c_int);
type GlCreateShader = unsafe extern "C" fn(u32) -> u32;
type GlShaderSource = unsafe extern "C" fn(u32, c_int, *const *const c_char, *const c_int);
type GlCompileShader = unsafe extern "C" fn(u32);
type GlGetShaderiv = unsafe extern "C" fn(u32, u32, *mut c_int);
type GlGetShaderInfoLog = unsafe extern "C" fn(u32, c_int, *mut c_int, *mut c_char);
type GlCreateProgram = unsafe extern "C" fn() -> u32;
type GlAttachShader = unsafe extern "C" fn(u32, u32);
type GlLinkProgram = unsafe extern "C" fn(u32);
type GlGetProgramiv = unsafe extern "C" fn(u32, u32, *mut c_int);
type GlUseProgram = unsafe extern "C" fn(u32);
type GlGetAttribLocation = unsafe extern "C" fn(u32, *const c_char) -> c_int;
type GlGetUniformLocation = unsafe extern "C" fn(u32, *const c_char) -> c_int;
type GlGenBuffers = unsafe extern "C" fn(c_int, *mut u32);
type GlBindBuffer = unsafe extern "C" fn(u32, u32);
type GlBufferData = unsafe extern "C" fn(u32, isize, *const c_void, u32);
type GlEnableVertexAttribArray = unsafe extern "C" fn(u32);
type GlVertexAttribPointer = unsafe extern "C" fn(u32, c_int, u32, u8, c_int, *const c_void);
type GlUniform1i = unsafe extern "C" fn(c_int, c_int);
type GlActiveTexture = unsafe extern "C" fn(u32);
type GlDrawArrays = unsafe extern "C" fn(u32, c_int, c_int);

struct Gles {
    clear_color: GlClearColor,
    clear: GlClear,
    viewport: GlViewport,
    bind_texture: GlBindTexture,
    tex_sub_image_2d: GlTexSubImage2D,
    use_program: GlUseProgram,
    draw_arrays: GlDrawArrays,
    tex: u32,
    prog: u32,
    vbo: u32,
}

fn gl_proc(name: &str) -> *mut c_void {
    let c = CString::new(name).unwrap();
    unsafe { SDL_GL_GetProcAddress(c.as_ptr()) }
}

fn load_fn<T>(name: &str) -> Option<T> {
    let p = gl_proc(name);
    if p.is_null() {
        eprintln!("slotted-hello: missing GL proc {name}");
        return None;
    }
    Some(unsafe { std::mem::transmute_copy(&p) })
}

struct Buttons {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    b: bool,
    x: bool,
    y: bool,
    l1: bool,
    r1: bool,
    l2: bool,
    r2: bool,
    start: bool,
    select: bool,
    power: bool,
}

impl Buttons {
    fn new() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            a: false,
            b: false,
            x: false,
            y: false,
            l1: false,
            r1: false,
            l2: false,
            r2: false,
            start: false,
            select: false,
            power: false,
        }
    }

    fn apply_key(&mut self, code: i32, down: bool) -> Option<&'static str> {
        let slot = match code {
            SDLK_UP => {
                self.up = down;
                "UP"
            }
            SDLK_DOWN => {
                self.down = down;
                "DOWN"
            }
            SDLK_LEFT => {
                self.left = down;
                "LEFT"
            }
            SDLK_RIGHT => {
                self.right = down;
                "RIGHT"
            }
            SDLK_LCTRL => {
                self.a = down;
                "A"
            }
            SDLK_LALT => {
                self.b = down;
                "B"
            }
            SDLK_LSHIFT => {
                self.x = down;
                "X"
            }
            SDLK_SPACE => {
                self.y = down;
                "Y"
            }
            SDLK_TAB => {
                self.l1 = down;
                "L1"
            }
            SDLK_BACKSPACE => {
                self.r1 = down;
                "R1"
            }
            SDLK_PAGEUP | SDLK_RSHIFT => {
                self.l2 = down;
                "L2"
            }
            SDLK_PAGEDOWN | SDLK_RCTRL => {
                self.r2 = down;
                "R2"
            }
            SDLK_RETURN => {
                self.start = down;
                "START"
            }
            SDLK_ESCAPE | SDLK_HOME => {
                self.select = down;
                "SELECT"
            }
            SDLK_POWER => {
                self.power = down;
                "POWER"
            }
            _ => return None,
        };
        Some(slot)
    }
}

fn sdl_error() -> String {
    unsafe {
        let p = SDL_GetError();
        if p.is_null() {
            return String::new();
        }
        CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

fn key_from_event(ev: &[u8; 64]) -> Option<(i32, bool)> {
    let typ = u32::from_le_bytes(ev[0..4].try_into().unwrap());
    if typ != SDL_KEYDOWN && typ != SDL_KEYUP {
        return None;
    }
    let code = i32::from_le_bytes(ev[20..24].try_into().unwrap());
    Some((code, typ == SDL_KEYDOWN))
}

fn joy_from_event(ev: &[u8; 64]) -> Option<(u8, bool)> {
    let typ = u32::from_le_bytes(ev[0..4].try_into().unwrap());
    if typ != SDL_JOYBUTTONDOWN && typ != SDL_JOYBUTTONUP {
        return None;
    }
    // SDL_JoyButtonEvent: type, timestamp, which (i32), button (u8), state (u8)
    let button = ev[12];
    Some((button, typ == SDL_JOYBUTTONDOWN))
}

fn apply_joy(buttons: &mut Buttons, button: u8, down: bool) -> &'static str {
    // GCW0 / OpenDingux joystick button numbers vary; show both the name we
    // guess and the index in the event line.
    match button {
        0 => {
            buttons.a = down;
            "A"
        }
        1 => {
            buttons.b = down;
            "B"
        }
        2 => {
            buttons.x = down;
            "X"
        }
        3 => {
            buttons.y = down;
            "Y"
        }
        4 => {
            buttons.l1 = down;
            "L1"
        }
        5 => {
            buttons.r1 = down;
            "R1"
        }
        6 => {
            buttons.l2 = down;
            "L2"
        }
        7 => {
            buttons.r2 = down;
            "R2"
        }
        8 => {
            buttons.select = down;
            "SELECT"
        }
        9 => {
            buttons.start = down;
            "START"
        }
        _ => "JOY",
    }
}

// Public-domain 8x8 font (subset of font8x8_basic). Row-major, LSB is the left pixel.
fn glyph(ch: u8) -> [u8; 8] {
    match ch {
        b' ' => [0, 0, 0, 0, 0, 0, 0, 0],
        b'!' => [0x18, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x00],
        b'+' => [0x00, 0x18, 0x18, 0x7E, 0x7E, 0x18, 0x18, 0x00],
        b'-' => [0x00, 0x00, 0x00, 0x7E, 0x7E, 0x00, 0x00, 0x00],
        b'.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
        b'/' => [0x02, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x40, 0x00],
        b'0' => [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
        b'1' => [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        b'2' => [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00],
        b'3' => [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
        b'4' => [0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x00],
        b'5' => [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
        b'6' => [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
        b'7' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x30, 0x00],
        b'8' => [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
        b'9' => [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00],
        b':' => [0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00],
        b'A' => [0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        b'B' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
        b'C' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
        b'D' => [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
        b'E' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x7E, 0x00],
        b'F' => [0x7E, 0x60, 0x60, 0x7C, 0x60, 0x60, 0x60, 0x00],
        b'G' => [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3C, 0x00],
        b'H' => [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
        b'I' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
        b'J' => [0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x6C, 0x38, 0x00],
        b'K' => [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
        b'L' => [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
        b'M' => [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
        b'N' => [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
        b'O' => [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'P' => [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
        b'Q' => [0x3C, 0x66, 0x66, 0x66, 0x6A, 0x6C, 0x36, 0x00],
        b'R' => [0x7C, 0x66, 0x66, 0x7C, 0x78, 0x6C, 0x66, 0x00],
        b'S' => [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
        b'T' => [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
        b'U' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
        b'V' => [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
        b'W' => [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
        b'X' => [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
        b'Y' => [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
        b'Z' => [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
        _ => [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00],
    }
}

fn put_px(fb: &mut [u8], x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= W || y >= H {
        return;
    }
    let i = ((y * W + x) * 4) as usize;
    fb[i..i + 4].copy_from_slice(&rgba);
}

fn fill_rect(fb: &mut [u8], x: i32, y: i32, w: i32, h: i32, rgba: [u8; 4]) {
    for yy in y..y + h {
        for xx in x..x + w {
            put_px(fb, xx, yy, rgba);
        }
    }
}

fn draw_text(fb: &mut [u8], mut x: i32, y: i32, text: &str, rgba: [u8; 4], scale: i32) {
    for ch in text.chars() {
        let g = glyph(ch.to_ascii_uppercase() as u8);
        for row in 0..8 {
            for col in 0..8 {
                if (g[row] >> col) & 1 == 1 {
                    fill_rect(
                        fb,
                        x + col as i32 * scale,
                        y + row as i32 * scale,
                        scale,
                        scale,
                        rgba,
                    );
                }
            }
        }
        x += 8 * scale;
    }
}

fn pad_cell(
    fb: &mut [u8],
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &str,
    on: bool,
) {
    let bg = if on {
        [240, 200, 40, 255]
    } else {
        [40, 48, 58, 255]
    };
    let fg = if on {
        [20, 20, 20, 255]
    } else {
        [220, 220, 220, 255]
    };
    fill_rect(fb, x, y, w, h, bg);
    fill_rect(fb, x, y, w, 2, [10, 10, 10, 255]);
    fill_rect(fb, x, y + h - 2, w, 2, [10, 10, 10, 255]);
    let tw = label.len() as i32 * 8;
    draw_text(fb, x + (w - tw).max(0) / 2, y + (h - 8).max(0) / 2, label, fg, 1);
}

fn compose(fb: &mut [u8], gles: bool, buttons: &Buttons, event_line: &str, frames: u32) {
    let bg = if gles {
        [16, 92, 96, 255]
    } else {
        [72, 28, 88, 255]
    };
    fill_rect(fb, 0, 0, W, H, bg);
    fill_rect(fb, 0, 0, W, 56, [16, 18, 22, 255]);
    draw_text(fb, 16, 10, "SLOTTED", [250, 230, 80, 255], 3);
    draw_text(fb, 16, 40, "RG280V HELLO", [200, 200, 200, 255], 1);

    let mode = if gles { "PATH: GLES2 / ETNAVIV" } else { "PATH: SDL SOFTWARE" };
    draw_text(fb, 16, 72, mode, [255, 255, 255, 255], 1);
    draw_text(fb, 16, 88, "START+SELECT QUITS", [200, 200, 200, 255], 1);

    pad_cell(fb, 56, 130, 48, 36, "UP", buttons.up);
    pad_cell(fb, 8, 170, 48, 36, "LEFT", buttons.left);
    pad_cell(fb, 56, 170, 48, 36, "DOWN", buttons.down);
    pad_cell(fb, 104, 170, 48, 36, "RIGHT", buttons.right);

    pad_cell(fb, 200, 130, 44, 36, "X", buttons.x);
    pad_cell(fb, 248, 170, 44, 36, "A", buttons.a);
    pad_cell(fb, 200, 170, 44, 36, "B", buttons.b);
    pad_cell(fb, 152, 170, 44, 36, "Y", buttons.y);

    pad_cell(fb, 16, 230, 64, 32, "L1", buttons.l1);
    pad_cell(fb, 88, 230, 64, 32, "L2", buttons.l2);
    pad_cell(fb, 168, 230, 64, 32, "R2", buttons.r2);
    pad_cell(fb, 240, 230, 64, 32, "R1", buttons.r1);

    pad_cell(fb, 48, 280, 100, 36, "SELECT", buttons.select);
    pad_cell(fb, 172, 280, 100, 36, "START", buttons.start);
    pad_cell(fb, 110, 328, 100, 32, "POWER", buttons.power);

    let ev = if event_line.is_empty() { "NO EVENT YET" } else { event_line };
    draw_text(fb, 16, 380, ev, [255, 255, 255, 255], 1);
    draw_text(fb, 16, 400, &format!("FRAME {frames}"), [180, 180, 180, 255], 1);
    draw_text(fb, 16, 440, "PRESS EVERY BUTTON", [220, 220, 180, 255], 1);
}

fn compile_gl_shader(
    create: GlCreateShader,
    source: GlShaderSource,
    compile: GlCompileShader,
    get_iv: GlGetShaderiv,
    get_log: GlGetShaderInfoLog,
    kind: u32,
    src: &str,
) -> Option<u32> {
    let id = unsafe { create(kind) };
    if id == 0 {
        return None;
    }
    let c = CString::new(src).ok()?;
    let ptr = c.as_ptr();
    unsafe {
        source(id, 1, &ptr, ptr::null());
        compile(id);
        let mut ok = 0;
        get_iv(id, GL_COMPILE_STATUS, &mut ok);
        if ok == 0 {
            let mut buf = [0u8; 256];
            get_log(id, buf.len() as c_int, ptr::null_mut(), buf.as_mut_ptr() as *mut c_char);
            eprintln!(
                "slotted-hello: shader: {}",
                String::from_utf8_lossy(&buf)
            );
            return None;
        }
    }
    Some(id)
}

fn setup_gles(w: i32, h: i32) -> Option<Gles> {
    let clear_color: GlClearColor = load_fn("glClearColor")?;
    let clear: GlClear = load_fn("glClear")?;
    let viewport: GlViewport = load_fn("glViewport")?;
    let gen_textures: GlGenTextures = load_fn("glGenTextures")?;
    let bind_texture: GlBindTexture = load_fn("glBindTexture")?;
    let tex_image_2d: GlTexImage2D = load_fn("glTexImage2D")?;
    let tex_sub_image_2d: GlTexSubImage2D = load_fn("glTexSubImage2D")?;
    let tex_param: GlTexParameteri = load_fn("glTexParameteri")?;
    let create_shader: GlCreateShader = load_fn("glCreateShader")?;
    let shader_source: GlShaderSource = load_fn("glShaderSource")?;
    let compile_shader: GlCompileShader = load_fn("glCompileShader")?;
    let get_shader_iv: GlGetShaderiv = load_fn("glGetShaderiv")?;
    let get_shader_log: GlGetShaderInfoLog = load_fn("glGetShaderInfoLog")?;
    let create_program: GlCreateProgram = load_fn("glCreateProgram")?;
    let attach: GlAttachShader = load_fn("glAttachShader")?;
    let link: GlLinkProgram = load_fn("glLinkProgram")?;
    let get_program_iv: GlGetProgramiv = load_fn("glGetProgramiv")?;
    let use_program: GlUseProgram = load_fn("glUseProgram")?;
    let get_attrib: GlGetAttribLocation = load_fn("glGetAttribLocation")?;
    let get_uniform: GlGetUniformLocation = load_fn("glGetUniformLocation")?;
    let gen_buffers: GlGenBuffers = load_fn("glGenBuffers")?;
    let bind_buffer: GlBindBuffer = load_fn("glBindBuffer")?;
    let buffer_data: GlBufferData = load_fn("glBufferData")?;
    let enable_attrib: GlEnableVertexAttribArray = load_fn("glEnableVertexAttribArray")?;
    let attrib_ptr: GlVertexAttribPointer = load_fn("glVertexAttribPointer")?;
    let uniform1i: GlUniform1i = load_fn("glUniform1i")?;
    let active_texture: GlActiveTexture = load_fn("glActiveTexture")?;
    let draw_arrays: GlDrawArrays = load_fn("glDrawArrays")?;

    let vs = compile_gl_shader(
        create_shader,
        shader_source,
        compile_shader,
        get_shader_iv,
        get_shader_log,
        GL_VERTEX_SHADER,
        VERT,
    )?;
    let fs = compile_gl_shader(
        create_shader,
        shader_source,
        compile_shader,
        get_shader_iv,
        get_shader_log,
        GL_FRAGMENT_SHADER,
        FRAG,
    )?;
    let prog = unsafe { create_program() };
    let mut ok = 0;
    unsafe {
        attach(prog, vs);
        attach(prog, fs);
        link(prog);
        get_program_iv(prog, GL_LINK_STATUS, &mut ok);
    }
    if ok == 0 {
        eprintln!("slotted-hello: GLES program did not link");
        return None;
    }

    let mut tex = 0u32;
    let mut vbo = 0u32;
    // x, y, u, v  — two triangles covering the screen, Y flipped for GL.
    let verts: [f32; 24] = [
        -1.0, -1.0, 0.0, 1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, 0.0, 0.0, -1.0, 1.0, 0.0, 0.0, 1.0,
        -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.0,
    ];
    unsafe {
        viewport(0, 0, w, h);
        gen_textures(1, &mut tex);
        bind_texture(GL_TEXTURE_2D, tex);
        tex_param(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
        tex_param(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
        tex_param(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        tex_param(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
        tex_image_2d(
            GL_TEXTURE_2D,
            0,
            GL_RGBA as c_int,
            w,
            h,
            0,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            ptr::null(),
        );
        gen_buffers(1, &mut vbo);
        bind_buffer(GL_ARRAY_BUFFER, vbo);
        buffer_data(
            GL_ARRAY_BUFFER,
            (verts.len() * 4) as isize,
            verts.as_ptr() as *const c_void,
            GL_STATIC_DRAW,
        );
        use_program(prog);
        let a_pos = get_attrib(prog, b"a_pos\0".as_ptr() as *const c_char);
        let a_uv = get_attrib(prog, b"a_uv\0".as_ptr() as *const c_char);
        let u_tex = get_uniform(prog, b"u_tex\0".as_ptr() as *const c_char);
        if a_pos < 0 || a_uv < 0 {
            eprintln!("slotted-hello: attribs missing");
            return None;
        }
        enable_attrib(a_pos as u32);
        enable_attrib(a_uv as u32);
        attrib_ptr(a_pos as u32, 2, GL_FLOAT, GL_FALSE, 16, ptr::null());
        attrib_ptr(
            a_uv as u32,
            2,
            GL_FLOAT,
            GL_FALSE,
            16,
            (8isize) as *const c_void,
        );
        active_texture(GL_TEXTURE0);
        uniform1i(u_tex, 0);
    }
    Some(Gles {
        clear_color,
        clear,
        viewport,
        bind_texture,
        tex_sub_image_2d,
        use_program,
        draw_arrays,
        tex,
        prog,
        vbo,
    })
}

fn present_gles(gles: &Gles, fb: &[u8], w: i32, h: i32) {
    unsafe {
        (gles.viewport)(0, 0, w, h);
        (gles.clear_color)(0.0, 0.0, 0.0, 1.0);
        (gles.clear)(GL_COLOR_BUFFER_BIT);
        (gles.bind_texture)(GL_TEXTURE_2D, gles.tex);
        (gles.tex_sub_image_2d)(
            GL_TEXTURE_2D,
            0,
            0,
            0,
            w,
            h,
            GL_RGBA,
            GL_UNSIGNED_BYTE,
            fb.as_ptr() as *const c_void,
        );
        (gles.use_program)(gles.prog);
        (gles.draw_arrays)(GL_TRIANGLES, 0, 6);
    }
    let _ = gles.vbo;
}

fn main() {
    unsafe {
        if SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS | SDL_INIT_JOYSTICK) != 0 {
            eprintln!("slotted-hello: SDL_Init failed: {}", sdl_error());
            std::process::exit(1);
        }

        let njs = SDL_NumJoysticks();
        for i in 0..njs {
            let js = SDL_JoystickOpen(i);
            let name = if js.is_null() {
                "open failed".into()
            } else {
                let p = SDL_JoystickName(js);
                if p.is_null() {
                    "unnamed".into()
                } else {
                    CStr::from_ptr(p).to_string_lossy().into_owned()
                }
            };
            eprintln!("slotted-hello: joystick {i}: {name}");
        }

        let title = CString::new("Slotted Hello").unwrap();
        let mut gles_ok = false;
        let _ = SDL_GL_SetAttribute(SDL_GL_CONTEXT_PROFILE_MASK, SDL_GL_CONTEXT_PROFILE_ES);
        let _ = SDL_GL_SetAttribute(SDL_GL_CONTEXT_MAJOR_VERSION, 2);
        let _ = SDL_GL_SetAttribute(SDL_GL_CONTEXT_MINOR_VERSION, 0);
        let _ = SDL_GL_SetAttribute(SDL_GL_DOUBLEBUFFER, 1);

        let flags = SDL_WINDOW_SHOWN | SDL_WINDOW_FULLSCREEN | SDL_WINDOW_OPENGL;
        let window = SDL_CreateWindow(title.as_ptr(), 0, 0, W, H, flags);
        if window.is_null() {
            eprintln!("slotted-hello: SDL_CreateWindow failed: {}", sdl_error());
            SDL_Quit();
            std::process::exit(1);
        }

        let gl_ctx = SDL_GL_CreateContext(window);
        let mut gles = None;
        if !gl_ctx.is_null() {
            match setup_gles(W, H) {
                Some(g) => {
                    gles_ok = true;
                    gles = Some(g);
                    eprintln!("slotted-hello: GLES2 context ok");
                }
                None => {
                    eprintln!("slotted-hello: GLES2 shaders failed, falling back to software");
                    SDL_GL_DeleteContext(gl_ctx);
                }
            }
        } else {
            eprintln!("slotted-hello: SDL_GL_CreateContext failed: {}", sdl_error());
        }

        let mut renderer = ptr::null_mut();
        let mut texture = ptr::null_mut();
        if !gles_ok {
            renderer = SDL_CreateRenderer(window, -1, 0);
            if renderer.is_null() {
                eprintln!("slotted-hello: SDL_CreateRenderer failed: {}", sdl_error());
                SDL_DestroyWindow(window);
                SDL_Quit();
                std::process::exit(1);
            }
            texture = SDL_CreateTexture(
                renderer,
                SDL_PIXELFORMAT_ABGR8888,
                SDL_TEXTUREACCESS_STREAMING,
                W,
                H,
            );
            if texture.is_null() {
                eprintln!("slotted-hello: SDL_CreateTexture failed: {}", sdl_error());
                SDL_DestroyRenderer(renderer);
                SDL_DestroyWindow(window);
                SDL_Quit();
                std::process::exit(1);
            }
            eprintln!("slotted-hello: software renderer ok");
        }

        let mut fb = vec![0u8; (W * H * 4) as usize];
        let mut buttons = Buttons::new();
        let mut event_line = String::from("NO EVENT YET");
        let mut frames = 0u32;
        let start = Instant::now();
        let mut running = true;

        while running {
            let mut ev = [0u8; 64];
            while SDL_PollEvent(ev.as_mut_ptr()) != 0 {
                let typ = u32::from_le_bytes(ev[0..4].try_into().unwrap());
                if typ == SDL_QUIT {
                    running = false;
                    event_line = "QUIT".into();
                    continue;
                }
                if typ == SDL_JOYHATMOTION {
                    // value at offset 16? SDL_JoyHatEvent: type, timestamp, which, hat, value
                    let value = ev[13];
                    buttons.up = value & 0x01 != 0;
                    buttons.right = value & 0x02 != 0;
                    buttons.down = value & 0x04 != 0;
                    buttons.left = value & 0x08 != 0;
                    event_line = format!("HAT {value}");
                    continue;
                }
                if let Some((code, down)) = key_from_event(&ev) {
                    let name = buttons.apply_key(code, down).unwrap_or("KEY");
                    event_line = format!(
                        "{name} {}  SYM {code}",
                        if down { "DOWN" } else { "UP" }
                    );
                    continue;
                }
                if let Some((button, down)) = joy_from_event(&ev) {
                    let name = apply_joy(&mut buttons, button, down);
                    event_line = format!(
                        "{name} {}  JOY {button}",
                        if down { "DOWN" } else { "UP" }
                    );
                }
            }
            if buttons.start && buttons.select {
                running = false;
            }

            compose(&mut fb, gles_ok, &buttons, &event_line, frames);
            if let Some(ref g) = gles {
                present_gles(g, &fb, W, H);
                SDL_GL_SwapWindow(window);
            } else {
                SDL_UpdateTexture(texture, ptr::null(), fb.as_ptr() as *const c_void, W * 4);
                SDL_RenderClear(renderer);
                SDL_RenderCopy(renderer, texture, ptr::null(), ptr::null());
                SDL_RenderPresent(renderer);
            }
            frames = frames.wrapping_add(1);
            SDL_Delay(16);
        }

        eprintln!(
            "slotted-hello: exit after {frames} frames, {:?}",
            start.elapsed()
        );
        if !texture.is_null() {
            SDL_DestroyTexture(texture);
        }
        if !renderer.is_null() {
            SDL_DestroyRenderer(renderer);
        }
        if !gl_ctx.is_null() && gles_ok {
            SDL_GL_DeleteContext(gl_ctx);
        }
        SDL_DestroyWindow(window);
        SDL_Quit();
    }
}
