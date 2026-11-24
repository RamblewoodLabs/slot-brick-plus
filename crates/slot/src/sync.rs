//! Continue-anywhere saves. A Mac on the home LAN holds the newest copy. Slot
//! pushes what changed and pulls what is newer. See docs/sync.md.
//!
//! Off unless `Config/sync.txt` exists and names a token. The token is never logged.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use slot_store::{Core, Platform};

const FILE: &str = "Config/sync.txt";
pub const DEFAULT_PORT: u16 = 8791;
const CONNECT: Duration = Duration::from_millis(350);
const LAUNCH_BUDGET: Duration = Duration::from_millis(1000);
const BACKGROUND_BUDGET: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub token: String,
    pub device: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Remote {
    mtime: u64,
    #[allow(dead_code)]
    device: String,
    sha256: String,
    /// Carried in the index so a later app can show it. The client trusts the body it downloads.
    #[allow(dead_code)]
    size: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContinueGame {
    pub platform: Platform,
    pub stem: String,
    pub unix: i64,
}

static STATUS: Mutex<String> = Mutex::new(String::new());
static PENDING: Mutex<Option<ContinueGame>> = Mutex::new(None);
static BUSY: AtomicBool = AtomicBool::new(false);
static TRANSFER: Mutex<()> = Mutex::new(());

pub fn status_line() -> String {
    STATUS
        .lock()
        .ok()
        .map(|s| if s.is_empty() { "Off".into() } else { s.clone() })
        .unwrap_or_else(|| "Off".into())
}

fn set_status(line: &str) {
    if let Ok(mut slot) = STATUS.lock() {
        *slot = line.to_string();
    }
}

pub fn take_continue() -> Option<ContinueGame> {
    PENDING.lock().ok().and_then(|mut slot| slot.take())
}

fn offer_continue(game: ContinueGame) {
    if let Ok(mut slot) = PENDING.lock() {
        *slot = Some(game);
    }
}

/// True when the card has a token, which is what turns the feature on.
pub fn configured(root: &Path) -> bool {
    load(root).is_some()
}

pub fn load(root: &Path) -> Option<Config> {
    let text = std::fs::read_to_string(root.join(FILE)).ok()?;
    let mut host = String::new();
    let mut port = DEFAULT_PORT;
    let mut token = String::new();
    let mut device = String::new();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "host" => host = value.trim().to_string(),
            "port" => port = value.trim().parse().unwrap_or(DEFAULT_PORT),
            "token" => token = value.trim().to_string(),
            "device" => device = value.trim().to_string(),
            _ => {}
        }
    }
    if token.is_empty() || !header_ok(&token) || port == 0 {
        return None;
    }
    if device.is_empty() || !header_ok(&device) {
        device = new_device();
        let _ = write_device(root, &text, &device);
    }
    Some(Config { host, port, token, device })
}

fn write_device(root: &Path, text: &str, device: &str) -> std::io::Result<()> {
    let mut lines: Vec<String> = text
        .lines()
        .filter(|line| !line.starts_with("device="))
        .map(str::to_string)
        .collect();
    lines.push(format!("device={device}"));
    let path = root.join(FILE);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    slot_store::atomic_write(&path, lines.join("\n").as_bytes())
}

fn new_device() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    format!("slot-{:08x}", (nanos as u64) ^ (nanos >> 32) as u64)
}

fn header_ok(text: &str) -> bool {
    !text.is_empty() && text.len() <= 64 && text.bytes().all(|b| b.is_ascii_graphic())
}

/// A relative path Slot is willing to send or store. Secrets and `.bak` files are not.
pub fn path_ok(rel: &str) -> bool {
    if rel.is_empty() || rel.starts_with('/') || rel.contains('\\') || rel.contains('\t') || rel.contains('\n') {
        return false;
    }
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.iter().any(|part| part.is_empty() || *part == "." || *part == ".." || part.starts_with('.')) {
        return false;
    }
    if rel.ends_with(".bak") {
        return false;
    }
    if matches!(
        rel,
        "Config/library.txt" | "Config/continue.txt" | "Config/selected_core.ini" | "Config/Achievements/queue.txt"
    ) {
        return true;
    }
    if let Some(name) = rel.strip_prefix("Config/Achievements/sets/") {
        return achievement_name(name, &["json", "list"]);
    }
    if let Some(name) = rel.strip_prefix("Config/Achievements/sessions/") {
        return name
            .strip_suffix(".json")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && !name.contains('/'));
    }
    if let Some(rest) = rel.strip_prefix("Saves/") {
        let mut bits = rest.split('/');
        let Some(system) = bits.next() else { return false };
        let Some(file) = bits.next() else { return false };
        return bits.next().is_none()
            && system_ok(system)
            && stem_ok(file)
            && (file.ends_with(".sav") || file.ends_with(".srm"));
    }
    if let Some(rest) = rel.strip_prefix("States/") {
        let bits: Vec<&str> = rest.split('/').collect();
        if bits.len() != 4 || !system_ok(bits[0]) || !core_ok(bits[1]) || !stem_ok(bits[2]) {
            return false;
        }
        let leaf = bits[3];
        if leaf == "resume.state" {
            return true;
        }
        let Some((stem, ext)) = leaf.rsplit_once('.') else { return false };
        return (ext == "state" || ext == "png") && stamp_ok(stem);
    }
    false
}

fn achievement_name(name: &str, exts: &[&str]) -> bool {
    let Some((stem, ext)) = name.rsplit_once('.') else { return false };
    !name.contains('/')
        && exts.contains(&ext)
        && !stem.is_empty()
        && stem.len() <= 40
        && stem.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn system_ok(name: &str) -> bool {
    matches!(name, "GB" | "GBC" | "GBA" | "NES" | "SNES" | "PS1")
}

fn core_ok(name: &str) -> bool {
    matches!(name, "mgba" | "gpsp" | "fceumm" | "snes9x" | "pcsx_rearmed")
}

fn stem_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 120
        && !name.starts_with('.')
        && !name.contains('/')
        && name.bytes().all(|b| b != b'\t' && b != b'\r' && b != b'\n' && b != b'\\')
}

fn stamp_ok(text: &str) -> bool {
    const SHAPE: &[u8] = b"0000-00-00_00-00-00";
    text.len() == SHAPE.len()
        && text.bytes().zip(SHAPE).all(|(got, want)| match *want {
            b'0' => got.is_ascii_digit(),
            other => got == other,
        })
}

/// Newest mtime wins. Identical bytes stay. Equal times break on the device id.
pub fn decide(inc_mtime: u64, inc_sha: &str, inc_device: &str, cur_mtime: Option<u64>, cur_sha: Option<&str>, cur_device: Option<&str>) -> &'static str {
    if cur_sha == Some(inc_sha) {
        return "same";
    }
    let Some(cur_mtime) = cur_mtime else {
        return "incoming";
    };
    if inc_mtime > cur_mtime {
        return "incoming";
    }
    if inc_mtime < cur_mtime {
        return "current";
    }
    if cur_device == Some(inc_device) {
        return "incoming";
    }
    if inc_device > cur_device.unwrap_or("") {
        "incoming"
    } else {
        "current"
    }
}

pub fn write_continue(root: &Path, platform: Platform, core: Core, stem: &str) {
    if !configured(root) {
        return;
    }
    let device = load(root).map(|cfg| cfg.device).unwrap_or_default();
    let unix = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let text = format!(
        "platform={}\ncore={}\nstem={stem}\nunix={unix}\ndevice={device}\n",
        platform.dir_name(),
        core.as_str()
    );
    let path = root.join("Config/continue.txt");
    let _ = std::fs::create_dir_all(root.join("Config"));
    let _ = slot_store::atomic_write(&path, text.as_bytes());
}

fn parse_continue(text: &str) -> Option<ContinueGame> {
    let mut platform = None;
    let mut stem = None;
    let mut unix = 0i64;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else { continue };
        match key {
            "platform" => platform = platform_of(value),
            "stem" => stem = Some(value.to_string()),
            "unix" => unix = value.parse().unwrap_or(0),
            _ => {}
        }
    }
    Some(ContinueGame { platform: platform?, stem: stem.filter(|s| stem_ok(s))?, unix })
}

fn platform_of(name: &str) -> Option<Platform> {
    Platform::ALL.into_iter().find(|platform| platform.dir_name() == name)
}

/// Start a background pass. It gives up after a few seconds and never logs the token.
pub fn kick(root: PathBuf) {
    if load(&root).is_none() {
        set_status("Off");
        return;
    }
    if BUSY.swap(true, Ordering::AcqRel) {
        return;
    }
    set_status("Looking");
    let _ = std::thread::Builder::new().name("slot-sync".into()).spawn(move || {
        let line = match load(&root) {
            Some(cfg) => match run(&root, &cfg, BACKGROUND_BUDGET) {
                Ok(line) => line,
                Err(line) => line,
            },
            None => "Off".into(),
        };
        set_status(&line);
        BUSY.store(false, Ordering::Release);
    });
}

/// Pull this game's save and resume before the core starts. A quiet hub does not hold the launch.
pub fn pull_before_play(root: &Path, platform: Platform, core: Core, stem: &str) {
    let Some(cfg) = load(root) else { return };
    let Ok(_guard) = TRANSFER.try_lock() else { return };
    let rels = [
        format!("Saves/{}/{stem}.sav", platform.dir_name()),
        format!("Saves/{}/{stem}.srm", platform.dir_name()),
        format!("States/{}/{}/{stem}/resume.state", platform.dir_name(), core.as_str()),
    ];
    let deadline = Instant::now() + LAUNCH_BUDGET;
    if pull_some(root, &cfg, &rels, deadline).is_err() {
        set_status("Hub not found");
    }
}

fn run(root: &Path, cfg: &Config, budget: Duration) -> Result<String, String> {
    let _guard = TRANSFER.lock().map_err(|_| "Hub not found".to_string())?;
    let deadline = Instant::now() + budget;
    let index = get_index(cfg, deadline)?;
    let mut sent = 0u32;
    let mut got = 0u32;
    let local = local_files(root);
    let mut seen = std::collections::HashSet::new();
    for rel in &local {
        if Instant::now() >= deadline {
            break;
        }
        seen.insert(rel.clone());
        let path = root.join(rel);
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let digest = sha256(&bytes);
        let mtime = mtime_nanos(&path);
        match index.get(rel) {
            Some(remote) if remote.sha256 == digest => {}
            Some(remote) if remote.mtime > mtime => {
                if pull_one(root, cfg, rel, deadline)? {
                    got += 1;
                }
            }
            _ => match put_one(cfg, rel, &bytes, mtime, &cfg.device, deadline)? {
                "kept" => {
                    if pull_one(root, cfg, rel, deadline)? {
                        got += 1;
                    }
                }
                "stored" => sent += 1,
                _ => {}
            },
        }
    }
    for rel in index.keys() {
        if seen.contains(rel) || !path_ok(rel) || Instant::now() >= deadline {
            continue;
        }
        if pull_one(root, cfg, rel, deadline)? {
            got += 1;
        }
    }
    Ok(if sent == 0 && got == 0 {
        "Up to date".into()
    } else {
        format!("Sent {sent}, got {got}")
    })
}

fn pull_some(root: &Path, cfg: &Config, rels: &[String], deadline: Instant) -> Result<(), String> {
    for rel in rels {
        if Instant::now() >= deadline {
            break;
        }
        let _ = pull_one(root, cfg, rel, deadline)?;
    }
    Ok(())
}

fn pull_one(root: &Path, cfg: &Config, rel: &str, deadline: Instant) -> Result<bool, String> {
    if !path_ok(rel) {
        return Ok(false);
    }
    let path = format!("/v1/file?path={}", encode(rel));
    let reply = talk(cfg, "GET", &path, &[], &[], deadline)?;
    if reply.status == 404 {
        return Ok(false);
    }
    if reply.status == 401 {
        return Err("Refused".into());
    }
    if reply.status != 200 {
        return Ok(false);
    }
    let dest = root.join(rel);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "Hub not found".to_string())?;
    }
    if dest.is_file() {
        let bak = PathBuf::from(format!("{}.bak", dest.display()));
        let _ = std::fs::remove_file(&bak);
        let _ = std::fs::rename(&dest, bak);
    }
    slot_store::atomic_write(&dest, &reply.body).map_err(|_| "Hub not found".to_string())?;
    if let Some(mtime) = header(&reply, "x-slot-mtime").and_then(|v| v.parse().ok()) {
        set_mtime(&dest, mtime);
    }
    if rel == "Config/continue.txt" {
        if let Some(game) = std::str::from_utf8(&reply.body).ok().and_then(parse_continue) {
            offer_continue(game);
        }
    }
    Ok(true)
}

fn put_one(cfg: &Config, rel: &str, bytes: &[u8], mtime: u64, device: &str, deadline: Instant) -> Result<&'static str, String> {
    let path = format!("/v1/file?path={}", encode(rel));
    let headers = [
        format!("X-Slot-Mtime: {mtime}"),
        format!("X-Slot-Device: {device}"),
    ];
    let reply = talk(cfg, "PUT", &path, &headers, bytes, deadline)?;
    if reply.status == 401 {
        return Err("Refused".into());
    }
    if reply.status != 200 {
        return Err("Hub not found".into());
    }
    let text = std::str::from_utf8(&reply.body).unwrap_or("").trim();
    Ok(match text {
        "stored" => "stored",
        "kept" => "kept",
        _ => "same",
    })
}

fn get_index(cfg: &Config, deadline: Instant) -> Result<HashMap<String, Remote>, String> {
    let reply = talk(cfg, "GET", "/v1/index", &[], &[], deadline)?;
    if reply.status == 401 {
        return Err("Refused".into());
    }
    if reply.status != 200 {
        return Err("Hub not found".into());
    }
    let text = std::str::from_utf8(&reply.body).map_err(|_| "Hub not found".to_string())?;
    Ok(parse_index(text))
}

fn parse_index(text: &str) -> HashMap<String, Remote> {
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut bits = line.split('\t');
        let Some(path) = bits.next() else { continue };
        let Ok(mtime) = bits.next().unwrap_or("").parse::<u64>() else { continue };
        let Some(device) = bits.next() else { continue };
        let Some(sha256) = bits.next() else { continue };
        let Ok(size) = bits.next().unwrap_or("").parse::<u64>() else { continue };
        if path_ok(path) {
            out.insert(path.to_string(), Remote { mtime, device: device.to_string(), sha256: sha256.to_string(), size });
        }
    }
    out
}

fn local_files(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for rel in ["Config/library.txt", "Config/continue.txt", "Config/selected_core.ini", "Config/Achievements/queue.txt"] {
        if root.join(rel).is_file() {
            out.push(rel.to_string());
        }
    }
    push_dir(&mut out, &root.join("Config/Achievements/sets"), "Config/Achievements/sets");
    push_dir(&mut out, &root.join("Config/Achievements/sessions"), "Config/Achievements/sessions");
    if let Ok(systems) = std::fs::read_dir(root.join("Saves")) {
        for system in systems.flatten() {
            let Ok(files) = std::fs::read_dir(system.path()) else { continue };
            for file in files.flatten() {
                let rel = format!("Saves/{}/{}", system.file_name().to_string_lossy(), file.file_name().to_string_lossy());
                if path_ok(&rel) {
                    out.push(rel);
                }
            }
        }
    }
    if let Ok(systems) = std::fs::read_dir(root.join("States")) {
        for system in systems.flatten() {
            let Ok(cores) = std::fs::read_dir(system.path()) else { continue };
            for core in cores.flatten() {
                let Ok(games) = std::fs::read_dir(core.path()) else { continue };
                for game in games.flatten() {
                    push_states(&mut out, &game.path(), &system.file_name().to_string_lossy(), &core.file_name().to_string_lossy(), &game.file_name().to_string_lossy());
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn push_dir(out: &mut Vec<String>, dir: &Path, prefix: &str) {
    let Ok(files) = std::fs::read_dir(dir) else { return };
    for file in files.flatten() {
        let rel = format!("{prefix}/{}", file.file_name().to_string_lossy());
        if path_ok(&rel) {
            out.push(rel);
        }
    }
}

fn push_states(out: &mut Vec<String>, dir: &Path, system: &str, core: &str, game: &str) {
    let resume = format!("States/{system}/{core}/{game}/resume.state");
    if dir.join("resume.state").is_file() && path_ok(&resume) {
        out.push(resume);
    }
    let mut newest: Option<String> = None;
    let Ok(files) = std::fs::read_dir(dir) else { return };
    for file in files.flatten() {
        let name = file.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".state") else { continue };
        if stamp_ok(stem) && newest.as_ref().is_none_or(|have| stem > have.as_str()) {
            newest = Some(stem.to_string());
        }
    }
    if let Some(stamp) = newest {
        for ext in ["state", "png"] {
            let rel = format!("States/{system}/{core}/{game}/{stamp}.{ext}");
            if dir.join(format!("{stamp}.{ext}")).is_file() && path_ok(&rel) {
                out.push(rel);
            }
        }
    }
}

struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn header<'a>(reply: &'a Reply, name: &str) -> Option<&'a str> {
    reply.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
}

fn talk(cfg: &Config, method: &str, path: &str, extra: &[String], body: &[u8], deadline: Instant) -> Result<Reply, String> {
    if Instant::now() >= deadline {
        return Err("Hub not found".into());
    }
    let (host, port) = endpoint(cfg, deadline)?;
    let remain = deadline.saturating_duration_since(Instant::now()).min(CONNECT);
    let mut stream = connect(&host, port, remain)?;
    let _ = stream.set_read_timeout(Some(remain.max(Duration::from_millis(50))));
    let _ = stream.set_write_timeout(Some(remain.max(Duration::from_millis(50))));
    let mut text = format!(
        "{method} {path} HTTP/1.0\r\nHost: {host}\r\nUser-Agent: SlotSync\r\nAuthorization: Bearer {}\r\nConnection: close\r\n",
        cfg.token
    );
    for header in extra {
        text.push_str(header);
        text.push_str("\r\n");
    }
    if method == "PUT" {
        text.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    text.push_str("\r\n");
    stream.write_all(text.as_bytes()).map_err(|_| "Hub not found".to_string())?;
    if !body.is_empty() {
        stream.write_all(body).map_err(|_| "Hub not found".to_string())?;
    }
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        if Instant::now() >= deadline {
            break;
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => raw.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(_) => break,
        }
        if raw.len() > 32 * 1024 * 1024 {
            break;
        }
    }
    parse_http(&raw)
}

fn endpoint(cfg: &Config, deadline: Instant) -> Result<(String, u16), String> {
    if !cfg.host.is_empty() {
        return Ok((cfg.host.clone(), cfg.port));
    }
    discover(deadline).ok_or_else(|| "Hub not found".into())
}

fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let addrs = (host, port).to_socket_addrs().map_err(|_| "Hub not found".to_string())?;
    for addr in addrs {
        if let Ok(stream) = TcpStream::connect_timeout(&addr, timeout) {
            return Ok(stream);
        }
    }
    Err("Hub not found".into())
}

fn parse_http(raw: &[u8]) -> Result<Reply, String> {
    let split = raw
        .windows(4)
        .position(|mark| mark == b"\r\n\r\n")
        .ok_or_else(|| "Hub not found".to_string())?;
    let head = std::str::from_utf8(&raw[..split]).map_err(|_| "Hub not found".to_string())?;
    let mut lines = head.lines();
    let status = lines
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "Hub not found".to_string())?;
    let headers = lines
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();
    Ok(Reply { status, headers, body: raw[split + 4..].to_vec() })
}

fn encode(rel: &str) -> String {
    let mut out = String::new();
    for byte in rel.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'/' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn sha256(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn mtime_nanos(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn set_mtime(path: &Path, nanos: u64) {
    let Ok(file) = std::fs::OpenOptions::new().write(true).open(path) else { return };
    let time = UNIX_EPOCH + Duration::from_nanos(nanos);
    let _ = file.set_modified(time);
}

fn discover(deadline: Instant) -> Option<(String, u16)> {
    let sock = UdpSocket::bind("0.0.0.0:0").ok()?;
    let remain = deadline.saturating_duration_since(Instant::now()).min(Duration::from_millis(300));
    sock.set_read_timeout(Some(remain)).ok()?;
    sock.send_to(&mdns_query(), "224.0.0.251:5353").ok()?;
    let mut buf = [0u8; 1500];
    let (n, _) = sock.recv_from(&mut buf).ok()?;
    parse_mdns(&buf[..n])
}

fn mdns_query() -> Vec<u8> {
    let mut packet = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["_slotsync", "_tcp", "local"] {
        packet.push(label.len() as u8);
        packet.extend(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&[0, 12, 0, 1]);
    packet
}

fn parse_mdns(packet: &[u8]) -> Option<(String, u16)> {
    if packet.len() < 12 {
        return None;
    }
    let questions = u16::from_be_bytes([packet[4], packet[5]]) as usize;
    let answers = u16::from_be_bytes([packet[6], packet[7]]) as usize;
    let mut i = 12;
    for _ in 0..questions {
        let (_, next) = read_name(packet, i)?;
        i = next + 4;
    }
    let mut port = None;
    let mut ip = None;
    for _ in 0..answers {
        let (_, next) = read_name(packet, i)?;
        if next + 10 > packet.len() {
            break;
        }
        let kind = u16::from_be_bytes([packet[next], packet[next + 1]]);
        let len = u16::from_be_bytes([packet[next + 8], packet[next + 9]]) as usize;
        let data = next + 10;
        if data + len > packet.len() {
            break;
        }
        if kind == 33 && len >= 6 {
            port = Some(u16::from_be_bytes([packet[data + 4], packet[data + 5]]));
        }
        if kind == 1 && len == 4 {
            ip = Some(format!("{}.{}.{}.{}", packet[data], packet[data + 1], packet[data + 2], packet[data + 3]));
        }
        i = data + len;
    }
    Some((ip?, port.unwrap_or(DEFAULT_PORT)))
}

fn read_name(packet: &[u8], mut i: usize) -> Option<(String, usize)> {
    let mut out = String::new();
    let mut hops = 0;
    loop {
        if i >= packet.len() || hops > 12 {
            return None;
        }
        let len = packet[i] as usize;
        if len == 0 {
            return Some((out, i + 1));
        }
        if len & 0xC0 == 0xC0 {
            if i + 1 >= packet.len() {
                return None;
            }
            let ptr = ((len & 0x3F) << 8) | packet[i + 1] as usize;
            let (rest, _) = read_name(packet, ptr)?;
            if !out.is_empty() && !rest.is_empty() {
                out.push('.');
            }
            out.push_str(&rest);
            return Some((out, i + 2));
        }
        i += 1;
        if i + len > packet.len() {
            return None;
        }
        if !out.is_empty() {
            out.push('.');
        }
        out.push_str(std::str::from_utf8(&packet[i..i + len]).ok()?);
        i += len;
        hops += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_conflicts() {
        assert!(path_ok("Saves/GBA/Emerald.sav"));
        assert!(path_ok("Saves/GBA/Pixel Racer.srm"));
        assert!(path_ok("States/GBA/mgba/Emerald/resume.state"));
        assert!(path_ok("States/NES/fceumm/Duck/2026-11-24_01-02-03.state"));
        assert!(path_ok("Config/library.txt"));
        assert!(path_ok("Config/Achievements/sets/ab12.json"));
        assert!(path_ok("Config/Achievements/sessions/42.json"));
        assert!(!path_ok("../Saves/GBA/Emerald.sav"));
        assert!(!path_ok("Config/sync.txt"));
        assert!(!path_ok("Config/Achievements/login.json"));
        assert!(!path_ok("Saves/GBA/Emerald.sav.bak"));
        assert_eq!(decide(20, "aa", "slot-b", Some(10), Some("bb"), Some("slot-a")), "incoming");
        assert_eq!(decide(10, "aa", "slot-b", Some(20), Some("bb"), Some("slot-a")), "current");
        assert_eq!(decide(10, "aa", "slot-b", Some(10), Some("aa"), Some("slot-a")), "same");
        assert_eq!(decide(10, "aa", "slot-b", Some(10), Some("bb"), Some("slot-a")), "incoming");
        assert_eq!(decide(10, "aa", "slot-a", Some(10), Some("bb"), Some("slot-b")), "current");
    }

    #[test]
    fn a_missing_token_stays_off_and_the_device_id_is_filled_in() {
        let root = tempfile::tempdir().unwrap();
        assert!(!configured(root.path()));
        std::fs::create_dir_all(root.path().join("Config")).unwrap();
        std::fs::write(root.path().join(FILE), "host=127.0.0.1\nport=8791\ntoken=abc123\n").unwrap();
        let cfg = load(root.path()).unwrap();
        assert_eq!(cfg.host, "127.0.0.1");
        assert!(cfg.device.starts_with("slot-"));
        let text = std::fs::read_to_string(root.path().join(FILE)).unwrap();
        assert!(text.contains("token=abc123"));
        assert!(text.contains(&format!("device={}", cfg.device)));
        assert!(!text.contains("abc123\ndevice=abc123"));
    }

    #[test]
    fn the_index_and_continue_file_parse() {
        let index = parse_index("Saves/GBA/Emerald.sav\t2000\tslot-b\tabc\t8\nConfig/sync.txt\t1\tslot-a\tnope\t1\n");
        assert_eq!(index.len(), 1);
        assert_eq!(index["Saves/GBA/Emerald.sav"].mtime, 2000);
        let game = parse_continue("platform=GBA\ncore=mgba\nstem=Emerald\nunix=50\ndevice=slot-a\n").unwrap();
        assert_eq!(game.platform, Platform::Gba);
        assert_eq!(game.stem, "Emerald");
        assert_eq!(game.unix, 50);
    }

    #[test]
    fn an_mdns_reply_yields_the_address_and_port() {
        let mut packet = vec![0, 0, 0x84, 0, 0, 0, 0, 2, 0, 0, 0, 0];
        for _ in 0..2 {
            packet.push(8);
            packet.extend_from_slice(b"slotsync");
            packet.push(0);
            if packet.len() < 30 {
                packet.extend_from_slice(&[0, 33, 0, 1, 0, 0, 0, 120, 0, 6, 0, 0, 0, 0, 0x22, 0x57]);
            } else {
                packet.extend_from_slice(&[0, 1, 0, 1, 0, 0, 0, 120, 0, 4, 192, 168, 7, 99]);
            }
        }
        let (ip, port) = parse_mdns(&packet).unwrap();
        assert_eq!(ip, "192.168.7.99");
        assert_eq!(port, 8791);
    }

    #[test]
    fn the_hub_keeps_the_newer_save_and_a_bak() {
        let script = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tools/sync-hub/hub.py");
        if !script.is_file() || std::process::Command::new("python3").arg("--version").output().is_err() {
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new("python3")
            .arg(&script)
            .arg("--root")
            .arg(root.path())
            .arg("--port")
            .arg("0")
            .arg("--bind")
            .arg("127.0.0.1")
            .arg("--token")
            .arg("secret-token")
            .arg("--no-mdns")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(child.stdout.take().unwrap()), &mut line).unwrap();
        let port: u16 = line.trim().trim_start_matches("port=").parse().unwrap();
        let card = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(card.path().join("Config")).unwrap();
        std::fs::write(
            card.path().join(FILE),
            format!("host=127.0.0.1\nport={port}\ntoken=secret-token\ndevice=slot-card\n"),
        )
        .unwrap();
        std::fs::create_dir_all(card.path().join("Saves/GBA")).unwrap();
        let sav = card.path().join("Saves/GBA/Emerald.sav");
        std::fs::write(&sav, b"from-card").unwrap();
        set_mtime(&sav, 5_000);
        let cfg = load(card.path()).unwrap();
        let line = run(card.path(), &cfg, Duration::from_secs(3)).unwrap();
        assert_eq!(line, "Sent 1, got 0");
        let cfg_b = Config { host: "127.0.0.1".into(), port, token: "secret-token".into(), device: "slot-other".into() };
        let reply = talk(
            &cfg_b,
            "PUT",
            "/v1/file?path=Saves/GBA/Emerald.sav",
            &["X-Slot-Mtime: 9000".into(), "X-Slot-Device: slot-other".into()],
            b"from-other",
            Instant::now() + Duration::from_secs(2),
        )
        .unwrap();
        assert_eq!(reply.status, 200);
        assert_eq!(std::str::from_utf8(&reply.body).unwrap().trim(), "stored");
        let line = run(card.path(), &cfg, Duration::from_secs(3)).unwrap();
        assert_eq!(line, "Sent 0, got 1");
        assert_eq!(std::fs::read(&sav).unwrap(), b"from-other");
        assert_eq!(std::fs::read(format!("{}.bak", sav.display())).unwrap(), b"from-card");
        let _ = child.kill();
        let _ = child.wait();
    }
}
