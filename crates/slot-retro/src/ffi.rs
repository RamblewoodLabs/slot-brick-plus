use std::ffi::{c_char, c_int, c_uint, c_void};

use libloading::Library;

use crate::core::CoreError;

pub const API_VERSION: c_uint = 1;

pub const GET_CAN_DUPE: c_uint = 3;
pub const GET_SYSTEM_DIRECTORY: c_uint = 9;
pub const SET_DISK_CONTROL_INTERFACE: c_uint = 13;
pub const GET_DISK_CONTROL_INTERFACE_VERSION: c_uint = 57;
pub const SET_DISK_CONTROL_EXT_INTERFACE: c_uint = 58;
pub const SET_PIXEL_FORMAT: c_uint = 10;
pub const GET_VARIABLE: c_uint = 15;
pub const SET_VARIABLES: c_uint = 16;
pub const GET_VARIABLE_UPDATE: c_uint = 17;
pub const GET_RUMBLE_INTERFACE: c_uint = 23;
pub const GET_LOG_INTERFACE: c_uint = 27;
pub const GET_SAVE_DIRECTORY: c_uint = 31;
pub const SET_AUDIO_BUFFER_STATUS_CALLBACK: c_uint = 62;
pub const SET_NETPACKET_INTERFACE: c_uint = 78;
pub const SET_HW_RENDER: c_uint = 14;
pub const SET_GEOMETRY: c_uint = 37;
pub const GET_PREFERRED_HW_RENDER: c_uint = 56;
/// 36 | RETRO_ENVIRONMENT_EXPERIMENTAL
pub const SET_MEMORY_MAPS: c_uint = 36 | 0x10000;

pub const RUMBLE_STRONG: c_uint = 0;
pub const RUMBLE_WEAK: c_uint = 1;

pub const PIXEL_FORMAT_XRGB8888: c_uint = 1;
pub const PIXEL_FORMAT_RGB565: c_uint = 2;
pub const DEVICE_JOYPAD: c_uint = 1;
pub const DEVICE_ANALOG: c_uint = 5;
pub const ANALOG_LEFT: c_uint = 0;
pub const ANALOG_X: c_uint = 0;
pub const ANALOG_Y: c_uint = 1;
/// PCSX ReARMed's DualShock: RETRO_DEVICE_SUBCLASS(RETRO_DEVICE_ANALOG, 1).
pub const DEVICE_DUALSHOCK: c_uint = (2 << 8) | DEVICE_ANALOG;
pub const MEMORY_SAVE_RAM: c_uint = 0;
pub const MEMORY_RTC: c_uint = 1;
pub const JOYPAD_MASK: c_uint = 256;

pub const NETPACKET_UNRELIABLE: i32 = 0;
pub const NETPACKET_RELIABLE: i32 = 1 << 0;
pub const NETPACKET_UNSEQUENCED: i32 = 1 << 1;
pub const NETPACKET_FLUSH_HINT: i32 = 1 << 2;
pub const NETPACKET_BROADCAST: u16 = 0xFFFF;

#[repr(C)]
pub struct GameInfo {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct SystemInfo {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

impl Default for SystemInfo {
    fn default() -> Self {
        SystemInfo {
            library_name: std::ptr::null(),
            library_version: std::ptr::null(),
            valid_extensions: std::ptr::null(),
            need_fullpath: false,
            block_extract: false,
        }
    }
}

/// retro_disk_control_callback: how a disc-based core lets the frontend swap discs.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskControl {
    pub set_eject_state: Option<unsafe extern "C" fn(bool) -> bool>,
    pub get_eject_state: Option<unsafe extern "C" fn() -> bool>,
    pub get_image_index: Option<unsafe extern "C" fn() -> c_uint>,
    pub set_image_index: Option<unsafe extern "C" fn(c_uint) -> bool>,
    pub get_num_images: Option<unsafe extern "C" fn() -> c_uint>,
    pub replace_image_index: Option<unsafe extern "C" fn(c_uint, *const GameInfo) -> bool>,
    pub add_image_index: Option<unsafe extern "C" fn() -> bool>,
}

/// retro_disk_control_ext_callback: the same, and the disc to start on, set before the game loads.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DiskControlExt {
    pub base: DiskControl,
    pub set_initial_image: Option<unsafe extern "C" fn(c_uint, *const c_char) -> bool>,
    pub get_image_path: Option<unsafe extern "C" fn(c_uint, *mut c_char, usize) -> bool>,
    pub get_image_label: Option<unsafe extern "C" fn(c_uint, *mut c_char, usize) -> bool>,
}

#[repr(C)]
#[derive(Default)]
pub struct GameGeometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Default)]
pub struct SystemTiming {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Default)]
pub struct SystemAvInfo {
    pub geometry: GameGeometry,
    pub timing: SystemTiming,
}

/// retro_memory_descriptor, as cores pass it in SET_MEMORY_MAPS.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MemoryDescriptor {
    pub flags: u64,
    pub ptr: *mut c_void,
    pub offset: usize,
    pub start: usize,
    pub select: usize,
    pub disconnect: usize,
    pub len: usize,
    pub addrspace: *const c_char,
}

// The pointers are into the core's own memory, which moves with the core and is only
// read on the thread that runs it.
unsafe impl Send for MemoryDescriptor {}

/// retro_memory_map
#[repr(C)]
pub struct MemoryMap {
    pub descriptors: *const MemoryDescriptor,
    pub num_descriptors: c_uint,
}

#[repr(C)]
pub struct Variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

#[repr(C)]
pub struct LogCallback {
    pub log: *const c_void,
}

pub type SetRumbleStateFn = unsafe extern "C" fn(c_uint, c_uint, u16) -> bool;

#[repr(C)]
pub struct RumbleInterface {
    pub set_rumble_state: SetRumbleStateFn,
}

pub type AudioBufferStatusFn =
    unsafe extern "C" fn(active: bool, occupancy: c_uint, underrun_likely: bool);

#[repr(C)]
pub struct AudioBufferStatusCallback {
    pub callback: Option<AudioBufferStatusFn>,
}

pub type NetpacketSend =
    unsafe extern "C" fn(flags: c_int, buf: *const c_void, len: usize, client_id: u16);
pub type NetpacketPollReceive = unsafe extern "C" fn();

pub type NetpacketStart =
    unsafe extern "C" fn(client_id: u16, send: NetpacketSend, poll_receive: NetpacketPollReceive);
pub type NetpacketReceive = unsafe extern "C" fn(buf: *const c_void, len: usize, client_id: u16);
pub type NetpacketStop = unsafe extern "C" fn();
pub type NetpacketPoll = unsafe extern "C" fn();
pub type NetpacketConnected = unsafe extern "C" fn(client_id: u16) -> bool;
pub type NetpacketDisconnected = unsafe extern "C" fn(client_id: u16);

#[repr(C)]
pub struct NetpacketCallback {
    pub start: Option<NetpacketStart>,
    pub receive: Option<NetpacketReceive>,
    pub stop: Option<NetpacketStop>,
    pub poll: Option<NetpacketPoll>,
    pub connected: Option<NetpacketConnected>,
    pub disconnected: Option<NetpacketDisconnected>,
    pub protocol_version: *const c_char,
}

// SAFETY: besides function pointers, `protocol_version` points at a string literal in the
unsafe impl Send for NetpacketCallback {}

pub type EnvironmentFn = unsafe extern "C" fn(c_uint, *mut c_void) -> bool;
pub type VideoRefreshFn = unsafe extern "C" fn(*const c_void, c_uint, c_uint, usize);
pub type AudioSampleFn = unsafe extern "C" fn(i16, i16);
pub type AudioBatchFn = unsafe extern "C" fn(*const i16, usize) -> usize;
pub type InputPollFn = unsafe extern "C" fn();
pub type InputStateFn = unsafe extern "C" fn(c_uint, c_uint, c_uint, c_uint) -> i16;

pub struct Api {
    pub init: unsafe extern "C" fn(),
    pub deinit: unsafe extern "C" fn(),
    pub api_version: unsafe extern "C" fn() -> c_uint,
    pub get_system_info: unsafe extern "C" fn(*mut SystemInfo),
    pub get_system_av_info: unsafe extern "C" fn(*mut SystemAvInfo),
    pub set_environment: unsafe extern "C" fn(EnvironmentFn),
    pub set_video_refresh: unsafe extern "C" fn(VideoRefreshFn),
    pub set_audio_sample: unsafe extern "C" fn(AudioSampleFn),
    pub set_audio_sample_batch: unsafe extern "C" fn(AudioBatchFn),
    pub set_input_poll: unsafe extern "C" fn(InputPollFn),
    pub set_input_state: unsafe extern "C" fn(InputStateFn),
    pub set_controller_port_device: unsafe extern "C" fn(c_uint, c_uint),
    pub load_game: unsafe extern "C" fn(*const GameInfo) -> bool,
    pub unload_game: unsafe extern "C" fn(),
    pub reset: unsafe extern "C" fn(),
    pub run: unsafe extern "C" fn(),
    pub serialize_size: unsafe extern "C" fn() -> usize,
    pub serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    pub unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    pub get_memory_data: unsafe extern "C" fn(c_uint) -> *mut c_void,
    pub get_memory_size: unsafe extern "C" fn(c_uint) -> usize,
}

impl Api {
    pub unsafe fn load(lib: &Library) -> Result<Self, CoreError> {
        macro_rules! get {
            ($name:literal) => {
                *lib.get(concat!($name, "\0").as_bytes())
                    .map_err(|e| CoreError::Load(format!("{}: {e}", $name)))?
            };
        }
        Ok(Api {
            init: get!("retro_init"),
            deinit: get!("retro_deinit"),
            api_version: get!("retro_api_version"),
            get_system_info: get!("retro_get_system_info"),
            get_system_av_info: get!("retro_get_system_av_info"),
            set_environment: get!("retro_set_environment"),
            set_video_refresh: get!("retro_set_video_refresh"),
            set_audio_sample: get!("retro_set_audio_sample"),
            set_audio_sample_batch: get!("retro_set_audio_sample_batch"),
            set_input_poll: get!("retro_set_input_poll"),
            set_input_state: get!("retro_set_input_state"),
            set_controller_port_device: get!("retro_set_controller_port_device"),
            load_game: get!("retro_load_game"),
            unload_game: get!("retro_unload_game"),
            reset: get!("retro_reset"),
            run: get!("retro_run"),
            serialize_size: get!("retro_serialize_size"),
            serialize: get!("retro_serialize"),
            unserialize: get!("retro_unserialize"),
            get_memory_data: get!("retro_get_memory_data"),
            get_memory_size: get!("retro_get_memory_size"),
        })
    }
}

/// Calls into a core that keep the caller's floating-point registers. AAPCS64 says a callee
/// keeps d8-d15, but some cores' coroutine switches (libco, under mupen64plus-next) do not
/// save them, and Slot's own floats held there across the call came back as garbage (frame
/// times read infinite). On aarch64 these name d8-d15 as clobbered, so the compiler saves
/// them around the call; elsewhere they are plain calls.
pub mod guard {
    use std::ffi::c_void;

    #[cfg(target_arch = "aarch64")]
    unsafe fn raw(f: usize, a0: usize, a1: usize) -> usize {
        let r: usize;
        std::arch::asm!(
            "blr {f}",
            f = in(reg) f,
            inlateout("x0") a0 => r,
            in("x1") a1,
            clobber_abi("C"),
            out("d8") _, out("d9") _, out("d10") _, out("d11") _,
            out("d12") _, out("d13") _, out("d14") _, out("d15") _,
        );
        r
    }

    /// `f()`.
    pub unsafe fn call0(f: unsafe extern "C" fn()) {
        #[cfg(target_arch = "aarch64")]
        raw(f as usize, 0, 0);
        #[cfg(not(target_arch = "aarch64"))]
        f();
    }

    /// `f(p)` for a core entry point that answers yes or no.
    pub unsafe fn call_ptr<T>(f: unsafe extern "C" fn(*const T) -> bool, p: *const T) -> bool {
        #[cfg(target_arch = "aarch64")]
        return raw(f as usize, p as usize, 0) & 0xff != 0;
        #[cfg(not(target_arch = "aarch64"))]
        f(p)
    }

    /// `f(buf, len)` for serialize.
    pub unsafe fn call_buf_mut(
        f: unsafe extern "C" fn(*mut c_void, usize) -> bool,
        p: *mut c_void,
        n: usize,
    ) -> bool {
        #[cfg(target_arch = "aarch64")]
        return raw(f as usize, p as usize, n) & 0xff != 0;
        #[cfg(not(target_arch = "aarch64"))]
        f(p, n)
    }

    /// `f(buf, len)` for unserialize.
    pub unsafe fn call_buf(
        f: unsafe extern "C" fn(*const c_void, usize) -> bool,
        p: *const c_void,
        n: usize,
    ) -> bool {
        #[cfg(target_arch = "aarch64")]
        return raw(f as usize, p as usize, n) & 0xff != 0;
        #[cfg(not(target_arch = "aarch64"))]
        f(p, n)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        static mut SEEN: usize = 0;
        unsafe extern "C" fn nothing() {}
        unsafe extern "C" fn yes(p: *const u8) -> bool {
            SEEN = p as usize;
            true
        }
        unsafe extern "C" fn sum(p: *const c_void, n: usize) -> bool {
            SEEN = p as usize + n;
            n % 2 == 0
        }
        unsafe extern "C" fn sum_mut(p: *mut c_void, n: usize) -> bool {
            sum(p, n)
        }

        #[test]
        fn guarded_calls_pass_their_arguments_and_answers_through() {
            unsafe {
                call0(nothing);
                assert!(call_ptr(yes, 40 as *const u8));
                assert_eq!(SEEN, 40);
                assert!(call_buf(sum, 8 as *const c_void, 2));
                assert_eq!(SEEN, 10);
                assert!(!call_buf_mut(sum_mut, 8 as *mut c_void, 3));
                let x = std::hint::black_box(1.5f64);
                call0(nothing);
                assert_eq!(x * 2.0, 3.0);
            }
        }
    }
}
