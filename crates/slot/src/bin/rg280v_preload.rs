//! Preload RG280V cart labels on the Mac from a ROM manifest, using ScreenScraper and
//! `slot-cartart`. Writes `Slot/Labels/<DIR>/<stem>.png` at 1× the label window.
//!
//! Credentials are never printed, logged, or written into reports. Media URLs (which carry
//! the account) are not logged either; errors go through `redact` first.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use slot::cart_art_ss::{
    fit_from_at, label_out_at, redact, stops_the_run, Client, Creds,
};
use slot_store::{Cart, Platform};

/// Systems hidden on the RG280V; skipped unless `--include-hidden` is passed.
const HIDDEN: [Platform; 3] = [Platform::N64, Platform::Nds, Platform::Psp];

const DEFAULT_OUT: &str = "/Users/jeff/Shared-Space/rg280v/Slot";
const DEFAULT_USER_LOGIN: &str = "screenscraper-user.properties";

fn main() {
    let code = match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("rg280v-preload: {}", redact(&e));
            1
        }
    };
    std::process::exit(code);
}

struct Args {
    manifest: PathBuf,
    out: PathBuf,
    scale: u32,
    dev_properties: Option<PathBuf>,
    user_login: Option<PathBuf>,
    roms_root: Option<PathBuf>,
    limit: Option<usize>,
    allow_dev_only: bool,
    dry_run: bool,
    include_hidden: bool,
    threads: Option<u32>,
}

fn usage() -> String {
    "Preload RG280V cart labels from a ROM manifest (host tool, no card writes).

Usage:
  cargo run -p slot --bin rg280v-preload -- --manifest GAMES.tsv [options]

Options:
  --manifest PATH        TSV: system dir, filename (extra columns ignored). Required.
  --out PATH             Slot root to write Labels/ into
                         (default /Users/jeff/Shared-Space/rg280v/Slot)
  --scale N              Times the label window, capped at 512 px (default 1)
  --dev-properties PATH  gitignored screenscraper.properties (devid / devpassword)
  --user-login PATH      gitignored screenscraper-user.properties (user= / password=)
  --roms-root PATH       Optional Games/ (or a tree of <DIR>/<file>) for CRC and size
  --limit N              Only the first N games after hidden systems are skipped
  --allow-dev-only       Allow a run with no user login (small tests only)
  --dry-run              Parse the manifest and report; do not call ScreenScraper
  --include-hidden       Also process N64, DS and PSP (off by default)
  --threads N            Cap worker threads (still never above the account's maxthreads)
".into()
}

fn parse_args() -> Result<Args, String> {
    let mut raw = std::env::args().skip(1);
    let mut manifest = None;
    let mut out = PathBuf::from(DEFAULT_OUT);
    let mut scale = 1u32;
    let mut dev_properties = None;
    let mut user_login = None;
    let mut roms_root = None;
    let mut limit = None;
    let mut allow_dev_only = false;
    let mut dry_run = false;
    let mut include_hidden = false;
    let mut threads = None;
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "-h" | "--help" => return Err(usage()),
            "--manifest" => manifest = Some(need(&mut raw, "--manifest")?),
            "--out" => out = need(&mut raw, "--out")?,
            "--scale" => {
                scale = need(&mut raw, "--scale")?
                    .to_str()
                    .unwrap_or("")
                    .parse()
                    .map_err(|_| "--scale needs a number".to_string())?
            }
            "--dev-properties" => dev_properties = Some(need(&mut raw, "--dev-properties")?),
            "--user-login" => user_login = Some(need(&mut raw, "--user-login")?),
            "--roms-root" => roms_root = Some(need(&mut raw, "--roms-root")?),
            "--limit" => {
                limit = Some(
                    need(&mut raw, "--limit")?
                        .to_str()
                        .unwrap_or("")
                        .parse()
                        .map_err(|_| "--limit needs a number".to_string())?,
                )
            }
            "--allow-dev-only" => allow_dev_only = true,
            "--dry-run" => dry_run = true,
            "--include-hidden" => include_hidden = true,
            "--threads" => {
                threads = Some(
                    need(&mut raw, "--threads")?
                        .to_str()
                        .unwrap_or("")
                        .parse()
                        .map_err(|_| "--threads needs a number".to_string())?,
                )
            }
            other => return Err(format!("unknown argument: {other}\n\n{}", usage())),
        }
    }
    let manifest = manifest.ok_or_else(|| format!("--manifest is required\n\n{}", usage()))?;
    Ok(Args {
        manifest,
        out,
        scale: scale.max(1),
        dev_properties,
        user_login,
        roms_root,
        limit,
        allow_dev_only,
        dry_run,
        include_hidden,
        threads,
    })
}

fn need(
    raw: &mut impl Iterator<Item = String>,
    flag: &str,
) -> Result<PathBuf, String> {
    raw.next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("{flag} needs a value"))
}

#[derive(Clone, Debug)]
struct Row {
    dir: String,
    file: String,
    famicom: bool,
    line: usize,
}

fn parse_manifest(path: &Path) -> Result<Vec<Row>, String> {
    let file = File::open(path).map_err(|e| format!("cannot read manifest: {e}"))?;
    let mut rows = Vec::new();
    for (i, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|e| format!("manifest read: {e}"))?;
        let raw = line.trim();
        if raw.is_empty() || raw.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = raw.split('\t').collect();
        if parts.len() < 2 {
            return Err(format!(
                "manifest line {}: expected system dir and filename, tab-separated",
                i + 1
            ));
        }
        let dir = parts[0].trim();
        let file = parts[1].trim();
        if dir.is_empty() || file.is_empty() {
            return Err(format!("manifest line {}: empty system dir or filename", i + 1));
        }
        let note = parts.get(2).copied().unwrap_or("");
        let famicom = parts
            .iter()
            .any(|p| p.trim().eq_ignore_ascii_case("famicom"));
        let _ = note;
        rows.push(Row {
            dir: dir.to_string(),
            file: file.to_string(),
            famicom,
            line: i + 1,
        });
    }
    Ok(rows)
}

fn platform_from_dir(dir: &str) -> Option<Platform> {
    Platform::ALL
        .iter()
        .copied()
        .find(|p| p.dir_name().eq_ignore_ascii_case(dir))
}

fn is_hidden(p: Platform) -> bool {
    HIDDEN.contains(&p)
}

fn stem_of(file: &str) -> String {
    Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file)
        .to_string()
}

fn label_path(out: &Path, dir: &str, stem: &str, famicom: bool) -> PathBuf {
    let mut p = out.join("Labels").join(dir);
    if famicom {
        p.push("Famicom");
    }
    p.join(format!("{stem}.png"))
}

fn already_done(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    bytes.len() > 32 && bytes.starts_with(b"\x89PNG")
}

fn find_dev_properties(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Some(p) = std::env::var_os("SLOT_SS_PROPERTIES") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    for cand in [
        PathBuf::from("screenscraper.properties"),
        PathBuf::from("android/screenscraper.properties"),
    ] {
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

fn find_user_login(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Some(p) = std::env::var_os("SLOT_SS_USER_PROPERTIES") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let p = PathBuf::from(DEFAULT_USER_LOGIN);
    if p.is_file() {
        Some(p)
    } else {
        None
    }
}

fn clean_query(stem: &str) -> String {
    let mut s = stem.to_string();
    while let Some(start) = s.rfind('(') {
        if let Some(end) = s[start..].find(')') {
            s.replace_range(start..start + end + 1, "");
        } else {
            break;
        }
    }
    s = s.replace(", The", "");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn rom_path(roms_root: Option<&Path>, dir: &str, file: &str) -> PathBuf {
    if let Some(root) = roms_root {
        let a = root.join(dir).join(file);
        if a.is_file() {
            return a;
        }
        let b = root.join("Games").join(dir).join(file);
        if b.is_file() {
            return b;
        }
    }
    // A dummy path whose file name is the ROM name, so jeuInfos can use romnom.
    PathBuf::from(file)
}

fn make_cart(platform: Platform, file: &str, rom: PathBuf) -> Cart {
    Cart {
        platform,
        stem: stem_of(file),
        rom,
        label: None,
        famicom_label: None,
        title: String::new(),
        code: String::new(),
        shell: None,
    }
}

enum Outcome {
    Cached,
    Wrote { how: String, width: u32, height: u32 },
    Miss { reason: String },
}

fn process_one(
    client: &mut Client,
    cart: &Cart,
    dest: &Path,
    scale: u32,
    fetch: bool,
) -> Result<Outcome, String> {
    if already_done(dest) {
        return Ok(Outcome::Cached);
    }
    if !fetch {
        return Ok(Outcome::Miss {
            reason: "dry-run (not fetched)".into(),
        });
    }
    let size = fs::metadata(&cart.rom).ok().map(|m| m.len());
    let crc = None; // ROM bytes are optional; name lookup is enough for a manifest-only run.
    let medias = match client.lookup(cart, crc, size) {
        Ok(Some(m)) => m,
        Ok(None) => {
            let term = clean_query(&cart.stem);
            match client.search(cart.platform, &term) {
                Ok(found) => found.into_iter().next().map(|f| f.medias).unwrap_or_default(),
                Err(e) => return Err(e),
            }
        }
        Err(e) => return Err(e),
    };
    if medias.is_empty() {
        return Ok(Outcome::Miss {
            reason: "not found".into(),
        });
    }
    let mut got: Option<(Vec<u8>, String)> = None;
    let tried = fit_from_at(client, cart, &medias, false, scale, &mut |png, how| {
        got = Some((png, how));
        false
    })?;
    match got {
        Some((png, how)) => {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create Labels directory: {e}"))?;
            }
            fs::write(dest, &png).map_err(|e| format!("cannot write label: {e}"))?;
            let (width, height) = png_size(&png).unwrap_or(label_out_at(cart, scale));
            let _ = tried;
            Ok(Outcome::Wrote { how, width, height })
        }
        None if tried.is_empty() => Ok(Outcome::Miss {
            reason: "no usable pictures".into(),
        }),
        None => Ok(Outcome::Miss {
            reason: format!("tried {}", tried.join(", ")),
        }),
    }
}

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG") {
        return None;
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    Some((w, h))
}

fn append_report(path: &Path, line: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("cannot create report directory: {e}"))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("cannot write report: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("cannot write report: {e}"))?;
    Ok(())
}

fn ensure_report_header(path: &Path, header: &str) -> Result<(), String> {
    if path.is_file() && fs::metadata(path).map(|m| m.len()).unwrap_or(0) > 0 {
        return Ok(());
    }
    append_report(path, header)
}

struct Counters {
    cached: AtomicU64,
    wrote: AtomicU64,
    miss: AtomicU64,
    skipped: AtomicU64,
}

fn run() -> Result<(), String> {
    let args = parse_args()?;
    let rows = parse_manifest(&args.manifest)?;
    let mut work = Vec::new();
    let mut skipped_hidden = 0u64;
    let mut unknown = 0u64;
    for row in rows {
        match platform_from_dir(&row.dir) {
            None => {
                unknown += 1;
                eprintln!(
                    "skip unknown system dir on line {}: {}",
                    row.line, row.dir
                );
            }
            Some(p) if is_hidden(p) && !args.include_hidden => {
                skipped_hidden += 1;
            }
            Some(p) => work.push((p, row)),
        }
    }
    if let Some(limit) = args.limit {
        work.truncate(limit);
    }

    println!(
        "manifest {}: {} to process, {} hidden skipped, {} unknown dirs; scale {}×; out {}",
        args.manifest.display(),
        work.len(),
        skipped_hidden,
        unknown,
        args.scale,
        args.out.display()
    );

    if args.dry_run {
        let mut cached = 0u64;
        for (platform, row) in &work {
            let dest = label_path(&args.out, platform.dir_name(), &stem_of(&row.file), row.famicom);
            if already_done(&dest) {
                cached += 1;
            }
        }
        println!(
            "dry-run: {cached} already have labels, {} would be fetched",
            work.len() as u64 - cached
        );
        return Ok(());
    }

    let dev = find_dev_properties(args.dev_properties.as_deref());
    let user = find_user_login(args.user_login.as_deref());
    let creds = Creds::from_files(dev.as_deref(), user.as_deref())?;
    if !creds.logged_in() && !args.allow_dev_only {
        return Err(
            "no ScreenScraper user login. Fill in user= and password= in the user login file \
             (see --user-login), or pass --allow-dev-only for a small test"
                .into(),
        );
    }
    if creds.logged_in() {
        println!("ScreenScraper: using a user login (username is not printed here)");
    } else {
        println!("ScreenScraper: developer id only (--allow-dev-only)");
    }

    let mut client0 = Client::new(creds.clone());
    let mut threads = 1u32;
    let mut remaining_calls: Option<u64> = None;
    if creds.logged_in() {
        match client0.user_info() {
            Ok(info) => {
                // Line already names the user; do not print it to keep logs quieter.
                let t = if info.threads == 0 { 1 } else { info.threads };
                threads = t.min(u64::from(args.threads.unwrap_or(t as u32))) as u32;
                threads = threads.max(1);
                if info.per_day > 0 {
                    remaining_calls = Some(info.per_day.saturating_sub(info.today));
                    println!(
                        "account level {}, {} thread{}, {} of {} lookups used today ({} left)",
                        if info.level.is_empty() {
                            "?".into()
                        } else {
                            info.level
                        },
                        threads,
                        if threads == 1 { "" } else { "s" },
                        info.today,
                        info.per_day,
                        remaining_calls.unwrap_or(0)
                    );
                } else {
                    println!("account allows {threads} thread{}", if threads == 1 { "" } else { "s" });
                }
            }
            Err(e) => {
                let e = redact(&e);
                if stops_the_run(&e) {
                    return Err(e);
                }
                eprintln!("could not read account quota ({e}); continuing with 1 thread");
                threads = 1;
            }
        }
    } else if let Some(n) = args.threads {
        threads = n.max(1);
    }

    // Reports sit next to Slot/: /Users/jeff/Shared-Space/rg280v/preload-report.tsv
    let reports_dir = args
        .out
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| args.out.clone());
    let report = reports_dir.join("preload-report.tsv");
    let misses = reports_dir.join("preload-misses.tsv");
    ensure_report_header(
        &report,
        "status\tsystem\tfile\tpath\twidth\theight\tdetail",
    )?;
    ensure_report_header(&misses, "system\tfile\treason")?;

    let stop = Arc::new(AtomicBool::new(false));
    let calls_left = remaining_calls.map(|n| Arc::new(AtomicU64::new(n)));
    let counters = Arc::new(Counters {
        cached: AtomicU64::new(0),
        wrote: AtomicU64::new(0),
        miss: AtomicU64::new(0),
        skipped: AtomicU64::new(skipped_hidden),
    });
    let queue = Arc::new(Mutex::new(work.into_iter()));
    let report = Arc::new((report, misses));
    let out = args.out.clone();
    let scale = args.scale;
    let roms_root = args.roms_root.clone();
    let start = Instant::now();

    let mut handles = Vec::new();
    for _ in 0..threads {
        let queue = Arc::clone(&queue);
        let stop = Arc::clone(&stop);
        let counters = Arc::clone(&counters);
        let report = Arc::clone(&report);
        let calls_left = calls_left.clone();
        let out = out.clone();
        let creds = creds.clone();
        let roms_root = roms_root.clone();
        handles.push(thread::spawn(move || {
            let mut client = Client::new(creds);
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                let job = {
                    let mut q = queue.lock().unwrap();
                    q.next()
                };
                let Some((platform, row)) = job else {
                    break;
                };
                let dest = label_path(&out, platform.dir_name(), &stem_of(&row.file), row.famicom);
                if already_done(&dest) {
                    counters.cached.fetch_add(1, Ordering::Relaxed);
                    let _ = append_report(
                        &report.0,
                        &format!(
                            "cached\t{}\t{}\t{}\t\t\t",
                            platform.dir_name(),
                            row.file,
                            dest.display()
                        ),
                    );
                    continue;
                }
                if let Some(left) = &calls_left {
                    // Budget two calls (lookup + one picture); stop before we trip the cap.
                    if left
                        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(2))
                        .is_err()
                    {
                        stop.store(true, Ordering::SeqCst);
                        println!("daily cap reached; stopping so the rest can resume later");
                        break;
                    }
                }
                let rom = rom_path(roms_root.as_deref(), platform.dir_name(), &row.file);
                let cart = make_cart(platform, &row.file, rom);
                let mut result = process_one(&mut client, &cart, &dest, scale, true);
                for attempt in 1..3 {
                    match &result {
                        Err(e) if !stops_the_run(&redact(e)) => {
                            thread::sleep(Duration::from_secs(2));
                            result = process_one(&mut client, &cart, &dest, scale, true);
                        }
                        _ => break,
                    }
                    let _ = attempt;
                }
                match result {
                    Ok(Outcome::Cached) => {
                        counters.cached.fetch_add(1, Ordering::Relaxed);
                        let _ = append_report(
                            &report.0,
                            &format!(
                                "cached\t{}\t{}\t{}\t\t\t",
                                platform.dir_name(),
                                row.file,
                                dest.display()
                            ),
                        );
                    }
                    Ok(Outcome::Wrote { how, width, height }) => {
                        counters.wrote.fetch_add(1, Ordering::Relaxed);
                        println!(
                            "ok {} {} {}x{} {}",
                            platform.dir_name(),
                            row.file,
                            width,
                            height,
                            redact(&how)
                        );
                        let _ = append_report(
                            &report.0,
                            &format!(
                                "ok\t{}\t{}\t{}\t{width}\t{height}\t{}",
                                platform.dir_name(),
                                row.file,
                                dest.display(),
                                redact(&how)
                            ),
                        );
                    }
                    Ok(Outcome::Miss { reason }) => {
                        counters.miss.fetch_add(1, Ordering::Relaxed);
                        println!("miss {} {}: {}", platform.dir_name(), row.file, redact(&reason));
                        let _ = append_report(
                            &report.0,
                            &format!(
                                "miss\t{}\t{}\t\t\t\t{}",
                                platform.dir_name(),
                                row.file,
                                redact(&reason)
                            ),
                        );
                        let _ = append_report(
                            &report.1,
                            &format!("{}\t{}\t{}", platform.dir_name(), row.file, redact(&reason)),
                        );
                    }
                    Err(e) => {
                        let e = redact(&e);
                        if stops_the_run(&e) {
                            stop.store(true, Ordering::SeqCst);
                            eprintln!("stopping: {e}");
                            let _ = append_report(
                                &report.0,
                                &format!(
                                    "stop\t{}\t{}\t\t\t\t{}",
                                    platform.dir_name(),
                                    row.file,
                                    e
                                ),
                            );
                            break;
                        }
                        counters.miss.fetch_add(1, Ordering::Relaxed);
                        println!("miss {} {}: {}", platform.dir_name(), row.file, e);
                        let _ = append_report(
                            &report.1,
                            &format!("{}\t{}\t{}", platform.dir_name(), row.file, e),
                        );
                    }
                }
            }
        }));
    }
    for h in handles {
        h.join().map_err(|_| "a worker thread panicked".to_string())?;
    }

    let wrote = counters.wrote.load(Ordering::Relaxed);
    let cached = counters.cached.load(Ordering::Relaxed);
    let miss = counters.miss.load(Ordering::Relaxed);
    println!(
        "done in {:.1}s: wrote {wrote}, cached {cached}, misses {miss}, hidden skipped {}",
        start.elapsed().as_secs_f32(),
        counters.skipped.load(Ordering::Relaxed)
    );
    println!("labels: {}", out.join("Labels").display());
    println!("report: {}", report.0.display());
    println!("misses: {}", report.1.display());
    Ok(())
}

// Tiny tests for the manifest parser live in this file so `cargo test -p slot --bin rg280v-preload`
// covers them without touching the network.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_reads_dir_and_filename_and_skips_comments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("games.tsv");
        std::fs::write(
            &path,
            "# dir\tfile\nGB\tTetris (World).gb\n\nNES\tSuper Mario Bros. (World).nes\tfamicom\n",
        )
        .unwrap();
        let rows = parse_manifest(&path).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].dir, "GB");
        assert_eq!(rows[0].file, "Tetris (World).gb");
        assert!(!rows[0].famicom);
        assert!(rows[1].famicom);
    }

    #[test]
    fn hidden_shelves_are_the_three_heavy_ones() {
        assert!(is_hidden(Platform::N64) && is_hidden(Platform::Nds) && is_hidden(Platform::Psp));
        assert!(!is_hidden(Platform::Gb) && !is_hidden(Platform::Snes));
    }

    #[test]
    fn platform_dirs_match_the_shelf_names() {
        assert_eq!(platform_from_dir("gba").unwrap(), Platform::Gba);
        assert_eq!(platform_from_dir("32X").unwrap(), Platform::S32x);
        assert!(platform_from_dir("DOS").is_none());
    }
}
