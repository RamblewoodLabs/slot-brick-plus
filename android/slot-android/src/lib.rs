//! Placeholder Slot shelf for the Android scaffold APK.
//!
//! This crate does not yet link `slot-ui` or any libretro core. It exists so we
//! can prove the Android toolchain and show a Slot-like shelf with sample data.

use android_activity::{
    input::{InputEvent, KeyAction, KeyEvent, Keycode},
    AndroidApp, InputStatus, MainEvent, PollEvent,
};
use ndk::hardware_buffer_format::HardwareBufferFormat;
use ndk::native_window::NativeWindow;

const SYSTEMS: &[(&str, u32, u8)] = &[
    ("Game Boy", 0xb8_c0_3c, 8),
    ("Game Boy Color", 0x4a_7c_c7, 8),
    ("Game Boy Advance", 0x5a_3c_8c, 8),
    ("NES", 0x9a_20_20, 10),
    ("SNES", 0x6c_38_8c, 9),
    ("PlayStation", 0x2c_2c_48, 7),
    ("Genesis", 0x20_38_78, 10),
    ("Master System", 0x18_40_80, 8),
    ("Game Gear", 0x18_30_70, 7),
    ("Arcade", 0x18_18_18, 12),
    ("TurboGrafx-16", 0x90_28_28, 8),
    ("Atari 2600", 0x28_28_28, 6),
    ("Intellivision", 0x98_20_28, 7),
    ("Atari Lynx", 0x90_70_18, 6),
    ("Atari 7800", 0x20_20_20, 5),
    ("Game & Watch", 0xc8_c8_c8, 5),
    ("ColecoVision", 0x18_48_28, 6),
    ("Atari 5200", 0x28_28_30, 5),
    ("Atari 800", 0x40_40_48, 5),
    ("Vectrex", 0x20_20_20, 4),
    ("32X", 0x20_38_78, 5),
    ("Nintendo 64", 0x18_40_28, 8),
    ("Nintendo DS", 0xa8_c0_28, 8),
    ("PSP", 0x18_18_20, 8),
    ("GameCube", 0x4c_3c_78, 8),
    ("PlayStation 2", 0x18_28_68, 8),
    ("Switch", 0x28_28_2c, 8),
    ("Nintendo 3DS", 0x98_20_28, 7),
    ("Dreamcast", 0xc8_c8_d0, 7),
    ("Saturn", 0x28_28_30, 6),
];

/// 5×7 caps + digits + a few punctuation marks, packed as 5-bit columns.
fn glyph(c: char) -> Option<[u8; 5]> {
    let c = if c.is_ascii_lowercase() {
        c.to_ascii_uppercase()
    } else {
        c
    };
    Some(match c {
        'A' => [0x0e, 0x11, 0x1f, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x1e, 0x11, 0x1e],
        'C' => [0x0e, 0x11, 0x10, 0x11, 0x0e],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x1e, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x1e, 0x10, 0x10],
        'G' => [0x0e, 0x11, 0x10, 0x13, 0x0e],
        'H' => [0x11, 0x11, 0x1f, 0x11, 0x11],
        'I' => [0x1f, 0x04, 0x04, 0x04, 0x1f],
        'J' => [0x01, 0x01, 0x01, 0x11, 0x0e],
        'K' => [0x11, 0x12, 0x1c, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x1e, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x1e, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x0e, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x15, 0x1b, 0x11],
        'X' => [0x11, 0x0a, 0x04, 0x0a, 0x11],
        'Y' => [0x11, 0x0a, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x02, 0x04, 0x08, 0x1f],
        '0' => [0x0e, 0x13, 0x15, 0x19, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x06, 0x08, 0x1f],
        '3' => [0x1e, 0x01, 0x0e, 0x01, 0x1e],
        '4' => [0x02, 0x06, 0x0a, 0x1f, 0x02],
        '5' => [0x1f, 0x10, 0x1e, 0x01, 0x1e],
        '6' => [0x0e, 0x10, 0x1e, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08],
        '8' => [0x0e, 0x11, 0x0e, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x0f, 0x01, 0x0e],
        ' ' => [0x00, 0x00, 0x00, 0x00, 0x00],
        '&' => [0x0c, 0x12, 0x0c, 0x13, 0x0d],
        '-' => [0x00, 0x00, 0x1f, 0x00, 0x00],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x04],
        ',' => [0x00, 0x00, 0x00, 0x04, 0x08],
        '/' => [0x01, 0x02, 0x04, 0x08, 0x10],
        ':' => [0x00, 0x04, 0x00, 0x04, 0x00],
        '\'' => [0x04, 0x04, 0x00, 0x00, 0x00],
        _ => return None,
    })
}

struct Framebuffer {
    w: i32,
    h: i32,
    stride: i32, // pixels
    bpp: i32,    // 2 or 4
    data: *mut u8,
}

impl Framebuffer {
    fn put(&self, x: i32, y: i32, rgb: u32) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.stride + x) as isize;
        unsafe {
            if self.bpp == 2 {
                let r = ((rgb >> 16) & 0xff) as u16;
                let g = ((rgb >> 8) & 0xff) as u16;
                let b = (rgb & 0xff) as u16;
                let p = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
                (self.data as *mut u16).offset(i).write(p);
            } else {
                let p = self.data.offset(i * 4);
                p.write((rgb & 0xff) as u8);
                p.add(1).write(((rgb >> 8) & 0xff) as u8);
                p.add(2).write(((rgb >> 16) & 0xff) as u8);
                p.add(3).write(0xff);
            }
        }
    }

    fn fill(&self, rgb: u32) {
        for y in 0..self.h {
            for x in 0..self.w {
                self.put(x, y, rgb);
            }
        }
    }

    fn rect(&self, x: i32, y: i32, w: i32, h: i32, rgb: u32) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, rgb);
            }
        }
    }

    fn text(&self, mut x: i32, y: i32, s: &str, scale: i32, rgb: u32) {
        for ch in s.chars() {
            if let Some(cols) = glyph(ch) {
                for (cx, col) in cols.iter().enumerate() {
                    for row in 0..7 {
                        if col & (1 << (6 - row)) != 0 {
                            self.rect(x + cx as i32 * scale, y + row * scale, scale, scale, rgb);
                        }
                    }
                }
                x += 6 * scale;
            } else {
                x += 6 * scale;
            }
        }
    }

    fn text_width(s: &str, scale: i32) -> i32 {
        s.chars().count() as i32 * 6 * scale
    }
}

struct State {
    system: usize,
    selected: usize,
}

impl State {
    fn current_count(&self) -> usize {
        SYSTEMS[self.system].2 as usize
    }

    fn left(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    fn right(&mut self) {
        let n = self.current_count();
        if self.selected + 1 < n {
            self.selected += 1;
        }
    }

    fn prev_system(&mut self) {
        if self.system == 0 {
            self.system = SYSTEMS.len() - 1;
        } else {
            self.system -= 1;
        }
        self.selected = 0;
    }

    fn next_system(&mut self) {
        self.system = (self.system + 1) % SYSTEMS.len();
        self.selected = 0;
    }
}

fn shade(rgb: u32, num: u32, den: u32) -> u32 {
    let r = ((rgb >> 16) & 0xff) * num / den;
    let g = ((rgb >> 8) & 0xff) * num / den;
    let b = (rgb & 0xff) * num / den;
    (r << 16) | (g << 8) | b
}

fn draw_shelf(fb: &Framebuffer, state: &State) {
    let w = fb.w;
    let h = fb.h;
    fb.fill(0x1a120c);

    // ceiling shadow
    fb.rect(0, 0, w, h / 8, 0x120c08);

    let title = "SLOT";
    let ts = (h / 90).max(3);
    let tw = Framebuffer::text_width(title, ts);
    fb.text((w - tw) / 2, h / 18, title, ts, 0xe8dcc0);

    let sub = "ANDROID SCAFFOLD";
    let ss = (h / 140).max(2);
    let sw = Framebuffer::text_width(sub, ss);
    fb.text((w - sw) / 2, h / 18 + 8 * ts, sub, ss, 0x8a7a60);

    // shelf plank
    let shelf_y = h * 62 / 100;
    let plank_h = h / 18;
    fb.rect(0, shelf_y, w, plank_h, 0x5a3a22);
    fb.rect(0, shelf_y, w, 3, 0x7a5434);
    fb.rect(0, shelf_y + plank_h - 4, w, 4, 0x3a2416);

    let (name, color, count) = SYSTEMS[state.system];
    let n = count as i32;
    let cart_h = h * 28 / 100;
    let cart_w = (cart_h * 18 / 28).clamp(w / 16, w / 9);
    let gap = cart_w / 5;
    let total = n * cart_w + (n - 1) * gap;
    let mut x0 = (w - total) / 2;
    if x0 < gap {
        x0 = gap;
    }

    for i in 0..n {
        let x = x0 + i * (cart_w + gap);
        let raised = if i as usize == state.selected { h / 40 } else { 0 };
        let y = shelf_y - cart_h - raised;
        let body = if i as usize == state.selected {
            color
        } else {
            shade(color, 8, 10)
        };
        let side = shade(color, 5, 10);
        let label = 0xf0ead8;
        // thickness / side
        fb.rect(x + cart_w - cart_w / 10, y + 4, cart_w / 10, cart_h - 4, side);
        fb.rect(x, y, cart_w - cart_w / 12, cart_h, body);
        // label window
        let lx = x + cart_w / 8;
        let ly = y + cart_h / 7;
        let lw = cart_w * 5 / 8;
        let lh = cart_h * 11 / 18;
        fb.rect(lx, ly, lw, lh, label);
        // sticker line
        fb.rect(lx + 2, ly + 2, lw - 4, ss.max(2), shade(color, 6, 10));
        // selection glow
        if i as usize == state.selected {
            fb.rect(x - 3, y - 3, cart_w + 6, 3, 0xf2e4b0);
            fb.rect(x - 3, y + cart_h, cart_w + 6, 3, 0xf2e4b0);
        }
    }

    let ls = (h / 110).max(2);
    let label = format!("{name}  {} / {}", state.selected + 1, n);
    let lw = Framebuffer::text_width(&label, ls);
    fb.text((w - lw) / 2, shelf_y + plank_h + ls * 2, &label, ls, 0xf0ead8);

    let hint = "D-PAD  CHANGE GAME   L/R OR UP/DOWN  CHANGE SYSTEM";
    let hs = (h / 160).max(2);
    let hw = Framebuffer::text_width(hint, hs);
    fb.text((w - hw) / 2, h - hs * 10, hint, hs, 0x8a7a60);

    let foot = "PLACEHOLDER DATA  NO CORES YET";
    let fw = Framebuffer::text_width(foot, hs);
    fb.text((w - fw) / 2, h - hs * 5, foot, hs, 0x6a5a44);
}

fn paint(window: &NativeWindow, state: &State) {
    // Ask for a 32-bit buffer so the software shelf can draw RGBA.
    let _ = window.set_buffers_geometry(0, 0, Some(HardwareBufferFormat::R8G8B8A8_UNORM));
    let mut lock = match window.lock(None) {
        Ok(l) => l,
        Err(e) => {
            log::warn!("native window lock failed: {e:?}");
            return;
        }
    };
    let w = lock.width() as i32;
    let h = lock.height() as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let Some(bpp_usz) = lock.format().bytes_per_pixel() else {
        log::warn!("unsupported window format {:?}", lock.format());
        return;
    };
    let bpp = bpp_usz as i32;
    let stride_px = lock.stride() as i32;
    let Some(bytes) = lock.bytes() else {
        log::warn!("no pixel buffer for window format");
        return;
    };
    // Every visible pixel is written by draw_shelf before the lock is posted.
    let data = bytes.as_mut_ptr() as *mut u8;
    let fb = Framebuffer {
        w,
        h,
        stride: stride_px,
        bpp,
        data,
    };
    draw_shelf(&fb, state);
}

fn handle_key(state: &mut State, event: &KeyEvent) -> bool {
    if event.action() != KeyAction::Down {
        return false;
    }
    match event.key_code() {
        Keycode::DpadLeft | Keycode::A => state.left(),
        Keycode::DpadRight | Keycode::D => state.right(),
        Keycode::DpadUp | Keycode::ButtonL1 | Keycode::W => state.prev_system(),
        Keycode::DpadDown | Keycode::ButtonR1 | Keycode::S => state.next_system(),
        Keycode::ButtonA | Keycode::Enter | Keycode::Space => { /* later: launch */ }
        _ => return false,
    }
    true
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("slot"),
    );
    log::info!("Slot Android scaffold starting");

    let mut state = State {
        system: 0,
        selected: 0,
    };
    let mut running = true;
    let mut need_redraw = true;

    while running {
        app.poll_events(
            Some(std::time::Duration::from_millis(16)),
            |event| match event {
                PollEvent::Main(main) => match main {
                    MainEvent::Destroy => running = false,
                    MainEvent::InitWindow { .. }
                    | MainEvent::WindowResized { .. }
                    | MainEvent::RedrawNeeded { .. }
                    | MainEvent::ConfigChanged { .. } => need_redraw = true,
                    MainEvent::InputAvailable => {
                        if let Ok(mut iter) = app.input_events_iter() {
                            loop {
                                let done = !iter.next(|ev| {
                                    if let InputEvent::KeyEvent(key) = ev {
                                        if handle_key(&mut state, key) {
                                            need_redraw = true;
                                        }
                                    }
                                    InputStatus::Handled
                                });
                                if done {
                                    break;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                PollEvent::Timeout => {}
                _ => {}
            },
        );

        if need_redraw {
            if let Some(window) = app.native_window() {
                paint(&window, &state);
            }
            need_redraw = false;
        }
    }
}
