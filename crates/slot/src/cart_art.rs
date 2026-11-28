//! Cart labels for games with no label, all from ScreenScraper (`cart_art_ss`) and all made by
//! `slot_cartart`: HOME → Cart Art fetches one for every cart without one, and a game's cart
//! menu can search for others to choose from. A label the user made is never replaced by the
//! bulk run: only carts without one are looked up.
//!
//! Both need the player's ScreenScraper login (HOME → Cart Art → ScreenScraper); without one
//! they say so rather than run. The pictures are fitted on the device, once, and the label kept
//! in `Labels/<system>/` like any other.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use slot_store::{atomic_write, scan, Cart, Platform};

use crate::cart_art_ss::{self as ss, Client, Creds, Got};

/// A label's longer side at most: as large as any view draws it.
const LABEL_MAX: u32 = 512;
/// ROM checksums, so a slow card is read once per ROM rather than once per run.
const CRC_CACHE: &str = "System/Cache/crc32.txt";
const REPORT: &str = "Logs/cart-art.txt";
/// The ROM's CRC32 is worked out only when it is quick to read.
const CRC_MAX: u64 = 64 << 20;
/// Network failures in a row (not refusals, which stop it at once) before a run gives up.
const FAILURES_MAX: usize = 3;

/// What Cart Art says when there is no login to use.
pub const LOG_IN: &str = "Log in to ScreenScraper first";

/// A label written to (or, with None, deleted from) the card, for the running library to pick up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fetched {
    pub platform: Platform,
    pub stem: String,
    pub label: Option<PathBuf>,
}

#[derive(Debug)]
pub enum Progress {
    Step { done: usize, total: usize, name: String },
    Got(Fetched),
    Done(Result<Summary, String>),
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub found: usize,
    pub missing: usize,
    pub failed: usize,
}

/// The account to fetch with, or what to tell the player instead.
pub fn ready(root: &Path) -> Result<Creds, String> {
    match Creds::load(root) {
        Some(c) if c.logged_in() => Ok(c),
        Some(_) => Err(LOG_IN.into()),
        None if !ss::has_developer(root) => Err("This Slot was built without a ScreenScraper developer id".into()),
        None => Err("Cart Art is switched off (cartart=off in Config/screenscraper.txt)".into()),
    }
}

fn client(creds: Creds, api: Option<&str>) -> Client {
    match api {
        Some(api) => Client::with_api(creds, api),
        None => Client::new(creds),
    }
}

pub struct ArtJob {
    rx: mpsc::Receiver<Progress>,
    cancel: Arc<AtomicBool>,
}

impl ArtJob {
    pub fn start(root: PathBuf) -> Self {
        Self::start_at(root, None)
    }

    /// Tests point it at a stand-in ScreenScraper.
    fn start_at(root: PathBuf, api: Option<String>) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let token = cancel.clone();
        let _ = crate::quiet::spawn("slot-cart-art", move || {
            let done = run(&root, api.as_deref(), &token, &tx);
            let _ = tx.send(Progress::Done(done));
        });
        ArtJob { rx, cancel }
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Result<Progress, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}

impl Drop for ArtJob {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn run(root: &Path, api: Option<&str>, cancel: &AtomicBool, tx: &mpsc::Sender<Progress>) -> Result<Summary, String> {
    let creds = ready(root)?;
    let mut report = format!("ScreenScraper, as {}\n", creds.user());
    let mut client = client(creds, api);
    let carts: Vec<Cart> = scan(root)
        .map_err(|e| format!("Cannot read games: {e}"))?
        .into_iter()
        .filter(needs_label)
        .collect();
    let mut summary = Summary::default();
    let mut crcs = Crcs::load(root);
    let mut failures = 0;
    let result = (|| {
        for (at, cart) in carts.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("Cancelled".to_string());
            }
            let _ = tx.send(Progress::Step { done: at, total: carts.len(), name: cart.stem.clone() });
            let place = format!("{}/{}", cart.platform.dir_name(), cart.stem);
            let small = std::fs::metadata(&cart.rom).is_ok_and(|m| m.len() <= CRC_MAX);
            let crc = if small { crcs.of(root, &cart.rom).ok() } else { None };
            match ss::label_for(&mut client, cart, crc) {
                Ok(Got::Label(png, how)) => {
                    failures = 0;
                    let sub = if slot_ui::nes_famicom_of(cart) { slot_store::FAMICOM_LABELS } else { "" };
                    match save_label(root, cart, sub, &png) {
                        Ok(label) => {
                            summary.found += 1;
                            report.push_str(&format!("found     {place} <- {how}\n"));
                            let _ = tx.send(Progress::Got(Fetched { platform: cart.platform, stem: cart.stem.clone(), label: Some(label) }));
                        }
                        Err(why) => {
                            summary.failed += 1;
                            report.push_str(&format!("failed    {place}: {why}\n"));
                        }
                    }
                }
                Ok(Got::NoMatch) => {
                    failures = 0;
                    summary.missing += 1;
                    report.push_str(&format!("no match  {place}\n"));
                }
                Ok(Got::NoArt(why)) => {
                    failures = 0;
                    summary.missing += 1;
                    report.push_str(&format!("no label  {place}: {why}\n"));
                }
                Err(e) => {
                    let e = ss::redact(&e);
                    report.push_str(&format!("failed    {place}: {e}\n"));
                    if ss::stops_the_run(&e) {
                        return Err(e);
                    }
                    summary.failed += 1;
                    failures += 1;
                    if failures >= FAILURES_MAX {
                        return Err(format!("Stopped: {e}"));
                    }
                }
            }
        }
        Ok(())
    })();
    // Checksums read before a cancel or a failure are kept for next time.
    crcs.save(root);
    if let Err(e) = std::fs::create_dir_all(root.join("Logs")).and_then(|_| std::fs::write(root.join(REPORT), &report)) {
        eprintln!("slot: cart art: {REPORT}: {e}");
    }
    result?;
    eprintln!("slot: cart art: {} found, {} no match, {} failed", summary.found, summary.missing, summary.failed);
    Ok(summary)
}

/// No label for the cart it is on now: a NES game on the Famicom cart wants a Famicom label
/// even when it has a NES one.
fn needs_label(cart: &Cart) -> bool {
    match cart.platform {
        Platform::Nes if slot_ui::nes_famicom_of(cart) => cart.famicom_label.is_none(),
        _ => cart.label.is_none(),
    }
}

fn save_label(root: &Path, cart: &Cart, sub: &str, png: &[u8]) -> Result<PathBuf, String> {
    let label = root.join("Labels").join(cart.platform.dir_name()).join(sub).join(format!("{}.png", cart.stem));
    if label.exists() {
        return Err("a label appeared meanwhile; kept it".into());
    }
    if let Some(dir) = label.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    atomic_write(&label, png).map_err(|e| format!("cannot write label: {e}"))?;
    Ok(label)
}

/// Where on a picture a label sits. Kept beside a search's pictures so the frame can be moved
/// by hand later: nothing is cut until someone says so.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Frame {
    /// Where the frame's top left corner sits on the picture. Can be off it: a picture shrunk
    /// smaller than the label leaves the label's edges empty, which is allowed.
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

/// The piece of `png` inside `frame`. Where the frame reaches past the picture, which it does
/// once the picture has been shrunk smaller than the label, what is out there is left clear,
/// so the cart's recess shows through at the edges.
fn cut_rgba(png: &[u8], frame: Frame) -> Result<(Vec<u8>, u32, u32), &'static str> {
    let (rgba, w, h) = decode_rgba(png).ok_or("not a readable PNG")?;
    let (lw, lh) = (frame.w, frame.h);
    if lw == 0 || lh == 0 {
        return Err("an empty picture");
    }
    let mut out = vec![0u8; (lw as usize) * (lh as usize) * 4];
    for row in 0..lh {
        let sy = frame.y + row as i32;
        if sy < 0 || sy >= h as i32 {
            continue;
        }
        // Where this row of the frame overlaps the picture, in both of their own columns.
        let from = frame.x.max(0);
        let to = (frame.x + lw as i32).min(w as i32);
        if to <= from {
            continue;
        }
        let run = (to - from) as usize * 4;
        let src = ((sy as usize) * w as usize + from as usize) * 4;
        let dst = (row as usize * lw as usize + (from - frame.x) as usize) * 4;
        out[dst..dst + run].copy_from_slice(&rgba[src..src + run]);
    }
    Ok((out, lw, lh))
}

/// The piece of `png` inside `frame`, as a label of its own shape.
pub fn cut_out(png: &[u8], frame: Frame) -> Result<Vec<u8>, &'static str> {
    let (out, lw, lh) = cut_rgba(png, frame)?;
    let (out, lw, lh) = shrink(out, lw, lh, LABEL_MAX);
    encode(&out, lw, lh)
}

/// The piece of `png` inside `frame`, in the shape `into` wants. A frame can be sized a side
/// at a time, so it need not be the shape of the slot it is bound for; the cart fills its slot
/// by cropping, so a label handed over in another shape would be cropped rather than stretched
/// and would not be what was framed. Stretching it here is what makes the two agree.
pub fn cut_to(png: &[u8], frame: Frame, into: (u32, u32)) -> Result<Vec<u8>, &'static str> {
    let (out, nw, nh) = cut_label(png, frame, into)?;
    encode(&out, nw, nh)
}

/// The label `cut_to` makes, as RGBA and its size, before it is made a PNG.
pub fn cut_label(png: &[u8], frame: Frame, into: (u32, u32)) -> Result<(Vec<u8>, u32, u32), &'static str> {
    let (out, lw, lh) = cut_rgba(png, frame)?;
    let (iw, ih) = (into.0.max(1), into.1.max(1));
    let long = lw.max(lh).clamp(1, LABEL_MAX);
    // Rounded, not floored: a whole number of pixels can only come so close to a given shape,
    // and rounding keeps it to within a pixel rather than always falling short.
    let other = |want: u32, of: u32| ((long as u64 * want as u64 + of as u64 / 2) / of as u64).max(1) as u32;
    let (nw, nh) = match iw >= ih {
        true => (long, other(ih, iw)),
        false => (other(iw, ih), long),
    };
    Ok((resample(&out, lw, lh, nw, nh), nw, nh))
}

fn encode(rgba: &[u8], w: u32, h: u32) -> Result<Vec<u8>, &'static str> {
    let mut bytes = Vec::new();
    let mut e = png::Encoder::new(&mut bytes, w, h);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header()
        .and_then(|mut w| w.write_image_data(rgba))
        .map_err(|_| "cannot encode the label")?;
    Ok(bytes)
}

/// To any size, each way on its own: the average of what each new pixel covers when there is
/// more than one, and the nearest when the picture is being stretched rather than squeezed.
fn resample(rgba: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    if (w, h) == (nw, nh) {
        return rgba.to_vec();
    }
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        let (sy0, sy1) = (y * h / nh, ((y + 1) * h / nh).max(y * h / nh + 1));
        for x in 0..nw {
            let (sx0, sx1) = (x * w / nw, ((x + 1) * w / nw).max(x * w / nw + 1));
            let mut sum = [0u64; 4];
            for sy in sy0..sy1.min(h) {
                for sx in sx0..sx1.min(w) {
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..4 {
                        sum[c] += rgba[i + c] as u64;
                    }
                }
            }
            let n = ((sy1.min(h) - sy0) * (sx1.min(w) - sx0)).max(1) as u64;
            out.extend(sum.map(|v| (v / n) as u8));
        }
    }
    out
}

/// A picture's size, without decoding it: the width and height in its IHDR.
pub fn png_size(png: &[u8]) -> Option<(u32, u32)> {
    let ihdr = png.get(12..16)?;
    if ihdr != b"IHDR" {
        return None;
    }
    let n = |at: usize| -> Option<u32> { Some(u32::from_be_bytes(png.get(at..at + 4)?.try_into().ok()?)) };
    match (n(16)?, n(20)?) {
        (0, _) | (_, 0) => None,
        size => Some(size),
    }
}

/// `rgba` (w x h) scaled down so its longer side is at most `max`, each pixel the average of
/// the ones it covers; a smaller picture is kept as it is.
fn shrink(rgba: Vec<u8>, w: u32, h: u32, max: u32) -> (Vec<u8>, u32, u32) {
    if w.max(h) <= max {
        return (rgba, w, h);
    }
    let scale = |v: u32| ((v as u64 * max as u64) / w.max(h) as u64).max(1) as u32;
    let (nw, nh) = (scale(w), scale(h));
    let mut out = Vec::with_capacity((nw * nh * 4) as usize);
    for y in 0..nh {
        let (sy0, sy1) = (y * h / nh, ((y + 1) * h / nh).max(y * h / nh + 1));
        for x in 0..nw {
            let (sx0, sx1) = (x * w / nw, ((x + 1) * w / nw).max(x * w / nw + 1));
            let mut sum = [0u64; 4];
            for sy in sy0..sy1 {
                for sx in sx0..sx1 {
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..4 {
                        sum[c] += rgba[i + c] as u64;
                    }
                }
            }
            let n = ((sy1 - sy0) * (sx1 - sx0)) as u64;
            out.extend(sum.map(|v| (v / n) as u8));
        }
    }
    (out, nw, nh)
}

pub(crate) fn decode_rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut dec = png::Decoder::new(bytes);
    dec.set_transformations(png::Transformations::normalize_to_color8());
    dec.set_limits(png::Limits { bytes: 64 << 20 });
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let n = (info.width as usize).checked_mul(info.height as usize)?;
    let src = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => src.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => src.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == n * 4).then_some((rgba, info.width, info.height))
}

/// ROM checksums by path, kept with each file's size and mtime.
struct Crcs {
    known: HashMap<String, (u64, u64, u32)>,
    changed: bool,
}

impl Crcs {
    fn load(root: &Path) -> Self {
        let text = std::fs::read_to_string(root.join(CRC_CACHE)).unwrap_or_default();
        let known = text
            .lines()
            .filter_map(|l| {
                let mut f = l.splitn(4, '\t');
                let crc = u32::from_str_radix(f.next()?, 16).ok()?;
                let size = f.next()?.parse().ok()?;
                let mtime = f.next()?.parse().ok()?;
                Some((f.next()?.to_string(), (size, mtime, crc)))
            })
            .collect();
        Crcs {
            known,
            changed: false,
        }
    }

    fn of(&mut self, root: &Path, rom: &Path) -> std::io::Result<u32> {
        let meta = std::fs::metadata(rom)?;
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |d| d.as_secs());
        let key = rom.strip_prefix(root).unwrap_or(rom).to_string_lossy().into_owned();
        if let Some(&(size, when, crc)) = self.known.get(&key) {
            if (size, when) == (meta.len(), mtime) {
                return Ok(crc);
            }
        }
        let crc = slot_store::archive::crc32(rom)?;
        self.known.insert(key, (meta.len(), mtime, crc));
        self.changed = true;
        Ok(crc)
    }

    fn save(&self, root: &Path) {
        if !self.changed {
            return;
        }
        let mut lines: Vec<String> = self
            .known
            .iter()
            .filter(|(k, _)| !k.contains(['\n', '\t']))
            .map(|(k, (size, mtime, crc))| format!("{crc:08X}\t{size}\t{mtime}\t{k}"))
            .collect();
        lines.sort();
        let path = root.join(CRC_CACHE);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = atomic_write(&path, lines.join("\n").as_bytes()) {
            eprintln!("slot: cart art: {CRC_CACHE}: {e}");
        }
    }
}

/// The widest or tallest picture read from the card, for labels and overlays alike.
pub(crate) const PICTURE_SIDE_MAX: u32 = 4096;


/// Where the labels a search finds wait, as pictures, until one is kept.
pub const PICK_DIR: &str = "System/Cache/LabelPick";
/// Enough to choose from: more are mostly the same picture again.
/// How far past the page being looked at a search runs: one page more, and no further. The
/// list is as long as the console's game list, so the next page being ready the moment it is
/// turned to is worth fetching ahead for; the one after it is not, until it is asked for.
const LOOKAHEAD: usize = slot_ui::PAGE;
/// How often a search that is far enough ahead looks again at how far the picker has flipped.
const WAIT: Duration = Duration::from_millis(100);

/// A label found for the cart menu to offer: where it waits, and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// The label as cut, which is what the grid shows.
    pub path: PathBuf,
    pub caption: String,
    /// The whole picture it was cut from, kept so the frame can be moved by hand.
    pub source: PathBuf,
    /// Where on `source` the cut was taken.
    pub frame: Frame,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PickNews {
    Found(Choice),
    /// The search is over: Err when it could not be made at all (no network, no index).
    Done(Result<(), String>),
}

/// Every label the sources have for one cart, fetched one after another in the background,
/// so the cart menu can offer them to choose from. Dropping it stops the search.
pub struct LabelSearch {
    rx: mpsc::Receiver<PickNews>,
    cancel: Arc<AtomicBool>,
    /// Which label is being looked at: the search keeps LOOKAHEAD ahead of it and no further.
    looked_at: Arc<AtomicUsize>,
}

impl LabelSearch {
    /// Labels for `cart`, made from the pictures of every game ScreenScraper finds by what was
    /// typed, best match first.
    pub fn start(root: PathBuf, cart: Cart, query: String) -> Self {
        // A sim scenario never goes to the internet: it is handed a made-up label instead.
        #[cfg(feature = "sim")]
        if std::env::var_os("SLOT_SIM_SCRIPT").is_some() {
            return Self::stand_in(root, cart);
        }
        Self::start_at(root, cart, query, None)
    }

    /// One label, drawn here: a striped picture with a disc on it, framed in its middle.
    #[cfg(feature = "sim")]
    fn stand_in(root: PathBuf, cart: Cart) -> Self {
        let (tx, rx) = mpsc::channel();
        let offered = (|| {
            let (pw, ph) = (480u32, 360u32);
            let mut rgba = Vec::with_capacity((pw * ph * 4) as usize);
            for y in 0..ph {
                for x in 0..pw {
                    let (dx, dy) = (x as f32 - 240.0, y as f32 - 180.0);
                    let px = match () {
                        _ if dx * dx + dy * dy < 90.0 * 90.0 => [30, 30, 46],
                        _ if (x + y) / 40 % 2 == 0 => [226, 72, 58],
                        _ => [246, 190, 64],
                    };
                    rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
            let whole = encode(&rgba, pw, ph)?;
            let (lw, lh) = slot_ui::label_size(&cart);
            let w = 360u32;
            let h = (w * lh / lw.max(1)).clamp(1, ph - 20);
            let frame = Frame { x: ((pw - w) / 2) as i32, y: ((ph - h) / 2) as i32, w, h };
            let cut = cut_to(&whole, frame, (lw, lh))?;
            let dir = root.join(PICK_DIR).join("sim");
            std::fs::create_dir_all(&dir).map_err(|_| "cannot write to the card")?;
            let (path, source) = (dir.join("0.png"), dir.join("0-whole.png"));
            atomic_write(&path, &cut).map_err(|_| "cannot write to the card")?;
            atomic_write(&source, &whole).map_err(|_| "cannot write to the card")?;
            Ok::<_, &str>(Choice { path, caption: "Sim Label (Stand-in)".into(), source, frame })
        })();
        let done = offered.map(|choice| {
            let _ = tx.send(PickNews::Found(choice));
        });
        let _ = tx.send(PickNews::Done(done.map_err(str::to_string)));
        LabelSearch { rx, cancel: Arc::new(AtomicBool::new(false)), looked_at: Arc::new(AtomicUsize::new(0)) }
    }

    fn start_at(root: PathBuf, cart: Cart, query: String, api: Option<String>) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let token = cancel.clone();
        let looked_at = Arc::new(AtomicUsize::new(0));
        let place = looked_at.clone();
        let spawned = thread::Builder::new().name("slot-label-search".into()).spawn(move || {
            let done = search(&root, &cart, &query, api.as_deref(), &token, &place, &tx);
            let _ = tx.send(PickNews::Done(done));
        });
        if let Err(e) = spawned {
            eprintln!("slot: label search: {e}");
        }
        LabelSearch { rx, cancel, looked_at }
    }

    pub fn poll(&self) -> Vec<PickNews> {
        self.rx.try_iter().collect()
    }

    /// The label the picker has flipped to, which is how far ahead the search may run.
    pub fn looking_at(&self, at: usize) {
        self.looked_at.fetch_max(at, Ordering::Relaxed);
    }
}

impl Drop for LabelSearch {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// "Label", "Cart", "Box", "Logo" from how a label was made, for its caption.
fn picture_kind(how: &str) -> &'static str {
    match how.split_whitespace().nth(1).unwrap_or("") {
        "support-texture" => "Label",
        "support-2D" => "Cart",
        "box-2D" => "Box",
        "marquee" | "screenmarquee" => "Marquee",
        _ => "Logo",
    }
}

fn search(root: &Path, cart: &Cart, query: &str, api: Option<&str>, cancel: &AtomicBool, looked_at: &AtomicUsize, tx: &mpsc::Sender<PickNews>) -> Result<(), String> {
    let mut client = client(ready(root)?, api);
    // Each search in a folder of its own, the last one's cleared away: a search stopped
    // mid-download can then never put its picture among the next one's.
    let _ = std::fs::remove_dir_all(root.join(PICK_DIR));
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let dir = root.join(PICK_DIR).join(stamp.to_string());
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot write to the card: {e}"))?;
    let mut found = 0;
    // A label goes to the card (the face builder reads it from there) and then to the menu. It
    // is already fitted to the cart, so it is its own whole picture to frame by hand.
    let mut offer = |png: &[u8], caption: String| -> bool {
        let path = dir.join(format!("{found}.png"));
        if atomic_write(&path, png).is_err() {
            return true;
        }
        let Some((w, h)) = png_size(png) else { return true };
        found += 1;
        let frame = Frame { x: 0, y: 0, w, h };
        if tx.send(PickNews::Found(Choice { path: path.clone(), caption, source: path, frame })).is_err() {
            return false;
        }
        // Nothing is left out of the list, so wait here rather than fetch a picture that is a
        // hundred flips away. Flipping on raises `looked_at` and the search carries on.
        while found >= looked_at.load(Ordering::Relaxed) + LOOKAHEAD {
            if cancel.load(Ordering::Relaxed) {
                return false;
            }
            thread::sleep(WAIT);
        }
        true
    };
    let games = client.search(cart.platform, query).map_err(|e| ss::redact(&e))?;
    for game in games {
        if cancel.load(Ordering::Relaxed) {
            return Ok(());
        }
        let mut go_on = true;
        let fitted = ss::fit_from(&mut client, cart, &game.medias, true, &mut |png, how| {
            go_on = offer(&png, format!("{} · {}", game.name, picture_kind(&how)));
            go_on && !cancel.load(Ordering::Relaxed)
        });
        match fitted {
            Err(e) if ss::stops_the_run(&e) => return Err(ss::redact(&e)),
            Err(e) => eprintln!("slot: label search: {}", ss::redact(&e)),
            Ok(_) => {}
        }
        if !go_on {
            return Ok(());
        }
    }
    Ok(())
}

/// A label as a picture to show in the grid: its pixels, no bigger than `max` either way.
pub fn label_picture(png: &[u8], max: u32) -> Option<(Vec<u8>, u32, u32)> {
    let (rgba, w, h) = decode_rgba(png)?;
    let (rgba, w, h) = if w.max(h) > max { shrink(rgba, w, h, max) } else { (rgba, w, h) };
    (w > 0 && h > 0).then_some((rgba, w, h))
}

/// The labels a search found and nobody kept, gone from the card. In the background: the
/// card is slow, and nothing waits on it.
pub fn forget_choices(root: &Path) {
    // Only the searches there now: one started a moment later keeps its folder.
    let Ok(entries) = std::fs::read_dir(root.join(PICK_DIR)) else {
        return;
    };
    let old: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    let spawned = thread::Builder::new().name("slot-label-pick".into()).spawn(move || {
        for dir in old {
            if let Err(e) = std::fs::remove_dir_all(&dir).or_else(|_| std::fs::remove_file(&dir)) {
                eprintln!("slot: label search: {}: {e}", dir.display());
            }
        }
    });
    if let Err(e) = spawned {
        eprintln!("slot: label search: {e}");
    }
}

/// The label kept from a search: written where the cart's label for its current shape goes,
/// in place of any it had.
/// Writes the piece of `source` inside `frame` as the cart's label: the cut happens here, once
/// whoever picked it has said where the frame goes.
pub fn keep_framed(root: &Path, cart: &Cart, source: &Path, frame: Frame) -> std::io::Result<PathBuf> {
    let png = std::fs::read(source)?;
    let cut = cut_label(&png, frame, slot_ui::label_size(cart))
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    keep_cut(root, cart, &cut)
}

/// Keeps a label already cut (`cut_label`) as the cart's own.
pub fn keep_cut(root: &Path, cart: &Cart, (rgba, w, h): &(Vec<u8>, u32, u32)) -> std::io::Result<PathBuf> {
    let cut = encode(rgba, *w, *h).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let target = label_target(root, cart);
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&target, &cut)?;
    Ok(target)
}

pub fn keep_choice(root: &Path, cart: &Cart, choice: &Path) -> std::io::Result<PathBuf> {
    let target = label_target(root, cart);
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&target, &std::fs::read(choice)?)?;
    Ok(target)
}

/// Where a cart's label goes: the one it already has, or a new file named after the game.
fn label_target(root: &Path, cart: &Cart) -> PathBuf {
    let famicom = cart.platform == Platform::Nes && slot_ui::nes_famicom_of(cart);
    let existing = if famicom { &cart.famicom_label } else { &cart.label };
    let dir = root.join("Labels").join(cart.platform.dir_name()).join(if famicom { slot_store::FAMICOM_LABELS } else { "" });
    // A label already there under another case keeps its name, so no second one appears.
    existing
        .as_ref()
        .filter(|p| !p.starts_with(root.join(PICK_DIR)))
        .cloned()
        .unwrap_or_else(|| dir.join(format!("{}.png", cart.stem)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in ScreenScraper: answers by path (the query ignored), `{BASE}` in a reply
    /// replaced by its own address.
    fn fake(routes: Vec<(&'static str, u16, Vec<u8>)>) -> String {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let addr = base.clone();
        thread::spawn(move || {
            for conn in listener.incoming() {
                let Ok(mut conn) = conn else { return };
                let mut raw = Vec::new();
                let mut buf = [0u8; 2048];
                while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                    match conn.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => raw.extend_from_slice(&buf[..n]),
                    }
                }
                let line = String::from_utf8_lossy(&raw).lines().next().unwrap_or("").to_string();
                let target = line.split_whitespace().nth(1).unwrap_or("").to_string();
                let path = target.split('?').next().unwrap_or("");
                let (status, body) = routes
                    .iter()
                    .find(|(p, _, _)| *p == path)
                    .map(|(_, s, b)| match b.starts_with(b"\x89PNG") {
                        true => (*s, b.clone()),
                        false => (*s, String::from_utf8_lossy(b).replace("{BASE}", &base).into_bytes()),
                    })
                    .unwrap_or((404, Vec::new()));
                let _ = write!(conn, "HTTP/1.1 {status} X\r\nContent-Length: {}\r\n\r\n", body.len());
                let _ = conn.write_all(&body);
            }
        });
        addr
    }

    /// A label scan: a striped picture the GB label window's shape.
    fn scan_png() -> Vec<u8> {
        let (w, h) = (300u32, 266u32);
        let px: Vec<u8> = (0..w * h).flat_map(|i| [((i % w) * 7 % 200) as u8, ((i / w) % 256) as u8, 90, 255]).collect();
        slot_cartart::encode_png_rgb(&slot_cartart::Image::from_rgba(w, h, px).unwrap())
    }

    fn rom(root: &Path, rel: &str) {
        let path = root.join("Games").join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, vec![0u8; 0x200]).unwrap();
    }

    fn login(root: &Path, user: bool) {
        std::fs::create_dir_all(root.join("Config")).unwrap();
        let account = if user { "user=jeff\npassword=pw\n" } else { "" };
        std::fs::write(root.join(ss::CONFIG), format!("devid=dev\ndevpassword=devpw\n{account}")).unwrap();
    }

    const GAME: &[u8] = br#"{"response":{"jeu":{"medias":[{"type":"support-texture","url":"{BASE}/api2/mediaJeu.php?media=support-texture(us)","region":"us"}]}}}"#;

    fn drain(job: &ArtJob) -> (Vec<Fetched>, Result<Summary, String>) {
        let mut got = Vec::new();
        loop {
            match job.rx.recv_timeout(Duration::from_secs(20)).expect("job finished") {
                Progress::Got(f) => got.push(f),
                Progress::Done(r) => return (got, r),
                Progress::Step { .. } => {}
            }
        }
    }

    #[test]
    fn cart_art_asks_for_a_login_rather_than_run_without_one() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        rom(root, "GB/Tetris (World).gb");
        let (_, done) = drain(&ArtJob::start_at(root.into(), Some("http://127.0.0.1:1".into())));
        assert_eq!(done.unwrap_err(), "This Slot was built without a ScreenScraper developer id");
        login(root, false);
        assert_eq!(ready(root).err().unwrap(), LOG_IN);
        let (_, done) = drain(&ArtJob::start_at(root.into(), Some("http://127.0.0.1:1".into())));
        assert_eq!(done.unwrap_err(), LOG_IN);
        let search = LabelSearch::start_at(root.into(), slot_store::scan(root).unwrap().remove(0), "tetris".into(), Some("http://127.0.0.1:1".into()));
        assert_eq!(search_all(&search).1.unwrap_err(), LOG_IN);
        std::fs::write(root.join(ss::CONFIG), "cartart=off\ndevid=dev\ndevpassword=devpw\nuser=jeff\npassword=pw\n").unwrap();
        assert!(ready(root).err().unwrap().contains("switched off"));
    }

    #[test]
    fn every_cart_without_a_label_gets_one_from_screenscraper() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        login(root, true);
        rom(root, "GB/Tetris (World).gb");
        rom(root, "MD/Sonic The Hedgehog (USA, Europe).md");
        std::fs::create_dir_all(root.join("Labels/GBA")).unwrap();
        rom(root, "GBA/Kept (USA).gba");
        std::fs::write(root.join("Labels/GBA/Kept (USA).png"), scan_png()).unwrap();
        let api = fake(vec![("/api2/jeuInfos.php", 200, GAME.to_vec()), ("/api2/mediaJeu.php", 200, scan_png())]);
        let (got, done) = drain(&ArtJob::start_at(root.into(), Some(api)));
        assert_eq!(done.unwrap(), Summary { found: 2, missing: 0, failed: 0 });
        let mut stems: Vec<_> = got.iter().map(|f| f.stem.as_str()).collect();
        stems.sort();
        assert_eq!(stems, ["Sonic The Hedgehog (USA, Europe)", "Tetris (World)"], "a label already there is kept");
        let label = std::fs::read(root.join("Labels/GB/Tetris (World).png")).unwrap();
        let (w, h) = png_size(&label).unwrap();
        let (lw, lh) = slot_ui::label_size(&slot_store::scan(root).unwrap().into_iter().find(|c| c.platform == Platform::Gb).unwrap());
        assert_eq!((w, h), (lw * 2, lh * 2), "twice the window: the sharper label is the default");
        let report = std::fs::read_to_string(root.join(REPORT)).unwrap();
        assert!(report.contains("ScreenScraper, as jeff") && !report.contains("pw"), "{report}");
    }

    #[test]
    fn a_refused_login_or_spent_quota_stops_the_run_at_once() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        login(root, true);
        rom(root, "GB/A (World).gb");
        rom(root, "GB/B (World).gb");
        let api = fake(vec![("/api2/jeuInfos.php", 403, Vec::new())]);
        let (_, done) = drain(&ArtJob::start_at(root.into(), Some(api)));
        assert!(done.unwrap_err().contains("refused the login"));
        let api = fake(vec![("/api2/jeuInfos.php", 430, Vec::new())]);
        let (_, done) = drain(&ArtJob::start_at(root.into(), Some(api)));
        assert!(done.unwrap_err().contains("quota"));
    }

    fn search_all(search: &LabelSearch) -> (Vec<Choice>, Result<(), String>) {
        let mut found = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        while std::time::Instant::now() < deadline {
            for news in search.poll() {
                match news {
                    PickNews::Found(c) => found.push(c),
                    PickNews::Done(done) => return (found, done),
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("the search never ended");
    }

    #[test]
    fn a_label_search_offers_each_games_labels_and_keeps_the_one_picked() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        login(root, true);
        rom(root, "GB/Tetris (World).gb");
        let cart = slot_store::scan(root).unwrap().remove(0);
        let found = br#"{"response":{"jeux":[
            {"id":"1","noms":[{"region":"wor","text":"Tetris"}],"medias":[{"type":"support-texture","url":"{BASE}/api2/mediaJeu.php?media=t","region":"wor"},{"type":"box-2D","url":"{BASE}/api2/mediaJeu.php?media=b","region":"wor"}]},
            {"id":"2","noms":[{"region":"jp","text":"Tetris DX"}],"medias":[{"type":"support-texture","url":"{BASE}/api2/mediaJeu.php?media=t2","region":"jp"}]}]}}"#;
        let api = fake(vec![("/api2/jeuRecherche.php", 200, found.to_vec()), ("/api2/mediaJeu.php", 200, scan_png())]);
        let search = LabelSearch::start_at(root.into(), cart.clone(), "tetris".into(), Some(api));
        search.looking_at(100);
        let (choices, done) = search_all(&search);
        done.unwrap();
        let captions: Vec<_> = choices.iter().map(|c| c.caption.as_str()).collect();
        assert_eq!(captions, ["Tetris · Label", "Tetris · Box", "Tetris DX · Label"]);
        let kept = keep_choice(root, &cart, &choices[0].path).unwrap();
        assert_eq!(kept, root.join("Labels/GB/Tetris (World).png"));
        assert_eq!(std::fs::read(&kept).unwrap(), std::fs::read(&choices[0].path).unwrap());
        assert_eq!(choices[0].frame, Frame { x: 0, y: 0, w: png_size(&std::fs::read(&kept).unwrap()).unwrap().0, h: png_size(&std::fs::read(&kept).unwrap()).unwrap().1 });
    }

    /// Collects what a search has offered so far, giving it a moment to stop on its own.
    fn search_so_far(search: &LabelSearch) -> (Vec<Choice>, bool) {
        let mut found = Vec::new();
        let mut done = false;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            for news in search.poll() {
                match news {
                    PickNews::Found(c) => found.push(c),
                    PickNews::Done(_) => done = true,
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        (found, done)
    }

    #[test]
    fn a_search_stays_a_page_ahead_until_the_picker_flips_on() {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        login(root, true);
        rom(root, "GB/Tetris (World).gb");
        let cart = slot_store::scan(root).unwrap().remove(0);
        let games: Vec<String> = (0..40)
            .map(|i| format!(r#"{{"id":"{i}","noms":[{{"region":"wor","text":"Game {i}"}}],"medias":[{{"type":"support-texture","url":"{{BASE}}/api2/mediaJeu.php?media={i}","region":"wor"}}]}}"#))
            .collect();
        let json = format!(r#"{{"response":{{"jeux":[{}]}}}}"#, games.join(","));
        let api = fake(vec![("/api2/jeuRecherche.php", 200, json.into_bytes()), ("/api2/mediaJeu.php", 200, scan_png())]);
        let search = LabelSearch::start_at(root.into(), cart, "game".into(), Some(api));
        let (first, done) = search_so_far(&search);
        assert!(!done, "it ran on past the page ahead");
        assert_eq!(first.len(), LOOKAHEAD);
        search.looking_at(LOOKAHEAD);
        let (more, _) = search_so_far(&search);
        assert_eq!(more.len(), LOOKAHEAD);
    }

    /// A picture in four coloured quarters, so where a cut landed can be read off its colour.
    fn quartered(w: u32, h: u32) -> Vec<u8> {
        let corner = |x: u32, y: u32| match (x < w / 2, y < h / 2) {
            (true, true) => [0xff, 0, 0, 0xff],
            (false, true) => [0, 0xff, 0, 0xff],
            (true, false) => [0, 0, 0xff, 0xff],
            (false, false) => [0xff, 0xff, 0, 0xff],
        };
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&corner(x, y));
            }
        }
        let mut out = Vec::new();
        let mut e = png::Encoder::new(&mut out, w, h);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.write_header().unwrap().write_image_data(&rgba).unwrap();
        out
    }

    #[test]
    fn a_frame_cuts_the_piece_it_covers_and_nothing_else() {
        let png = quartered(200, 100);
        assert_eq!(png_size(&png), Some((200, 100)));
        // The top right quarter is the green one, whole.
        let cut = cut_out(&png, Frame { x: 100, y: 0, w: 100, h: 50 }).unwrap();
        let (rgba, w, h) = decode_rgba(&cut).unwrap();
        assert_eq!((w, h), (100, 50));
        assert!(rgba.chunks_exact(4).all(|p| p == [0, 0xff, 0, 0xff]), "the cut took in another quarter");
        // And the bottom left is the blue one.
        let cut = cut_out(&png, Frame { x: 0, y: 50, w: 100, h: 50 }).unwrap();
        let (rgba, _, _) = decode_rgba(&cut).unwrap();
        assert!(rgba.chunks_exact(4).all(|p| p == [0, 0, 0xff, 0xff]));
    }

    #[test]
    fn a_frame_reaching_past_the_picture_leaves_what_is_out_there_clear() {
        let png = quartered(200, 100);
        // Past the right and bottom edges: the frame keeps its own size, and what hangs over
        // the edge is clear rather than clipped away.
        let cut = cut_out(&png, Frame { x: 150, y: 60, w: 400, h: 400 }).unwrap();
        let (over, w, h) = decode_rgba(&cut).unwrap();
        assert_eq!((w, h), (400, 400));
        let px = |rgba: &[u8], w: u32, x: u32, y: u32| -> [u8; 4] {
            let i = ((y * w + x) * 4) as usize;
            [rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
        };
        assert_eq!(px(&over, w, 10, 10), [0xff, 0xff, 0, 0xff], "the overlap was not cut");
        assert_eq!(px(&over, w, 399, 399), [0, 0, 0, 0], "past the picture was not left clear");
        // Wholly beyond the picture: a label's worth of nothing, rather than an error.
        assert!(cut_out(&png, Frame { x: 900, y: 900, w: 10, h: 10 }).is_ok());
        // A frame larger than the picture keeps it whole, with the rest left clear.
        let cut = cut_out(&png, Frame { x: -100, y: -50, w: 400, h: 200 }).unwrap();
        let (rgba, w, h) = decode_rgba(&cut).unwrap();
        assert_eq!((w, h), (400, 200));
        assert_eq!(px(&rgba, w, 0, 0), [0, 0, 0, 0], "outside the picture was not left clear");
        assert_eq!(px(&rgba, w, 399, 199), [0, 0, 0, 0]);
        // The picture itself sits at (100, 50) of the frame: its top left is the red quarter.
        assert_eq!(px(&rgba, w, 101, 51), [0xff, 0, 0, 0xff]);
        assert_eq!(cut_out(&png, Frame { x: 0, y: 0, w: 0, h: 10 }), Err("an empty picture"));
        assert_eq!(png_size(b"<html>"), None);
    }

    #[test]
    fn a_label_is_cut_in_the_shape_its_slot_wants() {
        let png = quartered(200, 100);
        // A frame of one shape, a slot of another: the label comes out in the slot's shape, so
        // the cart stretches it the way it was framed instead of cropping it to fit.
        let frame = Frame { x: 0, y: 0, w: 200, h: 100 };
        // A whole number of pixels only comes so close to a given shape, so near enough is the
        // contract: within a pixel of the slot's own.
        let shape_of = |w: u32, h: u32| w as f32 / h as f32;
        let near = |got: f32, want: f32| (got / want - 1.0).abs() < 0.02;
        let (_, w, h) = decode_rgba(&cut_to(&png, frame, (216, 99)).unwrap()).unwrap();
        assert!(near(shape_of(w, h), shape_of(216, 99)), "{w} x {h} is not the slot's shape");
        // A tall slot gets a tall label from the very same frame.
        let (_, w, h) = decode_rgba(&cut_to(&png, frame, (99, 216)).unwrap()).unwrap();
        assert!(near(shape_of(w, h), shape_of(99, 216)), "{w} x {h} is not the slot's shape");
        // What it holds is still the picture: the top left stays the red quarter.
        let cut = cut_to(&png, frame, (216, 99)).unwrap();
        let (rgba, w, _) = decode_rgba(&cut).unwrap();
        let at = |x: u32, y: u32| rgba[(((y * w + x) * 4) as usize)..][..4].to_vec();
        assert_eq!(at(2, 2), vec![0xff, 0, 0, 0xff]);
        // And no label is written larger than a label needs to be.
        let big = Frame { x: 0, y: 0, w: 4000, h: 2000 };
        let (_, w, h) = decode_rgba(&cut_to(&png, big, (216, 99)).unwrap()).unwrap();
        assert!(w.max(h) <= LABEL_MAX, "{w} x {h}");
    }

    #[test]
    fn a_big_label_is_scaled_down_by_averaging_and_a_small_one_kept() {
        let px = |c: u8| [c, c, c, 255];
        // 4x4: left half black, right half white.
        let rgba: Vec<u8> = (0..16).flat_map(|i| px(if i % 4 < 2 { 0 } else { 255 })).collect();
        let (out, w, h) = shrink(rgba.clone(), 4, 4, 2);
        assert_eq!((w, h), (2, 2));
        assert_eq!(out, [px(0), px(255), px(0), px(255)].concat());
        let (odd, w, h) = shrink(rgba.clone(), 4, 4, 3);
        assert_eq!((w, h, odd.len()), (3, 3, 36), "a size that does not divide evenly");
        assert_eq!(shrink(rgba.clone(), 4, 4, 4), (rgba, 4, 4));
    }
}
