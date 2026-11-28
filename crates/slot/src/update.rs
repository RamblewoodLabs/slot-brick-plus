//! Updating Slot from this fork's GitHub releases only.
//!
//! HOME → Online → Update asks `github.com/jfink22-design/slot-brick-plus` for its latest
//! non-draft release, downloads the Brick `slot` binary and its `slot.sha256`, and replaces
//! `Slot/System/slot` only after the checksum matches. The previous binary is kept as
//! `slot.bak`. launch.sh puts it back if the new one never shows a frame. Nothing is fetched
//! from any other project, and the player's games, saves and settings are never touched.
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The only repository Update talks to.

struct Sha256 {
    #[cfg(feature = "network")]
    inner: ring::digest::Context,
}

impl Sha256 {
    fn new() -> Self {
        Sha256 {
            #[cfg(feature = "network")]
            inner: ring::digest::Context::new(&ring::digest::SHA256),
        }
    }
    fn update(&mut self, bytes: &[u8]) {
        #[cfg(feature = "network")]
        self.inner.update(bytes);
        #[cfg(not(feature = "network"))]
        let _ = bytes;
    }
    fn finish(self) -> [u8; 32] {
        #[cfg(feature = "network")]
        {
            let mut out = [0u8; 32];
            out.copy_from_slice(self.inner.finish().as_ref());
            out
        }
        #[cfg(not(feature = "network"))]
        [0u8; 32]
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finish().iter().map(|b| format!("{b:02x}")).collect()
}

pub const REPO: &str = "jfink22-design/slot-brick-plus";
/// The Releases API, newest first. Drafts are not in this list; a draft flag is skipped anyway.
pub const RELEASES_API: &str =
    "https://api.github.com/repos/jfink22-design/slot-brick-plus/releases?per_page=20";

/// The two files a release must carry. `slot` is the Brick binary; `slot.sha256` is `sha256sum`'s line.
pub const BINARY_ASSET: &str = "slot";
pub const CHECKSUM_ASSET: &str = "slot.sha256";

/// Beside Slot/ on the card: the download, the files unpacked from it, and those they replaced.
pub const DIR: &str = ".slot-update";
/// Every file is unpacked: the swap may start (or finish, after a power cut).
const STAGED: &str = "staged";
/// The swap has started and the new Slot has not shown a frame yet. launch.sh, finding this
/// after Slot ended, puts the old files back.
const PENDING: &str = "pending";

/// The exit status that asks launch.sh to start Slot again, on the files just swapped in.
pub const EXIT_UPDATED: i32 = 66;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Channel {
    Release,
    Dev,
}

/// One release this fork published: its tag, the title and notes GitHub shows, and the
/// Brick binary's address and checksum.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Manifest {
    pub version: String,
    pub title: String,
    pub size: u64,
    pub sha256: [u8; 32],
    pub binary_url: String,
    pub notes: Vec<String>,
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(s.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// `v0.43` as [0, 43].
fn version_of(v: &str) -> Option<Vec<u32>> {
    let v = v.strip_prefix(['v', 'V']).unwrap_or(v);
    v.split('.').map(|p| p.parse().ok()).collect()
}

/// What is installed: its version, the commit it was built from, and whether it is a Dev build.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Installed {
    pub version: String,
    pub commit: String,
    pub channel: Channel,
}

impl Installed {
    pub fn current() -> Installed {
        // The simulator's scenarios name a version of their own, so their screenshots don't
        // change with every release.
        #[cfg(feature = "sim")]
        let version = std::env::var("SLOT_SIM_VERSION")
            .unwrap_or_else(|_| slot_ui::PORT_VERSION.to_lowercase());
        #[cfg(not(feature = "sim"))]
        let version = slot_ui::PORT_VERSION.to_lowercase();
        Installed {
            version,
            commit: crate::build_info::Build::current().hash.to_string(),
            channel: match option_env!("SLOT_CHANNEL") {
                Some("dev") => Channel::Dev,
                _ => Channel::Release,
            },
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Offer {
    UpToDate,
    /// Newer than what is installed.
    Newer,
}

pub fn offer(installed: &Installed, m: &Manifest) -> Offer {
    match (version_of(&m.version), version_of(&installed.version)) {
        (Some(new), Some(old)) if new > old => Offer::Newer,
        _ => Offer::UpToDate,
    }
}

/// Where Update looks. A device build always uses this fork's Releases API. A simulator or a
/// test may point `SLOT_UPDATE_URL` at a `file://` folder; anything else is ignored, so the
/// Brick cannot be sent to another project.
pub fn source() -> String {
    #[cfg(any(test, feature = "sim"))]
    if let Ok(url) = std::env::var("SLOT_UPDATE_URL") {
        if url.starts_with("file://") {
            return url;
        }
    }
    RELEASES_API.to_string()
}

fn allowed_url(url: &str) -> bool {
    #[cfg(any(test, feature = "sim"))]
    if url.starts_with("file://") && !url.contains("..") {
        return true;
    }
    let api = format!("https://api.github.com/repos/{REPO}/");
    let download = format!("https://github.com/{REPO}/releases/download/");
    if (url.starts_with(&api) || url.starts_with(&download)) && !url.contains("..") {
        return true;
    }
    // GitHub sends a release file on to its own asset host. That hop is allowed; a release
    // that names any other address is not.
    const CDN: [&str; 3] = [
        "https://release-assets.githubusercontent.com/",
        "https://objects.githubusercontent.com/",
        "https://github-releases.githubusercontent.com/",
    ];
    CDN.iter().any(|prefix| url.starts_with(prefix))
}

fn asset_url(tag: &str, name: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{tag}/{name}")
}

fn asset_ok(tag: &str, name: &str, url: &str) -> bool {
    if url == asset_url(tag, name) {
        return true;
    }
    #[cfg(any(test, feature = "sim"))]
    if url.starts_with("file://") && url.ends_with(&format!("/{name}")) && !url.contains("..") {
        return true;
    }
    false
}

/// A download, as it comes. A simulator build also reads `file://` folders, standing in for
/// GitHub in its scenarios.
fn open(url: &str) -> Result<crate::net::Body, String> {
    if !allowed_url(url) {
        return Err("Refusing a download from anywhere but this Slot's GitHub releases".into());
    }
    #[cfg(any(test, feature = "sim"))]
    if let Some(path) = url.strip_prefix("file://") {
        let file = fs::File::open(path).map_err(|_| "server said 404".to_string())?;
        let len = file.metadata().ok().map(|m| m.len());
        return Ok(crate::net::Body {
            len,
            reader: Box::new(file),
        });
    }
    crate::net::get_stream_allowed(url, &allowed_url)
}

fn fetch(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let body = open(url)?;
    let mut out = Vec::new();
    body.reader
        .take(limit as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| format!("download failed: {e}"))?;
    if out.len() > limit {
        return Err("reply too large".into());
    }
    Ok(out)
}

/// The latest non-draft release that carries `slot` and `slot.sha256`. `base` is the Releases
/// API, or a `file://` folder in a test. Offline, and any other address, come back as an error.
pub fn check(base: &str) -> Result<Manifest, String> {
    if !base_ok(base) {
        return Err("Refusing a download from anywhere but this Slot's GitHub releases".into());
    }
    let list_url = match base.starts_with("file://") {
        true => format!("{}/releases.json", base.trim_end_matches('/')),
        false => base.to_string(),
    };
    let text = fetch(&list_url, 2 * 1024 * 1024).map_err(explain)?;
    let text = std::str::from_utf8(&text)
        .map_err(|_| "The update's description is damaged".to_string())?;
    let listed = parse_releases(text)?;
    let sum = fetch(&listed.checksum_url, 4 * 1024).map_err(explain)?;
    let sum = std::str::from_utf8(&sum).map_err(|_| "The checksum is damaged".to_string())?;
    let sha256 = parse_sha256(sum).ok_or_else(|| "The checksum is damaged".to_string())?;
    Ok(Manifest {
        version: listed.version,
        title: listed.title,
        size: listed.size,
        sha256,
        binary_url: listed.binary_url,
        notes: listed.notes,
    })
}

fn base_ok(base: &str) -> bool {
    if base == RELEASES_API {
        return true;
    }
    #[cfg(any(test, feature = "sim"))]
    if base.starts_with("file://") && !base.contains("..") {
        return true;
    }
    false
}

fn explain(error: String) -> String {
    let low = error.to_ascii_lowercase();
    if low.contains("no internet")
        || low.contains("cannot connect")
        || low.contains("cannot look up")
        || low.contains("timed out")
        || low.contains("tls")
        || low.contains("send failed")
        || low.contains("download failed")
        || low.contains("connection")
    {
        "No Wi-Fi, or GitHub could not be reached. Try again when you are online.".into()
    } else if error.contains("404") {
        "No update has been published here yet".into()
    } else {
        error
    }
}

#[derive(Debug)]
struct Listed {
    version: String,
    title: String,
    size: u64,
    binary_url: String,
    checksum_url: String,
    notes: Vec<String>,
}

/// The first release in GitHub's list (newest first) that is not a draft, has a version tag,
/// and points both files at this repository.
fn parse_releases(json: &str) -> Result<Listed, String> {
    let none = || "No update has been published here yet".to_string();
    for object in json_objects(json) {
        if json_bool(object, "draft") == Some(true) {
            continue;
        }
        let Some(version) = json_string(object, "tag_name") else {
            continue;
        };
        if !tag_ok(&version) {
            continue;
        }
        let title = json_string(object, "name").unwrap_or_else(|| version.clone());
        let notes = json_string(object, "body")
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect();
        let Some(assets) = json_array(object, "assets") else {
            continue;
        };
        let mut binary = None;
        let mut checksum = None;
        let mut size = None;
        for asset in json_objects(assets) {
            let Some(name) = json_string(asset, "name") else {
                continue;
            };
            let Some(url) = json_string(asset, "browser_download_url") else {
                continue;
            };
            if !asset_ok(&version, &name, &url) {
                continue;
            }
            if name == BINARY_ASSET {
                binary = Some(url);
                size = json_u64(asset, "size");
            } else if name == CHECKSUM_ASSET {
                checksum = Some(url);
            }
        }
        if let (Some(binary_url), Some(checksum_url), Some(size)) = (binary, checksum, size) {
            if size == 0 || size > 64 * 1024 * 1024 {
                continue;
            }
            return Ok(Listed {
                version,
                title,
                size,
                binary_url,
                checksum_url,
                notes,
            });
        }
    }
    Err(none())
}

fn tag_ok(tag: &str) -> bool {
    version_of(tag).is_some()
        && tag
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
}

fn parse_sha256(text: &str) -> Option<[u8; 32]> {
    unhex(text.split_whitespace().next()?)
}

/// How an install is getting on, shared with the screen showing it.
#[derive(Default)]
pub struct Progress {
    pub downloaded: AtomicU64,
    /// Downloaded and checked: unpacking and swapping, which cannot be cancelled.
    pub unpacking: AtomicBool,
    pub cancel: AtomicBool,
}

/// Download `m`, check it against its checksum, and replace `Slot/System/slot`. `root` is the
/// Slot directory. The previous binary is copied to `slot.bak` first and a pending marker is
/// written, so launch.sh can put it back if the new one never starts. A bad checksum leaves
/// the installed binary where it was.
pub fn install(m: &Manifest, root: &Path, progress: &Progress) -> Result<(), String> {
    if !allowed_url(&m.binary_url) {
        return Err("Refusing a download from anywhere but this Slot's GitHub releases".into());
    }
    let system = root.join("System");
    let dest = system.join("slot");
    let incoming = system.join("slot.new");
    let backup = system.join("slot.bak");
    let pending = system.join("slot.pending");
    let _ = fs::remove_file(&incoming);
    let need = m.size.saturating_mul(2) + (8 << 20);
    if let Some(free) = slot_store::free_space(root) {
        if free < need {
            return Err(format!(
                "Needs {} MB free on the SD card",
                need.div_ceil(1 << 20)
            ));
        }
    }
    fs::create_dir_all(&system).map_err(|e| format!("Cannot write to the SD card: {e}"))?;
    if let Err(e) = download_binary(m, &incoming, progress) {
        let _ = fs::remove_file(&incoming);
        return Err(e);
    }
    progress.unpacking.store(true, Ordering::Relaxed);
    if dest.is_file() {
        fs::copy(&dest, &backup).map_err(|e| format!("Cannot keep the previous Slot: {e}"))?;
    }
    fs::write(&pending, format!("{}\n", m.version))
        .map_err(|e| format!("Cannot write to the SD card: {e}"))?;
    fs::rename(&incoming, &dest).map_err(|e| format!("Cannot swap in the update: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    }
    let _ = std::process::Command::new("sync").status();
    Ok(())
}

fn download_binary(m: &Manifest, path: &Path, progress: &Progress) -> Result<(), String> {
    let body = open(&m.binary_url).map_err(explain)?;
    if body.len.is_some_and(|n| n != m.size) {
        return Err("The download is not the size it should be".into());
    }
    let mut file =
        fs::File::create(path).map_err(|e| format!("Cannot write to the SD card: {e}"))?;
    let mut reader = body.reader.take(m.size + 1);
    let mut hash = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut got = 0u64;
    loop {
        if progress.cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && got == m.size => break,
            Err(e) => return Err(format!("Download failed: {e}")),
        };
        if got + n as u64 > m.size {
            return Err("The download is larger than it should be".into());
        }
        hash.update(&buf[..n]);
        file.write_all(&buf[..n])
            .map_err(|e| format!("Cannot write to the SD card: {e}"))?;
        got += n as u64;
        progress.downloaded.store(got, Ordering::Relaxed);
    }
    if got != m.size {
        return Err("The download was cut short. Try again".into());
    }
    if hash.finish().as_ref() != &m.sha256[..] {
        return Err("The download is damaged (its checksum does not match). Try again".into());
    }
    file.sync_all()
        .map_err(|e| format!("Cannot write to the SD card: {e}"))?;
    Ok(())
}

/// Slot's own files on the card, which an update replaces. Everything else is the player's:
/// games, saves, states, labels, BIOS, settings, wallpapers, and the boot picture they chose.
/// Slot/AUTOBOOT says whether Slot starts by itself, so it is left as it is; trimui/ is only
/// for installing from a computer. On a Slot OS card, slot-os/ (its libraries, drivers and a
/// new boot.img, which init installs) and the boot logo are Slot's too; stock zips have none.
pub fn ours(rel: &str) -> bool {
    const OURS: [&str; 8] = [
        ".slot-boot/",
        "Apps/Slot/",
        "Apps/SlotBootRestore/",
        "Apps/StartupCheck/",
        "Apps/BootLogo/",
        "Slot/System/",
        "Slot/Overlays/",
        "slot-os/",
    ];
    if rel.starts_with("Apps/BootLogo/Image/") {
        return rel == "Apps/BootLogo/Image/README.txt";
    }
    rel == "Slot/README.md" || rel == "bootlogo.bmp" || OURS.iter().any(|p| rel.starts_with(p))
}

/// A path inside the zip's SD-CARD/ that stays inside it.
fn safe(rel: &str) -> bool {
    !rel.is_empty()
        && !rel.contains('\\')
        && rel
            .split('/')
            .all(|c| !c.is_empty() && c != "." && c != "..")
}

/// Unpacks Slot's own files from the card zip (zip-card.py's: everything under SD-CARD/,
/// stored or deflated) into `into`.
pub fn stage(zip: &Path, into: &Path) -> std::io::Result<usize> {
    use slot_store::archive::zip as cardzip;
    let bad = |what: &str| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("damaged zip ({what})"),
        )
    };
    let mut file = fs::File::open(zip)?;
    let mut written = 0;
    for entry in cardzip::index(&mut file)? {
        let Some(rel) = entry.name.strip_prefix("SD-CARD/") else {
            continue;
        };
        if rel.is_empty() || rel.ends_with('/') || !ours(rel) {
            continue;
        }
        if !safe(rel) {
            return Err(bad("name"));
        }
        let mut bytes = Vec::with_capacity(entry.size as usize);
        cardzip::reader(&mut file, &entry)?.read_to_end(&mut bytes)?;
        if bytes.len() as u64 != entry.size {
            return Err(bad("size"));
        }
        let path = into.join(rel);
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut out = fs::File::create(&path)?;
        out.write_all(&bytes)?;
        out.sync_all()?;
        if entry.made_by == 3 {
            use std::os::unix::fs::PermissionsExt;
            let mode = (entry.attrs >> 16) & 0o777;
            if mode != 0 {
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(mode));
            }
        }
        written += 1;
    }
    if written == 0 || !into.join("Slot/System/slot").exists() {
        return Err(bad("no Slot in it"));
    }
    Ok(written)
}

/// Every file under `dir`, as paths relative to it.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(rel) = todo.pop() {
        let Ok(entries) = fs::read_dir(dir.join(&rel)) else {
            continue;
        };
        for entry in entries.flatten() {
            let rel = rel.join(entry.file_name());
            match entry.file_type() {
                Ok(t) if t.is_dir() => todo.push(rel),
                Ok(_) => out.push(rel),
                Err(_) => {}
            }
        }
    }
    out.sort();
    out
}

/// Swaps the unpacked files in, each old one moved aside to old/ first. Run again after a
/// power cut, it carries on where it stopped.
pub fn apply(card: &Path) -> std::io::Result<()> {
    let dir = card.join(DIR);
    let (new, old) = (dir.join("new"), dir.join("old"));
    fs::write(dir.join(PENDING), "")?;
    for rel in files_under(&new) {
        let (dest, keep) = (card.join(&rel), old.join(&rel));
        if dest.exists() && !keep.exists() {
            fs::create_dir_all(keep.parent().unwrap_or(&old))?;
            fs::rename(&dest, &keep)?;
        }
        fs::create_dir_all(dest.parent().unwrap_or(card))?;
        fs::rename(new.join(&rel), &dest)?;
    }
    fs::remove_dir_all(&new)?;
    fs::remove_file(dir.join(STAGED))?;
    let _ = std::process::Command::new("sync").status();
    Ok(())
}

/// At startup: an update unpacked but not (or not all) swapped in, as a power cut leaves it,
/// is finished. True when it was, and Slot should start again on the new files.
pub fn resume(card: &Path) -> bool {
    if !card.join(DIR).join(STAGED).exists() {
        return false;
    }
    match apply(card) {
        Ok(()) => {
            eprintln!("slot: update: finished swapping in an update that was cut short");
            true
        }
        Err(e) => {
            eprintln!("slot: update: cannot finish swapping in the update: {e}");
            false
        }
    }
}

/// What a download or unpacking cut short (by a power cut, say) leaves behind is deleted. A
/// swap under way is left alone: resume finishes it, and launch.sh may need its old files.
pub fn tidy(card: &Path) {
    let dir = card.join(DIR);
    if dir.exists() && !dir.join(STAGED).exists() && !dir.join(PENDING).exists() {
        match fs::remove_dir_all(&dir) {
            Ok(()) => eprintln!("slot: update: cleared away an update that never finished"),
            Err(e) => eprintln!("slot: update: cannot clear {}: {e}", dir.display()),
        }
    }
    let pending = card.join("Slot/System/slot.pending");
    let incoming = card.join("Slot/System/slot.new");
    if incoming.is_file() && !pending.is_file() {
        let _ = fs::remove_file(incoming);
    }
}

/// The new Slot is up: the old files are no longer needed, and a later crash must not roll
/// the binary back. `slot.bak` stays, so the previous one is still on the card.
pub fn settle(card: &Path) {
    let dir = card.join(DIR);
    if dir.join(PENDING).exists() && !dir.join(STAGED).exists() {
        match fs::remove_dir_all(&dir) {
            Ok(()) => eprintln!("slot: update: installed"),
            Err(e) => eprintln!("slot: update: cannot tidy {}: {e}", dir.display()),
        }
    }
    let pending = card.join("Slot/System/slot.pending");
    if pending.is_file() {
        match fs::remove_file(&pending) {
            Ok(()) => eprintln!("slot: update: installed"),
            Err(e) => eprintln!("slot: update: cannot clear {}: {e}", pending.display()),
        }
    }
}

/// The card's root: Slot/'s parent.
pub fn card_of(root: &Path) -> PathBuf {
    root.parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("/mnt/SDCARD"))
}

fn json_objects(json: &str) -> Vec<&str> {
    let bytes = json.as_bytes();
    let mut i = json.find('[').map(|at| at + 1).unwrap_or(0);
    let mut out = Vec::new();
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b']' || bytes[i] != b'{' {
            break;
        }
        let start = i;
        let mut depth = 0i32;
        let mut in_str = false;
        let mut esc = false;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if c == b'\\' {
                    esc = true;
                } else if c == b'"' {
                    in_str = false;
                }
            } else if c == b'"' {
                in_str = true;
            } else if c == b'{' {
                depth += 1;
            } else if c == b'}' {
                depth -= 1;
                if depth == 0 {
                    i += 1;
                    out.push(&json[start..i]);
                    break;
                }
            }
            i += 1;
        }
    }
    out
}

/// A top-level string, bool or number of one JSON object. Nested objects are skipped.
fn json_string(object: &str, key: &str) -> Option<String> {
    let raw = json_value(object, key)?;
    let raw = raw.trim();
    let raw = raw.strip_prefix('"')?.strip_suffix('"')?;
    Some(json_unescape(raw))
}

fn json_bool(object: &str, key: &str) -> Option<bool> {
    match json_value(object, key)?.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn json_u64(object: &str, key: &str) -> Option<u64> {
    json_value(object, key)?.trim().parse().ok()
}

fn json_array<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    let raw = json_value(object, key)?.trim();
    raw.starts_with('[').then_some(raw)
}

fn json_value<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    let bytes = object.as_bytes();
    let mut i = 0;
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    while i < bytes.len() {
        let c = bytes[i];
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => {
                let start = i + 1;
                i += 1;
                let mut s_esc = false;
                let mut end = None;
                while i < bytes.len() {
                    if s_esc {
                        s_esc = false;
                    } else if bytes[i] == b'\\' {
                        s_esc = true;
                    } else if bytes[i] == b'"' {
                        end = Some(i);
                        break;
                    }
                    i += 1;
                }
                let end = end?;
                let found = &object[start..end];
                i = end + 1;
                if depth == 1 && found == key {
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    if i >= bytes.len() || bytes[i] != b':' {
                        continue;
                    }
                    i += 1;
                    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    let value_at = i;
                    if i < bytes.len() && bytes[i] == b'"' {
                        i += 1;
                        let mut v_esc = false;
                        while i < bytes.len() {
                            if v_esc {
                                v_esc = false;
                            } else if bytes[i] == b'\\' {
                                v_esc = true;
                            } else if bytes[i] == b'"' {
                                i += 1;
                                return Some(&object[value_at..i]);
                            }
                            i += 1;
                        }
                        return None;
                    }
                    let mut v_depth = 0i32;
                    let mut v_str = false;
                    let mut v_esc = false;
                    while i < bytes.len() {
                        let c = bytes[i];
                        if v_str {
                            if v_esc {
                                v_esc = false;
                            } else if c == b'\\' {
                                v_esc = true;
                            } else if c == b'"' {
                                v_str = false;
                            }
                        } else if c == b'"' {
                            v_str = true;
                        } else if c == b'{' || c == b'[' {
                            v_depth += 1;
                        } else if c == b'}' || c == b']' {
                            if v_depth == 0 {
                                break;
                            }
                            v_depth -= 1;
                        } else if (c == b',' || c == b'}') && v_depth == 0 {
                            break;
                        }
                        i += 1;
                    }
                    return Some(&object[value_at..i]);
                }
                in_str = false;
            }
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth -= 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}

fn json_unescape(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some('/') => out.push('/'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(ch) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installed(version: &str) -> Installed {
        Installed {
            version: version.into(),
            commit: "1111111".into(),
            channel: Channel::Release,
        }
    }

    fn manifest(version: &str) -> Manifest {
        Manifest {
            version: version.into(),
            title: format!("Slot {version}"),
            size: 1,
            sha256: [0; 32],
            binary_url: asset_url(version, BINARY_ASSET),
            notes: vec!["- Something new.".into()],
        }
    }

    #[test]
    fn newer_versions_are_offered_and_the_same_one_is_not() {
        let have = installed("v0.43");
        assert_eq!(offer(&have, &manifest("v0.44")), Offer::Newer);
        assert_eq!(offer(&have, &manifest("v0.43")), Offer::UpToDate);
        assert_eq!(offer(&have, &manifest("v0.9")), Offer::UpToDate);
        assert_eq!(offer(&installed("v0.9"), &manifest("v0.10")), Offer::Newer);
    }

    #[test]
    fn a_release_from_anywhere_else_is_refused() {
        assert!(check("https://github.com/LordNordyx/Slot-Brick/releases")
            .unwrap_err()
            .contains("Refusing"));
        assert!(check("https://example.com/slot")
            .unwrap_err()
            .contains("Refusing"));
        let listed = parse_releases(
            r#"[{"tag_name":"v9.0","name":"Nope","draft":false,"body":"- hi","assets":[
                {"name":"slot","size":4,"browser_download_url":"https://github.com/LordNordyx/Slot-Brick/releases/download/v9.0/slot"},
                {"name":"slot.sha256","size":64,"browser_download_url":"https://github.com/LordNordyx/Slot-Brick/releases/download/v9.0/slot.sha256"}]}]"#,
        );
        assert!(listed.unwrap_err().contains("No update"));
    }

    #[test]
    fn the_latest_non_draft_release_with_both_files_is_the_one_offered() {
        let json = r#"[{"tag_name":"v9.2","draft":true,"name":"Draft","body":"- no","assets":[
                {"name":"slot","size":4,"browser_download_url":"https://github.com/jfink22-design/slot-brick-plus/releases/download/v9.2/slot"},
                {"name":"slot.sha256","size":8,"browser_download_url":"https://github.com/jfink22-design/slot-brick-plus/releases/download/v9.2/slot.sha256"}]},
            {"tag_name":"updates","name":"Updates","draft":false,"body":"","assets":[]},
            {"tag_name":"v9.1","name":"Pictures stay put","draft":false,"body":"- Something new.\n- And a fix.","assets":[
                {"name":"slot","size":8,"browser_download_url":"https://github.com/jfink22-design/slot-brick-plus/releases/download/v9.1/slot"},
                {"name":"slot.sha256","size":70,"browser_download_url":"https://github.com/jfink22-design/slot-brick-plus/releases/download/v9.1/slot.sha256"}]}]"#;
        let listed = parse_releases(json).unwrap();
        assert_eq!(listed.version, "v9.1");
        assert_eq!(listed.title, "Pictures stay put");
        assert_eq!(listed.notes, ["- Something new.", "- And a fix."]);
        assert_eq!(listed.size, 8);
    }

    #[test]
    fn being_offline_is_said_plainly() {
        let said = explain("cannot look up api.github.com (no internet?)".into());
        assert!(said.contains("No Wi-Fi"));
        assert!(explain("server said 404".into()).contains("No update"));
    }

    #[test]
    fn only_slots_own_files_are_replaced() {
        for rel in [
            "Slot/System/slot",
            "Slot/System/licenses/a.txt",
            ".slot-boot/boot.sh",
            "Apps/Slot/launch.sh",
            "Apps/BootLogo/Image/README.txt",
            "Slot/Overlays/GB/A.png",
            "Slot/README.md",
            "slot-os/boot.img",
            "slot-os/lib/aarch64-linux-gnu/libc.so.6",
            "bootlogo.bmp",
        ] {
            assert!(ours(rel), "{rel}");
        }
        for rel in [
            "Slot/Games/GB/a.gb",
            "Slot/Saves/GB/a.sav",
            "Slot/Config/wifi.txt",
            "Slot/BIOS/gba_bios.bin",
            "Slot/AUTOBOOT",
            "Apps/BootLogo/Image/boot.png",
            "trimui/app/MainUI",
            "Slot/Wallpapers/a.png",
        ] {
            assert!(!ours(rel), "{rel}");
        }
        assert!(
            !safe("Slot/System/../../x")
                && !safe("Slot//x")
                && !safe("Slot\\x")
                && safe("Slot/System/slot")
        );
    }

    /// A card zip as zip-card.py writes one: deflated, under SD-CARD/, Unix modes kept.
    fn card_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let (mut out, mut dir) = (Vec::new(), Vec::new());
        for (name, data) in files {
            let packed = miniz_oxide::deflate::compress_to_vec(data, 6);
            let offset = out.len() as u32;
            let head = |buf: &mut Vec<u8>, central: bool| {
                buf.extend_from_slice(
                    &(if central { 0x0201_4b50u32 } else { 0x0403_4b50 }).to_le_bytes(),
                );
                if central {
                    buf.extend_from_slice(&(0x0314u16).to_le_bytes());
                }
                buf.extend_from_slice(&[20, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
                buf.extend_from_slice(&(packed.len() as u32).to_le_bytes());
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(&(name.len() as u16).to_le_bytes());
                buf.extend_from_slice(&[0, 0]);
                if central {
                    buf.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
                    buf.extend_from_slice(&(0o100755u32 << 16).to_le_bytes());
                    buf.extend_from_slice(&offset.to_le_bytes());
                }
                buf.extend_from_slice(name.as_bytes());
            };
            head(&mut out, false);
            out.extend_from_slice(&packed);
            head(&mut dir, true);
        }
        let at = out.len() as u32;
        out.extend_from_slice(&dir);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(files.len() as u16).to_le_bytes());
        out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
        out.extend_from_slice(&at.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    const NEW_CARD: &[(&str, &[u8])] = &[
        ("SD-CARD/Slot/System/slot", b"new slot"),
        ("SD-CARD/Slot/System/launch.sh", b"new launch"),
        ("SD-CARD/Slot/Overlays/GB/Frame.png", b"new overlay"),
        ("SD-CARD/Slot/Config/defaults.txt", b"not ours"),
        ("SD-CARD/Slot/AUTOBOOT", b"not ours"),
    ];

    fn old_card(card: &Path) {
        for (rel, data) in [
            ("Slot/System/slot", "old slot"),
            ("Slot/System/launch.sh", "old launch"),
            ("Slot/Saves/GB/a.sav", "save"),
            ("Slot/Config/defaults.txt", "mine"),
        ] {
            fs::create_dir_all(card.join(rel).parent().unwrap()).unwrap();
            fs::write(card.join(rel), data).unwrap();
        }
    }

    fn read(card: &Path, rel: &str) -> String {
        fs::read_to_string(card.join(rel)).unwrap_or_default()
    }

    #[test]
    fn unpacking_takes_only_slots_own_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("card.zip"), card_zip(NEW_CARD)).unwrap();
        assert_eq!(
            stage(&dir.path().join("card.zip"), &dir.path().join("new")).unwrap(),
            3
        );
        assert_eq!(
            read(&dir.path().join("new"), "Slot/System/slot"),
            "new slot"
        );
        assert!(
            !dir.path().join("new/Slot/Config").exists()
                && !dir.path().join("new/Slot/AUTOBOOT").exists()
        );
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dir.path().join("new/Slot/System/slot"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    #[test]
    fn a_zip_reaching_outside_the_card_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let evil: &[(&str, &[u8])] = &[
            ("SD-CARD/Slot/System/slot", b"x"),
            ("SD-CARD/Slot/System/../../../escape", b"x"),
        ];
        fs::write(dir.path().join("card.zip"), card_zip(evil)).unwrap();
        assert!(stage(&dir.path().join("card.zip"), &dir.path().join("new")).is_err());
        assert!(!dir.path().join("escape").exists());
    }

    /// A folder standing in for this fork's GitHub releases: releases.json, slot, slot.sha256.
    fn publish(dir: &Path, binary: &[u8]) -> (String, PathBuf) {
        let sha = sha256_hex(binary);
        let at = dir.join("releases");
        fs::create_dir_all(&at).unwrap();
        let binary_url = format!("file://{}/slot", at.display());
        let checksum_url = format!("file://{}/slot.sha256", at.display());
        let json = format!(
            r#"[{{"tag_name":"v9.0","name":"Pictures stay put","draft":false,"body":"- Something new.\n- And a fix.","assets":[{{"name":"slot","size":{},"browser_download_url":"{binary_url}"}},{{"name":"slot.sha256","size":70,"browser_download_url":"{checksum_url}"}}]}}]"#,
            binary.len()
        );
        fs::write(at.join("releases.json"), json).unwrap();
        fs::write(at.join("slot"), binary).unwrap();
        fs::write(at.join("slot.sha256"), format!("{sha}  slot\n")).unwrap();
        (format!("file://{}", at.display()), at)
    }

    #[test]
    fn an_update_replaces_slot_keeps_the_previous_binary_and_shows_the_title() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Slot");
        fs::create_dir_all(root.join("System")).unwrap();
        fs::create_dir_all(root.join("Saves/GB")).unwrap();
        fs::write(root.join("System/slot"), b"old slot").unwrap();
        fs::write(root.join("Saves/GB/a.sav"), b"save").unwrap();
        let (base, _) = publish(dir.path(), b"new slot");
        let m = check(&base).unwrap();
        assert_eq!(m.title, "Pictures stay put");
        assert_eq!(m.notes, ["- Something new.", "- And a fix."]);
        assert_eq!(offer(&installed("v0.43"), &m), Offer::Newer);
        install(&m, &root, &Progress::default()).unwrap();
        assert_eq!(fs::read(root.join("System/slot")).unwrap(), b"new slot");
        assert_eq!(fs::read(root.join("System/slot.bak")).unwrap(), b"old slot");
        assert_eq!(
            fs::read_to_string(root.join("System/slot.pending"))
                .unwrap()
                .trim(),
            "v9.0"
        );
        assert_eq!(fs::read(root.join("Saves/GB/a.sav")).unwrap(), b"save");
        let card = dir.path();
        // settle takes the card root, whose Slot/ is the one we wrote.
        fs::create_dir_all(card.join("Slot")).unwrap();
        // root is already dir/Slot. settle looks at card/Slot.
        settle(dir.path());
        assert!(!root.join("System/slot.pending").exists());
        assert_eq!(fs::read(root.join("System/slot.bak")).unwrap(), b"old slot");
    }

    #[test]
    fn a_bad_checksum_or_a_cancel_leaves_the_installed_slot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("Slot");
        fs::create_dir_all(root.join("System")).unwrap();
        fs::write(root.join("System/slot"), b"old slot").unwrap();
        let (base, at) = publish(dir.path(), b"new slot");
        let m = check(&base).unwrap();
        fs::write(at.join("slot"), b"evil!!!!").unwrap();
        assert!(install(&m, &root, &Progress::default())
            .unwrap_err()
            .contains("checksum"));
        assert_eq!(fs::read(root.join("System/slot")).unwrap(), b"old slot");
        assert!(!root.join("System/slot.bak").exists());
        assert!(!root.join("System/slot.pending").exists());
        let progress = Progress::default();
        progress.cancel.store(true, Ordering::Relaxed);
        fs::write(at.join("slot"), b"new slot").unwrap();
        assert_eq!(install(&m, &root, &progress).unwrap_err(), "Cancelled");
        assert_eq!(fs::read(root.join("System/slot")).unwrap(), b"old slot");
    }

    #[test]
    fn a_swap_cut_short_is_finished_at_startup() {
        let dir = tempfile::tempdir().unwrap();
        let card = dir.path().join("card");
        old_card(&card);
        let up = card.join(DIR);
        fs::write(dir.path().join("card.zip"), card_zip(NEW_CARD)).unwrap();
        stage(&dir.path().join("card.zip"), &up.join("new")).unwrap();
        fs::write(up.join(STAGED), "").unwrap();
        // The power went with launch.sh swapped and slot not yet.
        fs::create_dir_all(up.join("old/Slot/System")).unwrap();
        fs::rename(
            card.join("Slot/System/launch.sh"),
            up.join("old/Slot/System/launch.sh"),
        )
        .unwrap();
        fs::rename(
            up.join("new/Slot/System/launch.sh"),
            card.join("Slot/System/launch.sh"),
        )
        .unwrap();
        assert!(resume(&card));
        assert_eq!(
            (
                read(&card, "Slot/System/slot"),
                read(&card, "Slot/System/launch.sh")
            ),
            ("new slot".into(), "new launch".into())
        );
        assert_eq!(
            read(&card, ".slot-update/old/Slot/System/launch.sh"),
            "old launch"
        );
        assert_eq!(read(&card, ".slot-update/old/Slot/System/slot"), "old slot");
        assert!(!resume(&card));
    }

    #[test]
    fn a_half_finished_update_is_cleared_but_a_swap_under_way_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let card = dir.path();
        let up = card.join(DIR);
        tidy(card);
        fs::create_dir_all(up.join("new/Slot/System")).unwrap();
        fs::write(up.join("download.zip"), "half").unwrap();
        tidy(card);
        assert!(!up.exists());
        for marker in [STAGED, PENDING] {
            fs::create_dir_all(up.join("old")).unwrap();
            fs::write(up.join(marker), "").unwrap();
            tidy(card);
            assert!(
                up.join(marker).exists() && up.join("old").exists(),
                "{marker}"
            );
            fs::remove_dir_all(&up).unwrap();
        }
    }
}
