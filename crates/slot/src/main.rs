#[cfg(not(feature = "host"))]
mod device_app;
#[cfg(feature = "host")]
mod host_app;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("--init-root") => {
            let Some(dir) = args.next() else {
                eprintln!("slot: --init-root needs a directory");
                std::process::exit(2);
            };
            slot::root::ensure(std::path::Path::new(&dir));
            return;
        }
        // HOME → Apps: the Brick's buttons as a joystick while an app runs (run-app.sh).
        #[cfg(all(feature = "brick", not(feature = "sim"), not(feature = "rg35xxsp")))]
        Some("--pad-bridge") => {
            let root = std::env::var_os("SLOT_ROOT")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("/mnt/SDCARD/Slot"));
            slot::input::pad_bridge::run(&root);
            return;
        }
        _ => {}
    }
    // 1.6.0: on a crash, log the signal, thread and code location to stderr (slot.log).
    slot::crash::install();

    #[cfg(feature = "host")]
    host_app::run();

    #[cfg(not(feature = "host"))]
    device_app::run();
}
