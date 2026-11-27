//! `slot --pad-bridge`: the Brick's buttons as a joystick for an app Slot opens (HOME → Apps).
//!
//! On Slot OS the buttons are GPIO pins that only Slot reads (brick_gpio), so a program using
//! SDL sees no gamepad. While an app runs, run-app.sh starts Slot in this mode: it reads the
//! pins as Slot does and presents them through /dev/uinput (built into TrimUI's kernel) as a
//! joystick laid out as the original Brick's raw SDL joystick is on KNULLI, which GrainBrick
//! falls back to: B 4, A 5, Y 6, X 7, L 8, R 9, L2 10, R2 11, Select 12, Start 13, Menu 14,
//! and the D-pad as hat 0. On stock firmware trimui_inputd already makes one, so this does
//! nothing. It runs until it is killed; the device goes away with the process.

use std::io::Write;
use std::os::fd::AsRawFd;

use slot_input::{Btn, RawEvent};

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
/// BTN_JOYSTICK (BTN_TRIGGER): SDL numbers a joystick's buttons from here in code order, and
/// keeping clear of the gamepad range (BTN_SOUTH, 0x130) stops SDL guessing a mapping of its own.
const BTN_BASE: u16 = 0x120;
const BUTTONS: u16 = 15;

const UI_SET_EVBIT: libc::c_ulong = 0x4004_5564;
const UI_SET_KEYBIT: libc::c_ulong = 0x4004_5565;
const UI_SET_ABSBIT: libc::c_ulong = 0x4004_5567;
const UI_DEV_CREATE: libc::c_ulong = 0x5501;

/// The joystick button index of a Brick button, as KNULLI's raw Brick joystick numbers it.
pub fn index_of(btn: Btn) -> Option<u16> {
    Some(match btn {
        Btn::B => 4,
        Btn::A => 5,
        Btn::Y => 6,
        Btn::X => 7,
        Btn::L1 => 8,
        Btn::R1 => 9,
        Btn::L2 => 10,
        Btn::R2 => 11,
        Btn::Select => 12,
        Btn::Start => 13,
        Btn::Menu => 14,
        _ => return None,
    })
}

/// The evdev events for one button change: (type, code, value), D-pad directions as the hat.
pub fn events_of(ev: RawEvent, hat: &mut (i32, i32)) -> Vec<(u16, u16, i32)> {
    let (btn, down) = match ev {
        RawEvent::Down(b) => (b, true),
        RawEvent::Up(b) => (b, false),
    };
    let axis = |v: i32, held: &mut i32, code: u16| {
        // Letting go of a direction only centres the hat if it still points that way.
        if down {
            *held = v;
        } else if *held == v {
            *held = 0;
        }
        vec![(EV_ABS, code, *held)]
    };
    match btn {
        Btn::Up => axis(-1, &mut hat.1, ABS_HAT0Y),
        Btn::Down => axis(1, &mut hat.1, ABS_HAT0Y),
        Btn::Left => axis(-1, &mut hat.0, ABS_HAT0X),
        Btn::Right => axis(1, &mut hat.0, ABS_HAT0X),
        b => index_of(b)
            .map(|i| vec![(EV_KEY, BTN_BASE + i, down as i32)])
            .unwrap_or_default(),
    }
}

#[allow(dead_code)]
fn write_event(dev: &mut std::fs::File, kind: u16, code: u16, value: i32) -> std::io::Result<()> {
    // struct input_event on aarch64: a 16-byte timeval (left zero: the kernel stamps it), then
    // type, code and value.
    let mut buf = [0u8; 24];
    buf[16..18].copy_from_slice(&kind.to_ne_bytes());
    buf[18..20].copy_from_slice(&code.to_ne_bytes());
    buf[20..24].copy_from_slice(&value.to_ne_bytes());
    dev.write_all(&buf)
}

#[allow(dead_code)]
fn create() -> std::io::Result<std::fs::File> {
    let dev = std::fs::OpenOptions::new().write(true).open("/dev/uinput")?;
    let fd = dev.as_raw_fd();
    let ioctl = |req: libc::c_ulong, arg: libc::c_int| -> std::io::Result<()> {
        if unsafe { libc::ioctl(fd, req as _, arg) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(())
    };
    ioctl(UI_SET_EVBIT, EV_KEY as _)?;
    ioctl(UI_SET_EVBIT, EV_ABS as _)?;
    ioctl(UI_SET_EVBIT, EV_SYN as _)?;
    for i in 0..BUTTONS {
        ioctl(UI_SET_KEYBIT, (BTN_BASE + i) as _)?;
    }
    ioctl(UI_SET_ABSBIT, ABS_HAT0X as _)?;
    ioctl(UI_SET_ABSBIT, ABS_HAT0Y as _)?;
    // struct uinput_user_dev: name[80], input_id (bustype, vendor, product, version: u16 each),
    // ff_effects_max (u32), then absmax, absmin, absfuzz and absflat, 64 i32s each.
    let mut user = vec![0u8; 80 + 8 + 4 + 4 * 64 * 4];
    let name = b"Slot Brick Pad";
    user[..name.len()].copy_from_slice(name);
    user[80..82].copy_from_slice(&0x06u16.to_ne_bytes()); // BUS_VIRTUAL
    user[82..84].copy_from_slice(&0x5107u16.to_ne_bytes());
    user[84..86].copy_from_slice(&0x0001u16.to_ne_bytes());
    user[86..88].copy_from_slice(&0x0001u16.to_ne_bytes());
    let absmax = 92;
    let absmin = absmax + 64 * 4;
    for code in [ABS_HAT0X, ABS_HAT0Y] {
        let c = code as usize * 4;
        user[absmax + c..absmax + c + 4].copy_from_slice(&1i32.to_ne_bytes());
        user[absmin + c..absmin + c + 4].copy_from_slice(&(-1i32).to_ne_bytes());
    }
    let mut dev = dev;
    dev.write_all(&user)?;
    ioctl(UI_DEV_CREATE, 0)?;
    Ok(dev)
}

/// Runs the bridge until killed. Returns at once on stock firmware (SDL's own pad is there).
#[cfg(all(feature = "brick", not(feature = "sim"), not(feature = "rg35xxsp")))]
pub fn run(root: &std::path::Path) {
    use slot_input::InputSource;
    if super::brick::sdl_available() {
        eprintln!("slot: pad bridge: the firmware's joystick is there; nothing to do");
        return;
    }
    let mut input = match super::brick_gpio::GpioInput::open(root) {
        Ok(i) => i,
        Err(e) => return eprintln!("slot: pad bridge: {e}"),
    };
    let mut dev = match create() {
        Ok(d) => d,
        Err(e) => return eprintln!("slot: pad bridge: /dev/uinput: {e}"),
    };
    eprintln!("slot: pad bridge: Brick buttons as joystick \"Slot Brick Pad\"");
    let start = std::time::Instant::now();
    let mut hat = (0, 0);
    loop {
        let now = start.elapsed().as_millis() as u64;
        let mut wrote = false;
        for ev in input.poll(now) {
            for (kind, code, value) in events_of(ev, &mut hat) {
                wrote |= write_event(&mut dev, kind, code, value).is_ok();
            }
        }
        if wrote {
            let _ = write_event(&mut dev, EV_SYN, 0, 0);
        }
        std::thread::sleep(std::time::Duration::from_millis(4));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_and_hat_follow_the_brick_layout() {
        let mut hat = (0, 0);
        assert_eq!(events_of(RawEvent::Down(Btn::A), &mut hat), vec![(EV_KEY, 0x125, 1)]);
        assert_eq!(events_of(RawEvent::Up(Btn::Start), &mut hat), vec![(EV_KEY, 0x12d, 0)]);
        assert_eq!(events_of(RawEvent::Down(Btn::Left), &mut hat), vec![(EV_ABS, ABS_HAT0X, -1)]);
        assert_eq!(events_of(RawEvent::Down(Btn::Right), &mut hat), vec![(EV_ABS, ABS_HAT0X, 1)]);
        // Left let go while Right is held: the hat stays right.
        assert_eq!(events_of(RawEvent::Up(Btn::Left), &mut hat), vec![(EV_ABS, ABS_HAT0X, 1)]);
        assert_eq!(events_of(RawEvent::Up(Btn::Right), &mut hat), vec![(EV_ABS, ABS_HAT0X, 0)]);
        assert!(events_of(RawEvent::Down(Btn::Power), &mut hat).is_empty());
        assert!(index_of(Btn::Menu).unwrap() < BUTTONS);
    }
}
