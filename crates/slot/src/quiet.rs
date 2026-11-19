//! Starting a thread must not close Slot. `thread::spawn` panics when the process
//! cannot make another thread, which is how a stuck download loop took the UI down.
use std::thread;

pub fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> bool {
    match thread::Builder::new().name(name.into()).spawn(work) {
        Ok(_) => true,
        Err(e) => {
            eprintln!("slot: could not start {name}: {e}");
            false
        }
    }
}
