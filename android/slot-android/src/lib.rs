//! JNI entry points for the Slotted Android app.

mod host;
mod registry;

use jni::objects::{JByteBuffer, JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use std::path::Path;

fn init_log() {
    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("slot"),
        );
        log::info!("slot-android JNI ready");
    });
}

fn java_string(env: &mut JNIEnv, s: JString) -> String {
    env.get_string(&s).map(|s| s.into()).unwrap_or_default()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_registryJson(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_log();
    env.new_string(registry::REGISTRY_JSON)
        .expect("registry json")
        .into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_coreFileName(
    mut env: JNIEnv,
    _class: JClass,
    core: JString,
) -> jstring {
    init_log();
    let core = java_string(&mut env, core);
    env.new_string(registry::core_so_name(&core))
        .expect("core name")
        .into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_isCoreEmbedded(
    mut env: JNIEnv,
    _class: JClass,
    core: JString,
) -> jboolean {
    init_log();
    if host::is_embedded(&java_string(&mut env, core)) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_bundledCoreName(
    mut env: JNIEnv,
    _class: JClass,
    core: JString,
) -> jstring {
    init_log();
    let core = java_string(&mut env, core);
    env.new_string(host::bundled_so_name(&core))
        .expect("bundled name")
        .into_raw()
}

/// Start a game. Returns an empty string on success, or an error message.
#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_startGame(
    mut env: JNIEnv,
    _class: JClass,
    core_so: JString,
    rom_path: JString,
    system_dir: JString,
    save_dir: JString,
) -> jstring {
    init_log();
    let core = java_string(&mut env, core_so);
    let rom = java_string(&mut env, rom_path);
    let sys = java_string(&mut env, system_dir);
    let save = java_string(&mut env, save_dir);
    let result = host::Host::start(Path::new(&core), Path::new(&rom), Path::new(&sys), Path::new(&save));
    let msg = match result {
        Ok(()) => String::new(),
        Err(e) => {
            log::error!("startGame: {e}");
            e
        }
    };
    env.new_string(msg).expect("msg").into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_stopGame(_env: JNIEnv, _class: JClass) {
    host::stop();
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_setButtons(
    _env: JNIEnv,
    _class: JClass,
    mask: jint,
) {
    host::set_buttons(mask as u16);
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_runFrame(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    if host::run_frame() {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_frameWidth(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    host::frame_info().map(|(w, _, _, _, _, _)| w as jint).unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_frameHeight(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    host::frame_info().map(|(_, h, _, _, _, _)| h as jint).unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_sampleRate(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    host::frame_info()
        .map(|(_, _, _, _, _, rate)| rate.round() as jint)
        .unwrap_or(44100)
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_copyFrameRgb565(
    env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
) -> jint {
    let ptr = match env.get_direct_buffer_address(&buffer) {
        Ok(p) => p,
        Err(_) => return 0,
    };
    let len = env.get_direct_buffer_capacity(&buffer).unwrap_or(0);
    let out = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
    host::copy_frame_rgb565(out) as jint
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_copyAudio(
    env: JNIEnv,
    _class: JClass,
    buffer: JByteBuffer,
) -> jint {
    let ptr = match env.get_direct_buffer_address(&buffer) {
        Ok(p) => p,
        Err(_) => return 0,
    };
    let len = env.get_direct_buffer_capacity(&buffer).unwrap_or(0);
    // Short samples: 2 bytes each.
    let samples = len / 2;
    let out = unsafe { std::slice::from_raw_parts_mut(ptr as *mut i16, samples) };
    host::copy_audio(out) as jint
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_saveSram(_env: JNIEnv, _class: JClass) {
    host::save_sram();
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_saveState(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    let path = java_string(&mut env, path);
    let msg = host::save_state(Path::new(&path)).err().unwrap_or_default();
    env.new_string(msg).expect("msg").into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_loadState(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
) -> jstring {
    let path = java_string(&mut env, path);
    let msg = host::load_state(Path::new(&path)).err().unwrap_or_default();
    env.new_string(msg).expect("msg").into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_resetGame(_env: JNIEnv, _class: JClass) {
    host::reset();
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_defaultStatePath(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let s = host::default_state_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    env.new_string(s).expect("path").into_raw()
}

#[no_mangle]
pub extern "system" fn Java_plus_slotbrick_slot_SlotNative_audioPending(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    // Not separately exposed; copyAudio drains whatever is queued.
    0
}
