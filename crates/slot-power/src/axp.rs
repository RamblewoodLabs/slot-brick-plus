//! The Brick's AXP2202 gauge. The battery `status` node often says "Not charging"
//! while nothing is plugged in, and that word contains "charging", so a cable is
//! read from the USB or AC `online` node instead.
use std::fs;

use crate::{hw, Battery, Charge};

pub fn battery() -> Option<Battery> {
    let percent = read_u32("axp2202-battery/capacity")?.min(100) as u8;
    Some(Battery {
        percent,
        charge: charge(percent),
    })
}

pub fn charge(percent: u8) -> Charge {
    let plugged = match (online("axp2202-usb/online"), online("axp2202-ac/online")) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), _) | (_, Some(false)) => Some(false),
        _ => None,
    };
    let status = fs::read_to_string(hw("/sys/class/power_supply/axp2202-battery/status"))
        .unwrap_or_default();
    charge_of(percent, plugged, status.trim())
}

/// `plugged` is the USB or AC `online` node: 1, 0, or missing.
/// A status of "Not charging" is not charging.
pub fn charge_of(percent: u8, plugged: Option<bool>, status: &str) -> Charge {
    match plugged {
        Some(true) if percent >= 100 || status == "Full" => Charge::Full,
        Some(true) => Charge::Charging,
        Some(false) => Charge::Discharging,
        None => match status {
            "Charging" => Charge::Charging,
            "Full" => Charge::Full,
            "Discharging" | "Not charging" => Charge::Discharging,
            _ => Charge::Unknown,
        },
    }
}

fn online(name: &str) -> Option<bool> {
    let text = fs::read_to_string(hw(&format!("/sys/class/power_supply/{name}"))).ok()?;
    match text.trim() {
        "1" => Some(true),
        "0" => Some(false),
        _ => None,
    }
}

fn read_u32(name: &str) -> Option<u32> {
    fs::read_to_string(hw(&format!("/sys/class/power_supply/{name}")))
        .ok()?
        .trim()
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_charging_is_unplugged_and_a_cable_is_the_online_node() {
        assert_eq!(
            charge_of(34, Some(false), "Not charging"),
            Charge::Discharging
        );
        assert_eq!(charge_of(34, Some(false), "Charging"), Charge::Discharging);
        assert_eq!(charge_of(34, Some(true), "Not charging"), Charge::Charging);
        assert_eq!(charge_of(100, Some(true), "Full"), Charge::Full);
        assert_eq!(charge_of(40, None, "Not charging"), Charge::Discharging);
        assert_eq!(charge_of(40, None, "Charging"), Charge::Charging);
        assert_eq!(charge_of(40, None, "Unknown"), Charge::Unknown);
    }
}
