//! Battery and Wi-Fi for the system menu, read off the UI thread.
//! A failed reading is simply missing, so that icon is not drawn.
use std::fs;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy, Default)]
pub struct Live {
    pub percent: Option<u8>,
    pub charging: bool,
    /// `None` hides the icon. `Some(0)` is off or not connected. `Some(1..=4)` is signal bars.
    pub wifi: Option<u8>,
}

fn cell() -> &'static Mutex<Live> {
    static CELL: OnceLock<Mutex<Live>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Live::default()))
}

pub fn current() -> Live {
    cell().lock().map(|g| *g).unwrap_or_default()
}

pub fn start() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        thread::Builder::new()
            .name("slot-status".into())
            .spawn(|| loop {
                let live = std::panic::catch_unwind(sample).unwrap_or_default();
                if let Ok(mut slot) = cell().lock() {
                    *slot = live;
                }
                thread::sleep(Duration::from_secs(3));
            })
            .ok();
    });
}

fn sample() -> Live {
    let (percent, charging) = battery();
    Live {
        percent,
        charging,
        wifi: wifi(),
    }
}

fn battery() -> (Option<u8>, bool) {
    let dir = slot_power::hw("/sys/class/power_supply/axp2202-battery");
    let Ok(text) = fs::read_to_string(dir.join("capacity")) else {
        return (None, false);
    };
    let Ok(percent) = text.trim().parse::<u8>() else {
        return (None, false);
    };
    let status = fs::read_to_string(dir.join("status")).unwrap_or_default();
    let status = status.to_ascii_lowercase();
    let charging = status.contains("charging") || status.contains("full");
    (Some(percent.min(100)), charging)
}

fn wifi() -> Option<u8> {
    #[cfg(feature = "brick")]
    let radio = crate::brick_wifi::radio_on();
    #[cfg(not(feature = "brick"))]
    let radio = false;
    let text = fs::read_to_string(slot_power::hw("/proc/net/wireless")).ok();
    if text.is_none() && !radio {
        // No wireless table and no supplicant: this machine has nothing to show.
        return None;
    }
    if !radio {
        return Some(0);
    }
    let Some(text) = text else {
        return Some(0);
    };
    let link = text.lines().nth(2).and_then(link_quality);
    match link {
        Some(q) if q > 0 => Some(bars(q)),
        _ => Some(0),
    }
}

fn link_quality(line: &str) -> Option<u32> {
    let mut parts = line.split_whitespace();
    let _face = parts.next()?;
    let _status = parts.next()?;
    let link = parts.next()?.trim_end_matches('.');
    link.parse().ok()
}

fn bars(quality: u32) -> u8 {
    match quality {
        0 => 0,
        1..=20 => 1,
        21..=40 => 2,
        41..=55 => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wireless_quality_becomes_bars_and_a_bad_file_hides_nothing_it_cannot_read() {
        let line = "wlan0: 0000   50.  -40.  -256        0      0      0      0      0        0";
        assert_eq!(link_quality(line), Some(50));
        assert_eq!(bars(50), 3);
        assert_eq!(bars(10), 1);
        assert!(link_quality("not a row").is_none());
    }
}
