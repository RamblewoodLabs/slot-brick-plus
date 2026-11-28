use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const CARD_LOGO: &str = "System/bootlogo.bmp";
const LOGO: &str = "bootlogo.bmp";
const ORIGINAL: &str = "bootlogo.baseos.bmp";
const PARTITION: &str = "boot-resource";
const MOUNT: &str = "/tmp/slot-boot-resource";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Same,
    Installed,
    Invalid,
}

pub struct Bmp {
    pub width: usize,
    pub height: usize,
    pub rgb: Vec<[u8; 3]>,
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

pub fn size(bmp: &[u8]) -> Option<(usize, usize)> {
    if bmp.get(0..2)? != b"BM" || u32_at(bmp, 14)? < 40 {
        return None;
    }
    let w = u32_at(bmp, 18)? as i32;
    let h = u32_at(bmp, 22)? as i32;
    let (w, h) = (w.unsigned_abs() as usize, h.unsigned_abs() as usize);
    ((1..=4096).contains(&w) && (1..=4096).contains(&h)).then_some((w, h))
}

/// Bytes per pixel if this is a 24-bit or 32-bit BMP we can read.
/// 32-bit BI_BITFIELDS is accepted only with BGRA masks (TrimUI's u-boot format).
fn pixel_bytes(bmp: &[u8]) -> Option<usize> {
    let bpp = u16_at(bmp, 28)?;
    let compression = u32_at(bmp, 30)?;
    match (bpp, compression) {
        (24, 0) => Some(3),
        (32, 0) => Some(4),
        (32, 3) => {
            if u32_at(bmp, 14)? >= 56 {
                let red = u32_at(bmp, 54)?;
                let green = u32_at(bmp, 58)?;
                let blue = u32_at(bmp, 62)?;
                if red != 0x00FF_0000 || green != 0x0000_FF00 || blue != 0x0000_00FF {
                    return None;
                }
            }
            Some(4)
        }
        _ => None,
    }
}

pub fn decode(bmp: &[u8]) -> Option<Bmp> {
    let (width, height) = size(bmp)?;
    let bpp = pixel_bytes(bmp)?;
    let top_down = (u32_at(bmp, 22)? as i32) < 0;
    let offset = u32_at(bmp, 10)? as usize;
    let stride = (width * bpp + 3) & !3;
    let data = bmp.get(offset..offset + stride * height)?;
    let mut rgb = Vec::with_capacity(width * height);
    for y in 0..height {
        let row = if top_down { y } else { height - 1 - y };
        for px in data[row * stride..].chunks_exact(bpp).take(width) {
            rgb.push([px[2], px[1], px[0]]);
        }
    }
    Some(Bmp { width, height, rgb })
}

pub fn encode(img: &Bmp) -> Vec<u8> {
    let stride = (img.width * 3 + 3) & !3;
    let data = stride * img.height;
    let mut out = Vec::with_capacity(54 + data);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((54 + data) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..img.height).rev() {
        for c in &img.rgb[y * img.width..(y + 1) * img.width] {
            out.extend_from_slice(&[c[2], c[1], c[0]]);
        }
        out.resize(out.len() + stride - img.width * 3, 0);
    }
    out
}

/// 32-bit BITMAPV5 BI_BITFIELDS, rows bottom-up: the format TrimUI's u-boot shows
/// (`brick/os/bootlogo.py`). A 1024×768 picture is 3,145,866 bytes.
pub fn encode_v5(img: &Bmp) -> Vec<u8> {
    let data = img.width * img.height * 4;
    let mut out = Vec::with_capacity(14 + 124 + data);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&((14 + 124 + data) as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&138u32.to_le_bytes());
    out.extend_from_slice(&124u32.to_le_bytes());
    out.extend_from_slice(&(img.width as i32).to_le_bytes());
    out.extend_from_slice(&(img.height as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(data as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0x00FF_0000u32.to_le_bytes());
    out.extend_from_slice(&0x0000_FF00u32.to_le_bytes());
    out.extend_from_slice(&0x0000_00FFu32.to_le_bytes());
    out.extend_from_slice(&0xFF00_0000u32.to_le_bytes());
    out.extend_from_slice(b"BGRs");
    out.extend_from_slice(&[0u8; 36]);
    out.extend_from_slice(&[0u8; 12]);
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 12]);
    for y in (0..img.height).rev() {
        for c in &img.rgb[y * img.width..(y + 1) * img.width] {
            out.extend_from_slice(&[c[2], c[1], c[0], 255]);
        }
    }
    out
}

fn encode_like(img: &Bmp, reference: &[u8]) -> Vec<u8> {
    if pixel_bytes(reference) == Some(4) {
        encode_v5(img)
    } else {
        encode(img)
    }
}

fn turned(img: &Bmp) -> Bmp {
    let (w, h) = (img.height, img.width);
    let mut rgb = vec![[0; 3]; w * h];
    for y in 0..img.height {
        for x in 0..img.width {
            rgb[(img.width - 1 - x) * w + y] = img.rgb[y * img.width + x];
        }
    }
    Bmp {
        width: w,
        height: h,
        rgb,
    }
}

pub fn compose(wordmark: &Bmp, (width, height): (usize, usize)) -> Option<Bmp> {
    let turned_mark;
    let mark = if height > width {
        turned_mark = turned(wordmark);
        &turned_mark
    } else {
        wordmark
    };
    if mark.width > width || mark.height > height {
        return None;
    }
    let mut rgb = vec![*mark.rgb.first()?; width * height];
    let (x0, y0) = ((width - mark.width) / 2, (height - mark.height) / 2);
    for y in 0..mark.height {
        let at = (y0 + y) * width + x0;
        rgb[at..at + mark.width].copy_from_slice(&mark.rgb[y * mark.width..(y + 1) * mark.width]);
    }
    Some(Bmp { width, height, rgb })
}

pub fn install(wordmark: &[u8], dir: &Path) -> io::Result<Outcome> {
    let Some(mark) = decode(wordmark) else {
        return Ok(Outcome::Invalid);
    };
    let current = fs::read(dir.join(LOGO)).ok();
    let original = fs::read(dir.join(ORIGINAL)).ok();
    let Some(panel_bytes) = original.as_deref().or(current.as_deref()) else {
        return Ok(Outcome::Invalid);
    };
    let Some(panel) = size(panel_bytes) else {
        return Ok(Outcome::Invalid);
    };
    let Some(logo) = compose(&mark, panel).map(|b| encode_like(&b, panel_bytes)) else {
        return Ok(Outcome::Invalid);
    };
    if current.as_deref() == Some(&logo[..]) {
        return Ok(Outcome::Same);
    }
    if let Some(current) = &current {
        if original.is_none() {
            replace(dir, ORIGINAL, current)?;
        }
        if current.len() == logo.len() {
            overwrite(dir, LOGO, &logo)?;
            return Ok(Outcome::Installed);
        }
    }
    replace(dir, LOGO, &logo)?;
    Ok(Outcome::Installed)
}

fn overwrite(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut f = fs::OpenOptions::new().write(true).open(dir.join(name))?;
    f.write_all(bytes)?;
    f.sync_all()
}

fn replace(dir: &Path, name: &str, bytes: &[u8]) -> io::Result<()> {
    let tmp = dir.join("bootlogo.tmp");
    let mut f = File::create(&tmp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    fs::rename(&tmp, dir.join(name))?;
    File::open(dir)?.sync_all()
}

pub fn refresh(card: &Path) {
    let Ok(logo) = fs::read(card.join(CARD_LOGO)) else {
        return;
    };
    if decode(&logo).is_none() {
        eprintln!("slot: bootlogo: {CARD_LOGO} is not a 24-bit bmp, leaving the boot logo alone");
        return;
    }
    let Some(device) = partition(PARTITION) else {
        eprintln!("slot: bootlogo: no {PARTITION} partition");
        return;
    };
    if fs::create_dir_all(MOUNT).is_err()
        || !run("mount", &["-t", "vfat", "-o", "rw,noatime", &device, MOUNT])
    {
        eprintln!("slot: bootlogo: could not mount {device}");
        return;
    }
    match install(&logo, Path::new(MOUNT)) {
        Ok(Outcome::Installed) => eprintln!("slot: bootlogo: installed"),
        Ok(Outcome::Invalid) => eprintln!("slot: bootlogo: no logo fits this panel, left alone"),
        Ok(_) => {}
        Err(e) => eprintln!("slot: bootlogo: {e}"),
    }
    if !run("umount", &[MOUNT]) {
        eprintln!("slot: bootlogo: could not unmount {MOUNT}");
    }
}

fn partition(name: &str) -> Option<String> {
    for entry in fs::read_dir("/sys/class/block").ok()?.flatten() {
        let uevent = fs::read_to_string(entry.path().join("uevent")).unwrap_or_default();
        if uevent.lines().any(|l| l == format!("PARTNAME={name}")) {
            return Some(
                PathBuf::from("/dev")
                    .join(entry.file_name())
                    .display()
                    .to_string(),
            );
        }
    }
    None
}

fn run(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}
