//! JNI entry points for the Slotted Android app.

mod registry;

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

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
    let core: String = env
        .get_string(&core)
        .map(|s| s.into())
        .unwrap_or_default();
    env.new_string(registry::core_so_name(&core))
        .expect("core name")
        .into_raw()
}
