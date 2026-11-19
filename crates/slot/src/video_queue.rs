//! A download that keeps going while a game is open.
//!
//! The Videos screen writes `Videos/queue.txt` and starts `slot --video-queue`. That process
//! is not the UI thread: a panic there is logged and the game is left alone. It runs niced,
//! throttled, and resumes a partial file with an HTTP Range after sleep or a reboot.
//! Downloads keep running during a game unless Videos → Pause downloads in games is turned on.
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::videos::{self, Saved};
use crate::youtube;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub id: String,
    pub got: u64,
    pub total: u64,
}

const QUEUE: &str = "Videos/queue.txt";
const PROGRESS: &str = "Videos/progress.txt";
const HOLD: &str = "Videos/hold";
const GAME_PAUSE: &str = "Videos/pause-games";
const SETTING: &str = "Config/videos.txt";
/// One HTTP range, in the span yt-dlp uses. A long open-ended request is what
/// googlevideo closes.
const RANGE_CHUNK: u64 = 4 * 1024 * 1024;
const READ: usize = 64 * 1024;
const GAP: Duration = Duration::from_millis(20);
const TRIES: u32 = 10;
const ERROR_FILE: &str = "Videos/error.txt";
pub const GIVE_UP: &str = "Download failed. The connection kept dropping. It will keep trying.";

pub fn jobs(root: &Path) -> Vec<Job> {
    let Ok(text) = fs::read_to_string(root.join(QUEUE)) else {
        return Vec::new();
    };
    text.lines().filter_map(parse_job).collect()
}

pub fn progress(root: &Path) -> Option<Progress> {
    let text = fs::read_to_string(root.join(PROGRESS)).ok()?;
    let mut fields = text.split_whitespace();
    Some(Progress {
        id: fields.next()?.to_string(),
        got: fields.next()?.parse().ok()?,
        total: fields.next()?.parse().ok()?,
    })
}

pub fn enqueue(
    root: &Path,
    id: &str,
    title: &str,
    channel: &str,
    duration_ms: u64,
) -> Result<(), String> {
    if !youtube::valid_id(id) {
        return Err("That video cannot be saved".into());
    }
    let mut rows = jobs(root);
    if rows.iter().any(|job| job.id == id) {
        return Err("Already in the queue".into());
    }
    rows.push(Job {
        id: id.to_string(),
        title: title.to_string(),
        channel: channel.to_string(),
        duration_ms,
    });
    write_jobs(root, &rows).map_err(|e| format!("Cannot save the queue: {e}"))?;
    videos::log_line(root, &format!("queued {id} {title}"));
    ensure(root);
    Ok(())
}

pub fn remove(root: &Path, id: &str) {
    let mut rows = jobs(root);
    rows.retain(|job| job.id != id);
    let _ = write_jobs(root, &rows);
    let _ = fs::remove_file(videos::video_path(root, id).with_extension("mp4.part"));
    videos::log_line(root, &format!("removed {id}"));
}

/// Off unless the player turns it on. Heavy systems such as PS1 can opt in.
pub fn pause_in_games(root: &Path) -> bool {
    fs::read_to_string(root.join(SETTING))
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("pause_in_game="))
                .map(|v| v.trim() == "1")
        })
        .unwrap_or(false)
}

pub fn set_pause_in_games(root: &Path, on: bool) -> std::io::Result<()> {
    let mut lines: Vec<String> = fs::read_to_string(root.join(SETTING))
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.starts_with("pause_in_game="))
        .map(str::to_string)
        .collect();
    lines.push(format!("pause_in_game={}", u8::from(on)));
    let _ = fs::create_dir_all(root.join("Config"));
    slot_store::atomic_write(&root.join(SETTING), lines.join("\n").as_bytes())?;
    if !on {
        let _ = fs::remove_file(root.join(GAME_PAUSE));
    }
    Ok(())
}

/// Called from the UI thread. Writes a flag the downloader reads. Default (setting off)
/// never pauses a download because a game started.
pub fn note_game(root: &Path, playing: bool) {
    let path = root.join(GAME_PAUSE);
    if playing && pause_in_games(root) {
        let _ = fs::create_dir_all(root.join("Videos"));
        let _ = fs::write(&path, b"1\n");
    } else if path.exists() {
        let _ = fs::remove_file(path);
    }
}

/// Start the downloader when the queue has work and one is not already running.
/// Tests do not spawn it. A live downloader, or a wait between tries, spawns nothing.
pub fn ensure(root: &Path) {
    if cfg!(test) {
        return;
    }
    consider_spawn(root, "video-queue", "--video-queue", !jobs(root).is_empty());
}

/// The `--video-queue` process. A failure here is logged and does not touch the UI.
pub fn run(root: &Path) {
    let root = root.to_path_buf();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if let Err(e) = daemon(&root) {
            videos::log_line(&root, &format!("downloader: {e}"));
        }
    }));
}

fn daemon(root: &Path) -> Result<(), String> {
    let _lock = claim_downloader("video-queue", "--video-queue")?;
    be_quiet();
    videos::log_line(root, "downloader running");
    loop {
        let Some(job) = jobs(root).into_iter().next() else {
            let _ = fs::remove_file(root.join(PROGRESS));
            videos::log_line(root, "queue empty");
            return Ok(());
        };
        while held(root) && jobs(root).iter().any(|row| row.id == job.id) {
            thread::sleep(Duration::from_millis(200));
        }
        if !jobs(root).iter().any(|row| row.id == job.id) {
            let _ = fs::remove_file(part_path(root, &job.id));
            continue;
        }
        match fetch(root, &job) {
            Ok(()) => {}
            Err(e) if e == "removed" => {}
            Err(e) => {
                videos::log_line(root, &format!("failed {} {e}", job.id));
                thread::sleep(Duration::from_secs(8));
            }
        }
    }
}

struct Chunk {
    end: u64,
    total: Option<u64>,
    finished: bool,
    advanced: bool,
    status: u16,
    content_length: Option<u64>,
}

struct Stream {
    name: String,
    url: String,
    itag: String,
}

fn fetch(root: &Path, job: &Job) -> Result<(), String> {
    videos::log_line(root, &format!("start {} {}", job.id, job.title));
    let part = part_path(root, &job.id);
    if let Some(dir) = part.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut source = 0usize;
    let mut media = next_media(&job.id, &mut source)?;
    let mut tries = 0u32;
    loop {
        if !still_queued(root, &job.id) {
            let _ = fs::remove_file(&part);
            return Err("removed".into());
        }
        while held(root) && still_queued(root, &job.id) {
            thread::sleep(Duration::from_millis(200));
        }
        let have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let end = have.saturating_add(RANGE_CHUNK).saturating_sub(1);
        match pull_chunk(&media.url, &part, have, end) {
            Ok(chunk) if chunk.advanced || chunk.finished => {
                tries = 0;
                let _ = fs::remove_file(root.join(ERROR_FILE));
                write_progress(root, &job.id, chunk.end, chunk.total.unwrap_or(0));
                videos::log_line(
                    root,
                    &format!(
                        "chunk {} client {} itag {} http {} content-length {} bytes {}",
                        job.id,
                        media.name,
                        media.itag,
                        chunk.status,
                        chunk
                            .content_length
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "-".into()),
                        chunk.end
                    ),
                );
                if chunk.finished || chunk.total.is_some_and(|total| chunk.end >= total) {
                    return finish(root, job, &part);
                }
            }
            other => {
                let (kind, detail) = match other {
                    Ok(_) => ("stall", "chunk made no progress".to_string()),
                    Err(e) => (kind_of(&e), e),
                };
                if detail.contains("already complete") && have > 0 {
                    return finish(root, job, &part);
                }
                tries += 1;
                videos::log_line(
                    root,
                    &format!(
                        "chunk {} client {} itag {} at {have} try {tries}/{TRIES} {kind}: {detail}",
                        job.id, media.name, media.itag
                    ),
                );
                if kind == "denied" || kind == "closed" || tries % 3 == 0 {
                    source += 1;
                    match next_media(&job.id, &mut source) {
                        Ok(fresh) => {
                            videos::log_line(
                                root,
                                &format!(
                                    "refresh {} client {} itag {}",
                                    job.id, fresh.name, fresh.itag
                                ),
                            );
                            media = fresh;
                        }
                        Err(e) => {
                            videos::log_line(root, &format!("refresh {} failed: {e}", job.id))
                        }
                    }
                }
                if tries >= TRIES {
                    let _ = fs::write(root.join(ERROR_FILE), GIVE_UP);
                    return Err(format!("{GIVE_UP} ({kind}: {detail})"));
                }
                thread::sleep(Duration::from_millis(backoff_ms(tries)));
            }
        }
    }
}

fn next_media(id: &str, source: &mut usize) -> Result<Stream, String> {
    let start = *source;
    loop {
        match youtube::media_at(id, *source) {
            Some(Ok((name, media))) => {
                return Ok(Stream {
                    itag: itag_of(&media.video_url),
                    url: media.video_url,
                    name,
                });
            }
            Some(Err(e)) => {
                *source += 1;
                if *source > start + 12 {
                    return Err(e);
                }
            }
            None => {
                if start == 0 {
                    return Err("No stream address for that video".into());
                }
                *source = 0;
                return match youtube::media_for(id) {
                    Ok(media) => Ok(Stream {
                        itag: itag_of(&media.video_url),
                        url: media.video_url,
                        name: "fallback".into(),
                    }),
                    Err(e) => Err(e),
                };
            }
        }
    }
}

fn itag_of(url: &str) -> String {
    url.split("itag=")
        .nth(1)
        .and_then(|rest| rest.split(['&', '#']).next())
        .filter(|s| !s.is_empty())
        .unwrap_or("-")
        .to_string()
}

fn pull_chunk(url: &str, part: &Path, have: u64, end: u64) -> Result<Chunk, String> {
    let ranged = crate::net::get_range(url, have, Some(end)).map_err(|e| e.to_string())?;
    if ranged.from_start && have > 0 {
        return Err("closed: server ignored the range".into());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(part)
        .map_err(|e| e.to_string())?;
    let start = if ranged.from_start { 0 } else { have };
    if ranged.from_start {
        file.set_len(0).map_err(|e| e.to_string())?;
    }
    file.seek(SeekFrom::Start(start))
        .map_err(|e| e.to_string())?;
    let want = end.saturating_sub(start).saturating_add(1);
    let mut reader = ranged.reader;
    let mut buf = vec![0u8; READ];
    let mut got = 0u64;
    while got < want {
        let n = match crate::net::read_soft(&mut *reader, &mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if crate::net::is_again(&e) => continue,
            Err(e) => return Err(format!("{}: {e}", crate::net::error_kind(&e))),
        };
        let n = n.min((want - got) as usize);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        thread::sleep(GAP);
    }
    file.sync_all().map_err(|e| e.to_string())?;
    let end_at = start + got;
    let finished = ranged.total.is_some_and(|total| end_at >= total);
    // A clean close before the chunk was full is a drop, not the end of the file,
    // unless the server already told us the total.
    if !finished && got < want {
        if end_at > have {
            return Ok(Chunk {
                end: end_at,
                total: ranged.total,
                finished: false,
                advanced: true,
                status: ranged.status,
                content_length: ranged.content_length,
            });
        }
        return Err("closed: connection closed before the chunk finished".into());
    }
    Ok(Chunk {
        end: end_at,
        total: ranged.total,
        finished,
        advanced: end_at > have,
        status: ranged.status,
        content_length: ranged.content_length,
    })
}

fn finish(root: &Path, job: &Job, part: &Path) -> Result<(), String> {
    let dest = videos::video_path(root, &job.id);
    fs::rename(part, &dest).map_err(|e| e.to_string())?;
    let bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let mut library = videos::read_index(root);
    library.retain(|row| row.id != job.id);
    library.push(Saved {
        id: job.id.clone(),
        title: job.title.clone(),
        channel: job.channel.clone(),
        bytes,
        duration_ms: job.duration_ms,
        position_ms: 0,
    });
    videos::write_index(root, &library).map_err(|e| e.to_string())?;
    let _ = videos::fetch_thumbnail(root, &job.id);
    let mut rows = jobs(root);
    rows.retain(|row| row.id != job.id);
    write_jobs(root, &rows).map_err(|e| e.to_string())?;
    let _ = fs::remove_file(root.join(ERROR_FILE));
    videos::log_line(root, &format!("saved {} {bytes} bytes", job.id));
    Ok(())
}

fn still_queued(root: &Path, id: &str) -> bool {
    jobs(root).iter().any(|row| row.id == id)
}

pub fn last_error(root: &Path) -> Option<String> {
    fs::read_to_string(root.join(ERROR_FILE))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn kind_of(error: &str) -> &'static str {
    let text = error.to_lowercase();
    if text.contains("403") || text.contains("denied") || text.contains("forbidden") {
        "denied"
    } else if text.contains("again")
        || text.contains("temporarily unavailable")
        || text.contains("timed out")
        || text.contains("would block")
    {
        "again"
    } else if text.contains("closed") || text.contains("reset") || text.contains("broken pipe") {
        "closed"
    } else {
        "io"
    }
}

pub(crate) fn backoff_ms(try_n: u32) -> u64 {
    let shift = try_n.saturating_sub(1).min(6);
    500u64.saturating_mul(1u64 << shift).min(30_000)
}

/// A on the queue screen. The downloader waits while this file is present.
pub fn toggle_hold(root: &Path) -> bool {
    let path = root.join(HOLD);
    if path.is_file() {
        let _ = fs::remove_file(path);
        false
    } else {
        let _ = fs::create_dir_all(root.join("Videos"));
        let _ = fs::write(&path, b"1\n");
        true
    }
}

fn held(root: &Path) -> bool {
    root.join(HOLD).is_file() || root.join(GAME_PAUSE).is_file()
}

fn part_path(root: &Path, id: &str) -> PathBuf {
    videos::video_path(root, id).with_extension("mp4.part")
}

fn write_progress(root: &Path, id: &str, got: u64, total: u64) {
    let _ = fs::write(root.join(PROGRESS), format!("{id} {got} {total}\n"));
}

fn parse_job(line: &str) -> Option<Job> {
    let mut fields = line.split('\t');
    let id = fields.next()?.to_string();
    if !youtube::valid_id(&id) {
        return None;
    }
    Some(Job {
        id,
        title: fields.next()?.to_string(),
        channel: fields.next()?.to_string(),
        duration_ms: fields.next()?.parse().ok()?,
    })
}

fn write_jobs(root: &Path, rows: &[Job]) -> std::io::Result<()> {
    let mut text = String::new();
    for job in rows {
        text.push_str(&job.id);
        text.push('\t');
        text.push_str(&flat(&job.title));
        text.push('\t');
        text.push_str(&flat(&job.channel));
        text.push('\t');
        text.push_str(&job.duration_ms.to_string());
        text.push('\n');
    }
    let _ = fs::create_dir_all(root.join("Videos"));
    slot_store::atomic_write(&root.join(QUEUE), text.as_bytes())
}

fn flat(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() || c == '\t' { ' ' } else { c })
        .collect()
}

/// How many times the UI will try to start a downloader before it says so and stops.
pub(crate) const MAX_SPAWN_TRIES: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SpawnChoice {
    Spawn,
    Wait,
    GiveUp,
}

#[derive(Clone, Debug)]
pub(crate) struct SpawnBudget {
    pub tries: u32,
    pub next_ms: u64,
    pub stopped: bool,
}

impl SpawnBudget {
    pub(crate) fn new() -> Self {
        Self {
            tries: 0,
            next_ms: 0,
            stopped: false,
        }
    }
}

/// `running` means a live downloader is already doing the work. A failure waits at least
/// five seconds, then twice as long each time, and stops after [`MAX_SPAWN_TRIES`].
pub(crate) fn choose_spawn(budget: &mut SpawnBudget, now_ms: u64, running: bool) -> SpawnChoice {
    if running {
        *budget = SpawnBudget::new();
        return SpawnChoice::Wait;
    }
    if budget.stopped {
        return SpawnChoice::GiveUp;
    }
    if now_ms < budget.next_ms {
        return SpawnChoice::Wait;
    }
    if budget.tries >= MAX_SPAWN_TRIES {
        budget.stopped = true;
        return SpawnChoice::GiveUp;
    }
    let shift = budget.tries.min(4);
    let delay = (5_000u64 << shift).min(60_000);
    budget.tries += 1;
    budget.next_ms = now_ms.saturating_add(delay);
    SpawnChoice::Spawn
}

pub(crate) struct PidGuard {
    path: PathBuf,
    pid: u32,
}

impl Drop for PidGuard {
    fn drop(&mut self) {
        if read_pid(&self.path) == Some(self.pid) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn pid_path(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/slot-{name}.pid"))
}

/// A downloader whose pid still answers and whose command line is this queue.
pub(crate) fn downloader_alive(name: &str, flag: &str) -> bool {
    let Some(pid) = read_pid(&pid_path(name)) else {
        return false;
    };
    process_alive(pid) && cmdline_has(pid, flag)
}

pub(crate) fn claim_downloader(name: &str, flag: &str) -> Result<PidGuard, String> {
    let path = pid_path(name);
    for _ in 0..2 {
        if downloader_alive(name, flag) {
            return Err("downloader already running".into());
        }
        let _ = fs::remove_file(&path);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                let pid = std::process::id();
                use std::io::Write;
                writeln!(file, "{pid}").map_err(|e| e.to_string())?;
                return Ok(PidGuard { path, pid });
            }
            Err(_) if downloader_alive(name, flag) => {
                return Err("downloader already running".into());
            }
            Err(_) => continue,
        }
    }
    Err("downloader already running".into())
}

fn read_pid(path: &Path) -> Option<u32> {
    let text = fs::read_to_string(path).ok()?;
    let pid: u32 = text.trim().parse().ok()?;
    (pid > 1).then_some(pid)
}

fn process_alive(pid: u32) -> bool {
    let rc = unsafe { libc::kill(pid as i32, 0) };
    if rc == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn cmdline_has(pid: u32, flag: &str) -> bool {
    let Ok(buf) = fs::read(format!("/proc/{pid}/cmdline")) else {
        return false;
    };
    String::from_utf8_lossy(&buf).contains(flag)
}

struct Gate {
    child: Option<std::process::Child>,
    budget: SpawnBudget,
    told: bool,
}

fn gate(name: &str) -> &'static Mutex<Gate> {
    if name == "music-queue" {
        static MUSIC: OnceLock<Mutex<Gate>> = OnceLock::new();
        return MUSIC.get_or_init(|| Mutex::new(Gate::fresh()));
    }
    static VIDEO: OnceLock<Mutex<Gate>> = OnceLock::new();
    VIDEO.get_or_init(|| Mutex::new(Gate::fresh()))
}

impl Gate {
    fn fresh() -> Self {
        Self {
            child: None,
            budget: SpawnBudget::new(),
            told: false,
        }
    }
}

pub(crate) fn consider_spawn(root: &Path, name: &str, flag: &str, wanted: bool) {
    let Ok(mut gate) = gate(name).lock() else {
        return;
    };
    if let Some(child) = gate.child.as_mut() {
        match child.try_wait() {
            Ok(None) => {}
            _ => gate.child = None,
        }
    }
    let running = gate.child.is_some() || downloader_alive(name, flag);
    if !wanted {
        if running {
            gate.budget = SpawnBudget::new();
        }
        return;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    match choose_spawn(&mut gate.budget, now, running) {
        SpawnChoice::Wait => {}
        SpawnChoice::GiveUp => {
            if !gate.told {
                gate.told = true;
                let msg = "Downloads could not start. Restart Slot and try again.";
                videos::log_line(root, msg);
                let _ = fs::write(
                    root.join(if name == "music-queue" {
                        "Music/error.txt"
                    } else {
                        ERROR_FILE
                    }),
                    msg,
                );
            }
        }
        SpawnChoice::Spawn => {
            // The old lock lived on the card. FAT32 does not lock it, so it is only in the way.
            let _ = fs::remove_file(root.join(if name == "music-queue" {
                "Music/downloader.lock"
            } else {
                "Videos/downloader.lock"
            }));
            let Ok(exe) = std::env::current_exe() else {
                videos::log_line(root, "could not start the downloader");
                return;
            };
            let Some(dir) = root.to_str() else { return };
            match Command::new(exe)
                .args([flag, dir])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => {
                    gate.child = Some(child);
                    gate.told = false;
                    videos::log_line(root, "downloader started");
                }
                Err(e) => videos::log_line(root, &format!("downloader did not start: {e}")),
            }
        }
    }
}

fn be_quiet() {
    unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, 19);
    }
    let _ = Command::new("ionice")
        .args(["-c", "3", "-p", &std::process::id().to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_downloader_that_cannot_start_is_tried_a_few_times_and_then_stops() {
        let mut budget = SpawnBudget::new();
        let mut now = 0u64;
        let mut spawns = 0u32;
        for _ in 0..100 {
            match choose_spawn(&mut budget, now, false) {
                SpawnChoice::Spawn => {
                    spawns += 1;
                    now = budget.next_ms;
                }
                SpawnChoice::Wait => now = budget.next_ms,
                SpawnChoice::GiveUp => break,
            }
        }
        assert_eq!(spawns, MAX_SPAWN_TRIES);
        assert_eq!(
            choose_spawn(&mut budget, now + 1_000_000, false),
            SpawnChoice::GiveUp
        );
        assert!(budget.next_ms - 0 >= 5_000);
        assert_eq!(choose_spawn(&mut budget, 0, true), SpawnChoice::Wait);
        assert_eq!(budget.tries, 0);
    }

    #[test]
    fn the_queue_is_kept_and_downloads_stay_on_during_a_game() {
        let root = tempfile::tempdir().unwrap();
        assert!(!pause_in_games(root.path()));
        enqueue(root.path(), "abcdefghijk", "Latest\tepisode", "Retro", 1000).unwrap();
        let rows = jobs(root.path());
        assert_eq!(rows[0].title, "Latest episode");
        note_game(root.path(), true);
        assert!(!root.path().join(GAME_PAUSE).exists());
        set_pause_in_games(root.path(), true).unwrap();
        note_game(root.path(), true);
        assert!(root.path().join(GAME_PAUSE).is_file());
        note_game(root.path(), false);
        assert!(!root.path().join(GAME_PAUSE).exists());
        assert!(crate::net::is_again(&std::io::Error::new(
            std::io::ErrorKind::WouldBlock,
            "Resource temporarily unavailable"
        )));
        assert_eq!(kind_of("closed by peer"), "closed");
        assert_eq!(kind_of("server said 403"), "denied");
        assert_eq!(kind_of("Resource temporarily unavailable"), "again");
        assert_eq!(backoff_ms(1), 500);
        assert_eq!(backoff_ms(4), 4_000);
        assert!(backoff_ms(20) <= 30_000);
        assert!(TRIES >= 10);
        let part = part_path(root.path(), "abcdefghijk");
        fs::write(&part, b"partial").unwrap();
        // Opening Videos used to delete every partial. A queued one stays, so it can resume.
        crate::videos::scrub_parts(root.path());
        assert!(part.is_file());
        remove(root.path(), "abcdefghijk");
        assert!(jobs(root.path()).is_empty());
        assert!(!part.exists());
    }
}
