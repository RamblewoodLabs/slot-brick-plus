//! A minimal libretro host for Slotted on Android.
//!
//! Software-rendered cores (the first wave: mGBA, FCEUmm, Snes9x,
//! Genesis Plus GX, PCSX ReARMed) write a framebuffer and an audio
//! batch each frame. GLES upload and AudioTrack playback happen on the
//! Java side. SRAM is flushed to the save directory on stop; save
//! states are files in the same directory.
use libloading::{Library, Symbol};
use log::{info, warn};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::ffi::{c_char, c_uint, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

pub const EMBEDDED_CORES: &[&str] = &[
    "mgba",
    "fceumm",
    "snes9x",
    "genesis_plus_gx",
    "pcsx_rearmed",
];

pub fn is_embedded(core: &str) -> bool {
    EMBEDDED_CORES.iter().any(|c| *c == core)
}

pub fn bundled_so_name(core: &str) -> String {
    format!("lib{core}_libretro.so")
}

const PIXEL_0RGB1555: u32 = 0;
const PIXEL_XRGB8888: u32 = 1;
const PIXEL_RGB565: u32 = 2;

const ENV_GET_CAN_DUPE: u32 = 3;
const ENV_SET_MESSAGE: u32 = 6;
const ENV_SET_PERFORMANCE_LEVEL: u32 = 8;
const ENV_GET_SYSTEM_DIRECTORY: u32 = 9;
const ENV_SET_PIXEL_FORMAT: u32 = 10;
const ENV_SET_INPUT_DESCRIPTORS: u32 = 11;
const ENV_GET_VARIABLE: u32 = 15;
const ENV_SET_VARIABLES: u32 = 16;
const ENV_GET_VARIABLE_UPDATE: u32 = 17;
const ENV_SET_SUPPORT_NO_GAME: u32 = 18;
const ENV_GET_LOG_INTERFACE: u32 = 27;
const ENV_GET_CORE_ASSETS_DIRECTORY: u32 = 30;
const ENV_GET_SAVE_DIRECTORY: u32 = 31;
const ENV_SET_SYSTEM_AV_INFO: u32 = 32;
const ENV_SET_CONTROLLER_INFO: u32 = 35;
const ENV_SET_MEMORY_MAPS: u32 = 36;
const ENV_SET_GEOMETRY: u32 = 37;
const ENV_GET_CORE_OPTIONS_VERSION: u32 = 52;
const ENV_SET_CORE_OPTIONS: u32 = 53;
const ENV_SET_CORE_OPTIONS_INTL: u32 = 54;
const ENV_SET_CORE_OPTIONS_DISPLAY: u32 = 55;
const ENV_SET_CORE_OPTIONS_V2: u32 = 67;
const ENV_SET_CORE_OPTIONS_V2_INTL: u32 = 68;
const ENV_SET_SUPPORT_ACHIEVEMENTS: u32 = 42;

const MEMORY_SAVE_RAM: u32 = 0;

#[repr(C)]
struct RetroSystemInfo {
    library_name: *const c_char,
    library_version: *const c_char,
    valid_extensions: *const c_char,
    need_fullpath: bool,
    block_extract: bool,
}

#[repr(C)]
struct RetroGameGeometry {
    base_width: c_uint,
    base_height: c_uint,
    max_width: c_uint,
    max_height: c_uint,
    aspect_ratio: f32,
}

#[repr(C)]
struct RetroSystemTiming {
    fps: f64,
    sample_rate: f64,
}

#[repr(C)]
struct RetroSystemAvInfo {
    geometry: RetroGameGeometry,
    timing: RetroSystemTiming,
}

#[repr(C)]
struct RetroGameInfo {
    path: *const c_char,
    data: *const c_void,
    size: usize,
    meta: *const c_char,
}

#[repr(C)]
struct RetroVariable {
    key: *const c_char,
    value: *const c_char,
}

#[repr(C)]
struct RetroMessage {
    msg: *const c_char,
    frames: c_uint,
}

type EnvFn = unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool;
type VideoFn = unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize);
type AudioFn = unsafe extern "C" fn(left: i16, right: i16);
type AudioBatchFn = unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
type InputPollFn = unsafe extern "C" fn();
type InputStateFn = unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16;

struct Api {
    _lib: Library,
    init: Symbol<'static, unsafe extern "C" fn()>,
    deinit: Symbol<'static, unsafe extern "C" fn()>,
    api_version: Symbol<'static, unsafe extern "C" fn() -> c_uint>,
    get_system_info: Symbol<'static, unsafe extern "C" fn(*mut RetroSystemInfo)>,
    get_system_av_info: Symbol<'static, unsafe extern "C" fn(*mut RetroSystemAvInfo)>,
    set_environment: Symbol<'static, unsafe extern "C" fn(EnvFn)>,
    set_video_refresh: Symbol<'static, unsafe extern "C" fn(VideoFn)>,
    set_audio_sample: Symbol<'static, unsafe extern "C" fn(AudioFn)>,
    set_audio_sample_batch: Symbol<'static, unsafe extern "C" fn(AudioBatchFn)>,
    set_input_poll: Symbol<'static, unsafe extern "C" fn(InputPollFn)>,
    set_input_state: Symbol<'static, unsafe extern "C" fn(InputStateFn)>,
    set_controller_port_device: Symbol<'static, unsafe extern "C" fn(c_uint, c_uint)>,
    load_game: Symbol<'static, unsafe extern "C" fn(*const RetroGameInfo) -> bool>,
    unload_game: Symbol<'static, unsafe extern "C" fn()>,
    run: Symbol<'static, unsafe extern "C" fn()>,
    reset: Symbol<'static, unsafe extern "C" fn()>,
    serialize_size: Symbol<'static, unsafe extern "C" fn() -> usize>,
    serialize: Symbol<'static, unsafe extern "C" fn(*mut c_void, usize) -> bool>,
    unserialize: Symbol<'static, unsafe extern "C" fn(*const c_void, usize) -> bool>,
    get_memory_data: Symbol<'static, unsafe extern "C" fn(c_uint) -> *mut c_void>,
    get_memory_size: Symbol<'static, unsafe extern "C" fn(c_uint) -> usize>,
}

impl Api {
    unsafe fn load(path: &Path) -> Result<Self, String> {
        let lib = Library::new(path).map_err(|e| format!("open {}: {e}", path.display()))?;
        // Extend the library's lifetime: we keep Library in the same struct.
        let lib_ref: &'static Library = &*(&lib as *const Library);
        unsafe fn sym<'a, T>(lib: &'a Library, name: &str) -> Result<Symbol<'static, T>, String> {
            let s: Symbol<T> = lib.get(name.as_bytes()).map_err(|e| format!("{name}: {e}"))?;
            Ok(std::mem::transmute(s))
        }
        let api = Api {
            init: sym(lib_ref, "retro_init")?,
            deinit: sym(lib_ref, "retro_deinit")?,
            api_version: sym(lib_ref, "retro_api_version")?,
            get_system_info: sym(lib_ref, "retro_get_system_info")?,
            get_system_av_info: sym(lib_ref, "retro_get_system_av_info")?,
            set_environment: sym(lib_ref, "retro_set_environment")?,
            set_video_refresh: sym(lib_ref, "retro_set_video_refresh")?,
            set_audio_sample: sym(lib_ref, "retro_set_audio_sample")?,
            set_audio_sample_batch: sym(lib_ref, "retro_set_audio_sample_batch")?,
            set_input_poll: sym(lib_ref, "retro_set_input_poll")?,
            set_input_state: sym(lib_ref, "retro_set_input_state")?,
            set_controller_port_device: sym(lib_ref, "retro_set_controller_port_device")?,
            load_game: sym(lib_ref, "retro_load_game")?,
            unload_game: sym(lib_ref, "retro_unload_game")?,
            run: sym(lib_ref, "retro_run")?,
            reset: sym(lib_ref, "retro_reset")?,
            serialize_size: sym(lib_ref, "retro_serialize_size")?,
            serialize: sym(lib_ref, "retro_serialize")?,
            unserialize: sym(lib_ref, "retro_unserialize")?,
            get_memory_data: sym(lib_ref, "retro_get_memory_data")?,
            get_memory_size: sym(lib_ref, "retro_get_memory_size")?,
            _lib: lib,
        };
        Ok(api)
    }
}

pub struct Host {
    api: Api,
    system_dir: CString,
    save_dir: CString,
    save_dir_path: PathBuf,
    sram_path: PathBuf,
    pixel_format: u32,
    width: u32,
    height: u32,
    pitch: usize,
    fps: f64,
    sample_rate: f64,
    frame: Vec<u8>,
    audio: VecDeque<i16>,
    buttons: u16,
    options: HashMap<String, String>,
    option_dirty: bool,
    option_value: Option<CString>,
    loaded: bool,
    rom_buf: Option<Vec<u8>>,
    rom_path: CString,
}

thread_local! {
    static CURRENT: RefCell<Option<Host>> = const { RefCell::new(None) };
}

impl Host {
    pub fn start(core_so: &Path, rom_path: &Path, system_dir: &Path, save_dir: &Path) -> Result<(), String> {
        stop();
        std::fs::create_dir_all(system_dir).ok();
        std::fs::create_dir_all(save_dir).ok();
        let api = unsafe { Api::load(core_so)? };
        let version = unsafe { (api.api_version)() };
        if version != 1 {
            return Err(format!("unsupported libretro API {version}"));
        }
        let stem = rom_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("game");
        let sram_path = save_dir.join(format!("{stem}.srm"));
        let host = Host {
            api,
            system_dir: cstring(system_dir),
            save_dir: cstring(save_dir),
            save_dir_path: save_dir.to_path_buf(),
            sram_path,
            pixel_format: PIXEL_RGB565,
            width: 0,
            height: 0,
            pitch: 0,
            fps: 60.0,
            sample_rate: 44100.0,
            frame: Vec::new(),
            audio: VecDeque::new(),
            buttons: 0,
            options: HashMap::new(),
            option_dirty: false,
            option_value: None,
            loaded: false,
            rom_buf: None,
            rom_path: cstring(rom_path),
        };
        unsafe {
            (host.api.set_environment)(env_cb);
            (host.api.set_video_refresh)(video_cb);
            (host.api.set_audio_sample)(audio_cb);
            (host.api.set_audio_sample_batch)(audio_batch_cb);
            (host.api.set_input_poll)(input_poll_cb);
            (host.api.set_input_state)(input_state_cb);
        }
        CURRENT.with(|c| c.replace(Some(host)));
        CURRENT.with(|c| {
            let mut slot = c.borrow_mut();
            let host = slot.as_mut().unwrap();
            unsafe { (host.api.init)() };
            let mut info = RetroSystemInfo {
                library_name: ptr::null(),
                library_version: ptr::null(),
                valid_extensions: ptr::null(),
                need_fullpath: false,
                block_extract: false,
            };
            unsafe { (host.api.get_system_info)(&mut info) };
            let name = unsafe { optional_cstr(info.library_name) };
            info!("loaded core {name}");
            let need_path = info.need_fullpath;
            let rom_c = host.rom_path.clone();
            let mut game = RetroGameInfo {
                path: rom_c.as_ptr(),
                data: ptr::null(),
                size: 0,
                meta: ptr::null(),
            };
            if !need_path {
                match std::fs::read(rom_path) {
                    Ok(buf) => {
                        game.data = buf.as_ptr() as *const c_void;
                        game.size = buf.len();
                        host.rom_buf = Some(buf);
                        // pointer stays valid: rom_buf is not moved until unload
                        if let Some(buf) = host.rom_buf.as_ref() {
                            game.data = buf.as_ptr() as *const c_void;
                            game.size = buf.len();
                        }
                    }
                    Err(e) => return Err(format!("read ROM: {e}")),
                }
            }
            let ok = unsafe { (host.api.load_game)(&game) };
            if !ok {
                return Err("core rejected the ROM".into());
            }
            host.loaded = true;
            let mut av = RetroSystemAvInfo {
                geometry: RetroGameGeometry {
                    base_width: 0,
                    base_height: 0,
                    max_width: 0,
                    max_height: 0,
                    aspect_ratio: 0.0,
                },
                timing: RetroSystemTiming {
                    fps: 60.0,
                    sample_rate: 44100.0,
                },
            };
            unsafe { (host.api.get_system_av_info)(&mut av) };
            host.apply_av(&av);
            unsafe { (host.api.set_controller_port_device)(0, 1) }; // JOYPAD
            host.load_sram();
            Ok(())
        })
    }

    fn apply_av(&mut self, av: &RetroSystemAvInfo) {
        if av.geometry.base_width > 0 {
            self.width = av.geometry.base_width;
        }
        if av.geometry.base_height > 0 {
            self.height = av.geometry.base_height;
        }
        if av.timing.fps > 1.0 {
            self.fps = av.timing.fps;
        }
        if av.timing.sample_rate > 1000.0 {
            self.sample_rate = av.timing.sample_rate;
        }
    }

    fn load_sram(&mut self) {
        let data = unsafe { (self.api.get_memory_data)(MEMORY_SAVE_RAM) };
        let size = unsafe { (self.api.get_memory_size)(MEMORY_SAVE_RAM) };
        if data.is_null() || size == 0 {
            return;
        }
        if let Ok(buf) = std::fs::read(&self.sram_path) {
            let n = size.min(buf.len());
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), data as *mut u8, n) };
            info!("loaded SRAM {} bytes from {}", n, self.sram_path.display());
        }
    }

    fn save_sram(&self) {
        let data = unsafe { (self.api.get_memory_data)(MEMORY_SAVE_RAM) };
        let size = unsafe { (self.api.get_memory_size)(MEMORY_SAVE_RAM) };
        if data.is_null() || size == 0 {
            return;
        }
        let slice = unsafe { std::slice::from_raw_parts(data as *const u8, size) };
        if let Err(e) = std::fs::write(&self.sram_path, slice) {
            warn!("SRAM write failed: {e}");
        }
    }

    fn save_state(&self, path: &Path) -> Result<(), String> {
        let size = unsafe { (self.api.serialize_size)() };
        if size == 0 {
            return Err("core has no savestate".into());
        }
        let mut buf = vec![0u8; size];
        let ok = unsafe { (self.api.serialize)(buf.as_mut_ptr() as *mut c_void, size) };
        if !ok {
            return Err("serialize failed".into());
        }
        std::fs::write(path, &buf).map_err(|e| e.to_string())
    }

    fn load_state(&self, path: &Path) -> Result<(), String> {
        let buf = std::fs::read(path).map_err(|e| e.to_string())?;
        let ok = unsafe { (self.api.unserialize)(buf.as_ptr() as *const c_void, buf.len()) };
        if ok {
            Ok(())
        } else {
            Err("unserialize failed".into())
        }
    }
}

pub fn stop() {
    CURRENT.with(|c| {
        if let Some(host) = c.borrow_mut().take() {
            if host.loaded {
                host.save_sram();
                unsafe { (host.api.unload_game)() };
            }
            unsafe { (host.api.deinit)() };
        }
    });
}

pub fn set_buttons(mask: u16) {
    CURRENT.with(|c| {
        if let Some(h) = c.borrow_mut().as_mut() {
            h.buttons = mask;
        }
    });
}

pub fn run_frame() -> bool {
    CURRENT.with(|c| {
        let mut slot = c.borrow_mut();
        let Some(host) = slot.as_mut() else { return false };
        if !host.loaded {
            return false;
        }
        unsafe { (host.api.run)() };
        true
    })
}

pub fn frame_info() -> Option<(u32, u32, usize, u32, f64, f64)> {
    CURRENT.with(|c| {
        c.borrow().as_ref().map(|h| {
            (h.width, h.height, h.pitch, h.pixel_format, h.fps, h.sample_rate)
        })
    })
}

/// Copy the current framebuffer as packed RGB565 little-endian. Converts
/// XRGB8888 and 0RGB1555 when needed. Returns bytes written.
pub fn copy_frame_rgb565(out: &mut [u8]) -> usize {
    CURRENT.with(|c| {
        let host = c.borrow();
        let Some(h) = host.as_ref() else { return 0 };
        if h.width == 0 || h.height == 0 || h.frame.is_empty() {
            return 0;
        }
        let needed = (h.width as usize) * (h.height as usize) * 2;
        if out.len() < needed {
            return 0;
        }
        match h.pixel_format {
            PIXEL_RGB565 => {
                for y in 0..h.height as usize {
                    let src = h.pitch * y;
                    let dst = y * h.width as usize * 2;
                    let row = (h.width as usize) * 2;
                    if src + row <= h.frame.len() {
                        out[dst..dst + row].copy_from_slice(&h.frame[src..src + row]);
                    }
                }
            }
            PIXEL_XRGB8888 => {
                for y in 0..h.height as usize {
                    for x in 0..h.width as usize {
                        let i = h.pitch * y + x * 4;
                        if i + 3 >= h.frame.len() {
                            continue;
                        }
                        let b = h.frame[i] as u16;
                        let g = h.frame[i + 1] as u16;
                        let r = h.frame[i + 2] as u16;
                        let pix = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
                        let o = (y * h.width as usize + x) * 2;
                        out[o..o + 2].copy_from_slice(&pix.to_le_bytes());
                    }
                }
            }
            PIXEL_0RGB1555 | _ => {
                // libretro default 0RGB1555
                for y in 0..h.height as usize {
                    for x in 0..h.width as usize {
                        let i = h.pitch * y + x * 2;
                        if i + 1 >= h.frame.len() {
                            continue;
                        }
                        let pix15 = u16::from_le_bytes([h.frame[i], h.frame[i + 1]]);
                        let r = (pix15 >> 10) & 0x1f;
                        let g = (pix15 >> 5) & 0x1f;
                        let b = pix15 & 0x1f;
                        let pix = (r << 11) | ((g << 1) << 5) | b;
                        let o = (y * h.width as usize + x) * 2;
                        out[o..o + 2].copy_from_slice(&pix.to_le_bytes());
                    }
                }
            }
        }
        needed
    })
}

pub fn copy_audio(out: &mut [i16]) -> usize {
    CURRENT.with(|c| {
        let mut host = c.borrow_mut();
        let Some(h) = host.as_mut() else { return 0 };
        let n = out.len().min(h.audio.len());
        for (i, s) in h.audio.drain(..n).enumerate() {
            out[i] = s;
        }
        n
    })
}

pub fn save_sram() {
    CURRENT.with(|c| {
        if let Some(h) = c.borrow().as_ref() {
            h.save_sram();
        }
    });
}

pub fn save_state(path: &Path) -> Result<(), String> {
    CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .ok_or_else(|| "no game".into())
            .and_then(|h| h.save_state(path))
    })
}

pub fn load_state(path: &Path) -> Result<(), String> {
    CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .ok_or_else(|| "no game".into())
            .and_then(|h| h.load_state(path))
    })
}

pub fn reset() {
    CURRENT.with(|c| {
        if let Some(h) = c.borrow().as_ref() {
            if h.loaded {
                unsafe { (h.api.reset)() };
            }
        }
    });
}

pub fn default_state_path() -> Option<PathBuf> {
    CURRENT.with(|c| {
        c.borrow().as_ref().map(|h| h.save_dir_path.join("slot0.state"))
    })
}

fn cstring(path: &Path) -> CString {
    CString::new(path.to_string_lossy().as_bytes()).unwrap_or_else(|_| CString::new("/").unwrap())
}

unsafe fn optional_cstr(p: *const c_char) -> String {
    if p.is_null() {
        return String::new();
    }
    CStr::from_ptr(p).to_string_lossy().into_owned()
}

unsafe extern "C" fn env_cb(cmd: c_uint, data: *mut c_void) -> bool {
    CURRENT.with(|c| {
        let mut slot = c.borrow_mut();
        let Some(host) = slot.as_mut() else { return false };
        match cmd {
            ENV_GET_CAN_DUPE => {
                if !data.is_null() {
                    unsafe { *(data as *mut bool) = true };
                }
                true
            }
            ENV_SET_MESSAGE => {
                if !data.is_null() {
                    let m = unsafe { &*(data as *const RetroMessage) };
                    info!("core: {}", unsafe { optional_cstr(m.msg) });
                }
                true
            }
            ENV_SET_PERFORMANCE_LEVEL => true,
            ENV_GET_SYSTEM_DIRECTORY | ENV_GET_CORE_ASSETS_DIRECTORY => {
                if !data.is_null() {
                    unsafe { *(data as *mut *const c_char) = host.system_dir.as_ptr() };
                }
                true
            }
            ENV_GET_SAVE_DIRECTORY => {
                if !data.is_null() {
                    unsafe { *(data as *mut *const c_char) = host.save_dir.as_ptr() };
                }
                true
            }
            ENV_SET_PIXEL_FORMAT => {
                if !data.is_null() {
                    host.pixel_format = unsafe { *(data as *const u32) };
                }
                true
            }
            ENV_SET_INPUT_DESCRIPTORS | ENV_SET_CONTROLLER_INFO | ENV_SET_MEMORY_MAPS => true,
            ENV_SET_SUPPORT_NO_GAME | ENV_SET_SUPPORT_ACHIEVEMENTS => true,
            ENV_GET_VARIABLE => {
                if data.is_null() {
                    return false;
                }
                let var = unsafe { &mut *(data as *mut RetroVariable) };
                let key = unsafe { optional_cstr(var.key) };
                if let Some(v) = host.options.get(&key).cloned() {
                    host.option_value = CString::new(v).ok();
                    var.value = host
                        .option_value
                        .as_ref()
                        .map(|c| c.as_ptr())
                        .unwrap_or(ptr::null());
                    true
                } else {
                    var.value = ptr::null();
                    false
                }
            }
            ENV_SET_VARIABLES => {
                // Array of key/value terminated by a null key. Values are
                // "default|other|options"; we keep the first as the default.
                let mut p = data as *const RetroVariable;
                if p.is_null() {
                    return true;
                }
                loop {
                    let v = unsafe { &*p };
                    if v.key.is_null() {
                        break;
                    }
                    let key = unsafe { optional_cstr(v.key) };
                    let raw = unsafe { optional_cstr(v.value) };
                    let def = raw
                        .split(';')
                        .next()
                        .and_then(|s| s.split('|').next())
                        .unwrap_or("")
                        .trim();
                    // Some cores use "Description;default|b|c"
                    let def = if raw.contains(';') {
                        raw.split_once(';')
                            .and_then(|(_, rest)| rest.split('|').next())
                            .unwrap_or(def)
                            .trim()
                    } else {
                        def
                    };
                    if !key.is_empty() && !def.is_empty() {
                        host.options.entry(key).or_insert_with(|| def.to_string());
                    }
                    p = unsafe { p.add(1) };
                }
                true
            }
            ENV_GET_VARIABLE_UPDATE => {
                if !data.is_null() {
                    unsafe { *(data as *mut bool) = host.option_dirty };
                }
                host.option_dirty = false;
                true
            }
            ENV_GET_LOG_INTERFACE => false,
            ENV_SET_SYSTEM_AV_INFO => {
                if !data.is_null() {
                    host.apply_av(unsafe { &*(data as *const RetroSystemAvInfo) });
                }
                true
            }
            ENV_SET_GEOMETRY => {
                if !data.is_null() {
                    let g = unsafe { &*(data as *const RetroGameGeometry) };
                    if g.base_width > 0 {
                        host.width = g.base_width;
                    }
                    if g.base_height > 0 {
                        host.height = g.base_height;
                    }
                }
                true
            }
            ENV_GET_CORE_OPTIONS_VERSION => {
                if !data.is_null() {
                    unsafe { *(data as *mut c_uint) = 0 };
                }
                true
            }
            ENV_SET_CORE_OPTIONS
            | ENV_SET_CORE_OPTIONS_INTL
            | ENV_SET_CORE_OPTIONS_DISPLAY
            | ENV_SET_CORE_OPTIONS_V2
            | ENV_SET_CORE_OPTIONS_V2_INTL => true,
            _ => false,
        }
    })
}

unsafe extern "C" fn video_cb(data: *const c_void, width: c_uint, height: c_uint, pitch: usize) {
    if data.is_null() || width == 0 || height == 0 {
        return;
    }
    CURRENT.with(|c| {
        let mut slot = c.borrow_mut();
        let Some(host) = slot.as_mut() else { return };
        host.width = width;
        host.height = height;
        host.pitch = pitch;
        let bytes = pitch * height as usize;
        if host.frame.len() < bytes {
            host.frame.resize(bytes, 0);
        }
        unsafe {
            ptr::copy_nonoverlapping(data as *const u8, host.frame.as_mut_ptr(), bytes);
        }
    });
}

unsafe extern "C" fn audio_cb(left: i16, right: i16) {
    CURRENT.with(|c| {
        if let Some(h) = c.borrow_mut().as_mut() {
            h.audio.push_back(left);
            h.audio.push_back(right);
            while h.audio.len() > 48_000 {
                h.audio.pop_front();
            }
        }
    });
}

unsafe extern "C" fn audio_batch_cb(data: *const i16, frames: usize) -> usize {
    if data.is_null() || frames == 0 {
        return 0;
    }
    CURRENT.with(|c| {
        if let Some(h) = c.borrow_mut().as_mut() {
            let slice = unsafe { std::slice::from_raw_parts(data, frames * 2) };
            h.audio.extend(slice.iter().copied());
            while h.audio.len() > 48_000 {
                h.audio.pop_front();
            }
        }
    });
    frames
}

unsafe extern "C" fn input_poll_cb() {}

unsafe extern "C" fn input_state_cb(port: c_uint, _device: c_uint, _index: c_uint, id: c_uint) -> i16 {
    if port != 0 || id > 15 {
        return 0;
    }
    CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .map(|h| if h.buttons & (1 << id) != 0 { 1 } else { 0 })
            .unwrap_or(0)
    })
}

