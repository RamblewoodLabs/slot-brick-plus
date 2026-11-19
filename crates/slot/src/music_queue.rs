//! Audio downloads for Music. Same chunked, resumable path as videos, audio only.
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::music::fat_name;
use crate::videos;

const QUEUE: &str = "Music/queue.txt";
const TRIES: u32 = 10;
const RANGE_CHUNK: u64 = 4 * 1024 * 1024;
const READ: usize = 64 * 1024;

#[derive(Clone)]
struct Job {
    id: String,
    title: String,
    channel: String,
}

pub fn push(root: &Path, id: &str, title: &str, channel: &str) {
    let mut rows = jobs(root);
    if rows.iter().any(|job| job.id == id) {
        return;
    }
    rows.push(Job {
        id: id.to_string(),
        title: title.to_string(),
        channel: channel.to_string(),
    });
    let _ = write_jobs(root, &rows);
    videos::log_line(root, &format!("music queued {id} {title}"));
    ensure(root);
}

pub fn downloading(root: &Path) -> bool {
    !jobs(root).is_empty()
}

pub fn ensure(root: &Path) {
    if cfg!(test) {
        return;
    }
    crate::video_queue::consider_spawn(
        root,
        "music-queue",
        "--music-queue",
        !jobs(root).is_empty(),
    );
}

pub fn run(root: &Path) {
    let root = root.to_path_buf();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(_lock) = crate::video_queue::claim_downloader("music-queue", "--music-queue") else {
            return;
        };
        be_quiet();
        videos::log_line(&root, "music downloader running");
        loop {
            let Some(job) = jobs(&root).into_iter().next() else {
                videos::log_line(&root, "music queue empty");
                return;
            };
            match fetch(&root, &job) {
                Ok(()) => {}
                Err(e) => {
                    videos::log_line(&root, &format!("music failed {} {e}", job.id));
                    thread::sleep(Duration::from_secs(8));
                }
            }
        }
    }));
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

fn fetch(root: &Path, job: &Job) -> Result<(), String> {
    if over_limit(root) {
        return Err("The video and music storage limit is full".into());
    }
    let dest = track_path(root, &job.channel, &job.title);
    if let Some(dir) = dest.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let part = dest.with_extension("m4a.part");
    let mut source = 0usize;
    let mut url = next_url(&job.id, &mut source)?;
    let mut tries = 0u32;
    loop {
        let have = fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        let end = have.saturating_add(RANGE_CHUNK).saturating_sub(1);
        match pull(&url.1, &part, have, end) {
            Ok((end_at, status, content_length)) if end_at > have => {
                let got = end_at - have;
                let asked = end.saturating_sub(have).saturating_add(1);
                videos::log_line(
                    root,
                    &format!(
                        "music chunk {} client {} itag {} http {status} content-length {content_length:?} bytes {end_at}",
                        job.id, url.0, itag(&url.1)
                    ),
                );
                let ended = got < asked && content_length.is_some_and(|n| got >= n);
                if ended {
                    fs::rename(&part, &dest).map_err(|e| e.to_string())?;
                    let _ = cover(root, &job.id, &dest);
                    let mut rows = jobs(root);
                    rows.retain(|row| row.id != job.id);
                    write_jobs(root, &rows).map_err(|e| e.to_string())?;
                    videos::log_line(root, &format!("music saved {} {}", job.id, dest.display()));
                    return Ok(());
                }
                if got >= asked {
                    tries = 0;
                    continue;
                }
                tries += 1;
                videos::log_line(
                    root,
                    &format!(
                        "music chunk {} client {} itag {} at {have} try {tries}/{TRIES} closed: short of the chunk",
                        job.id, url.0, itag(&url.1)
                    ),
                );
                if tries % 3 == 0 {
                    source += 1;
                    if let Ok(fresh) = next_url(&job.id, &mut source) {
                        url = fresh;
                    }
                }
                if tries >= TRIES {
                    return Err(crate::video_queue::GIVE_UP.into());
                }
                thread::sleep(Duration::from_millis(crate::video_queue::backoff_ms(tries)));
            }
            other => {
                let detail = match &other {
                    Ok(_) => "no progress".to_string(),
                    Err(e) => e.clone(),
                };
                if detail.contains("already complete") && part.is_file() {
                    fs::rename(&part, &dest).map_err(|e| e.to_string())?;
                    let _ = cover(root, &job.id, &dest);
                    let mut rows = jobs(root);
                    rows.retain(|row| row.id != job.id);
                    write_jobs(root, &rows).map_err(|e| e.to_string())?;
                    videos::log_line(root, &format!("music saved {} {}", job.id, dest.display()));
                    return Ok(());
                }
                tries += 1;
                let kind = if detail.to_lowercase().contains("403") {
                    "denied"
                } else if detail.to_lowercase().contains("closed")
                    || detail.to_lowercase().contains("timed out")
                    || detail.to_lowercase().contains("temporarily unavailable")
                    || detail.contains("os error 11")
                {
                    "again"
                } else {
                    "io"
                };
                videos::log_line(
                    root,
                    &format!(
                        "music chunk {} client {} itag {} at {have} try {tries}/{TRIES} {kind}: {detail}",
                        job.id, url.0, itag(&url.1)
                    ),
                );
                if kind == "denied" || tries % 3 == 0 {
                    source += 1;
                    if let Ok(fresh) = next_url(&job.id, &mut source) {
                        url = fresh;
                    }
                }
                if tries >= TRIES {
                    return Err(crate::video_queue::GIVE_UP.into());
                }
                thread::sleep(Duration::from_millis(crate::video_queue::backoff_ms(tries)));
            }
        }
    }
}

fn next_url(id: &str, source: &mut usize) -> Result<(String, String), String> {
    let start = *source;
    loop {
        match crate::youtube::audio_at(id, *source) {
            Some(Ok(pair)) => return Ok(pair),
            Some(Err(_)) => {
                *source += 1;
                if *source > start + 12 {
                    return Err("No audio address".into());
                }
            }
            None => {
                if start == 0 {
                    return Err("No audio address".into());
                }
                *source = 0;
                return Err("No audio address".into());
            }
        }
    }
}

fn pull(url: &str, part: &Path, have: u64, end: u64) -> Result<(u64, u16, Option<u64>), String> {
    let ranged = crate::net::get_range(url, have, Some(end))?;
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
    let mut last_byte = std::time::Instant::now();
    while got < want {
        if last_byte.elapsed() >= crate::net::IDLE_LIMIT {
            return Err("stall: no bytes for 30s".into());
        }
        let n = match crate::net::read_soft(&mut *reader, &mut buf) {
            Ok(0) => break,
            Ok(n) => {
                last_byte = std::time::Instant::now();
                n
            }
            Err(e) if crate::net::is_again(&e) => continue,
            Err(e) => {
                if ranged.content_length.is_some_and(|n| got >= n)
                    || ranged.total.is_some_and(|n| start + got >= n)
                {
                    break;
                }
                return Err(format!("{}: {e}", crate::net::error_kind(&e)));
            }
        };
        let n = n.min((want - got) as usize);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        thread::sleep(Duration::from_millis(20));
    }
    file.sync_all().map_err(|e| e.to_string())?;
    Ok((start + got, ranged.status, ranged.content_length))
}

fn cover(root: &Path, id: &str, dest: &Path) -> bool {
    let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
    let Ok(jpeg) = crate::net::get(&url, 1 << 20) else {
        return false;
    };
    let Some(png) = videos::jpeg_to_png(&jpeg) else {
        return false;
    };
    let _ = root;
    fs::write(dest.with_extension("png"), png).is_ok()
}

fn over_limit(root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(root.join("Config/videos.txt")) else {
        return false;
    };
    let limit: u64 = text
        .lines()
        .find_map(|line| line.strip_prefix("limit_mb=")?.trim().parse().ok())
        .unwrap_or(0);
    if limit == 0 {
        return false;
    }
    bytes_under(&root.join("Videos")) + bytes_under(&root.join("Music")) > limit * 1024 * 1024
}

fn bytes_under(dir: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += bytes_under(&path);
        } else {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

fn itag(url: &str) -> &str {
    url.split("itag=")
        .nth(1)
        .and_then(|s| s.split(['&', '#']).next())
        .unwrap_or("-")
}

pub fn track_path(root: &Path, channel: &str, title: &str) -> PathBuf {
    root.join("Music")
        .join(fat_name(channel))
        .join(format!("{}.m4a", fat_name(title)))
}

fn jobs(root: &Path) -> Vec<Job> {
    let Ok(text) = fs::read_to_string(root.join(QUEUE)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let mut f = line.split('\t');
            Some(Job {
                id: f.next()?.to_string(),
                title: f.next()?.to_string(),
                channel: f.next()?.to_string(),
            })
        })
        .collect()
}

fn write_jobs(root: &Path, rows: &[Job]) -> std::io::Result<()> {
    let mut text = String::new();
    for job in rows {
        text.push_str(&job.id);
        text.push('\t');
        text.push_str(&job.title.replace(['\t', '\n'], " "));
        text.push('\t');
        text.push_str(&job.channel.replace(['\t', '\n'], " "));
        text.push('\n');
    }
    let _ = fs::create_dir_all(root.join("Music"));
    slot_store::atomic_write(&root.join(QUEUE), text.as_bytes())
}
