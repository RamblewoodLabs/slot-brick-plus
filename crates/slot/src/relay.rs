//! A Mac on the LAN fetches the video with yt-dlp. Slot copies the file, checks
//! it, and only then keeps it. See `tools/mac-relay/`.

use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use crate::net::{self, Watch};

/// Bonjour name of a Mac whose Computer Name is "Mac mini".
pub const DEFAULT_HOST: &str = "Mac-mini.local";
pub const DEFAULT_PORT: u16 = 8787;

const FILE: &str = "Config/videos.txt";
const CHUNK: u64 = 4 * 1024 * 1024;
const POLL: Duration = Duration::from_millis(400);
const QUIET: Duration = Duration::from_secs(180);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    pub token: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Video,
    Audio,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Video => "video",
            Kind::Audio => "audio",
        }
    }

    pub fn job_id(self, video_id: &str) -> String {
        format!("{}-{video_id}", self.name())
    }
}

#[derive(Debug)]
pub struct Ready {
    pub id: String,
    pub size: u64,
    pub sha256: String,
}

/// None until `relay_token` is set in `Config/videos.txt`. A missing host is
/// [`DEFAULT_HOST`], and a missing port is [`DEFAULT_PORT`].
pub fn settings(root: &Path) -> Option<Settings> {
    let text = fs::read_to_string(root.join(FILE)).ok()?;
    let token = field(&text, "relay_token");
    if token.is_empty() || !token.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    let host = field(&text, "relay_host");
    let host = if host.is_empty() {
        DEFAULT_HOST.to_string()
    } else {
        host
    };
    if host.bytes().any(|b| !b.is_ascii_graphic()) || host.contains('/') {
        return None;
    }
    let port = match field(&text, "relay_port").parse::<u16>() {
        Ok(port) if port != 0 => port,
        Ok(_) => return None,
        Err(_) => DEFAULT_PORT,
    };
    Some(Settings { host, port, token })
}

fn field(text: &str, key: &str) -> String {
    let prefix = format!("{key}=");
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix))
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Download `video_id` into `part` and check the size and sha256. The caller
/// saves the file, then [`confirm`] deletes it on the Mac.
pub fn fetch_file(
    settings: &Settings,
    video_id: &str,
    kind: Kind,
    part: &Path,
    watch: &mut dyn FnMut() -> Watch,
    note: &mut dyn FnMut(&str, u64, u64),
) -> Result<Ready, String> {
    let ready = wait_ready(settings, video_id, kind, watch, note)?;
    transfer(settings, &ready, part, watch, note)?;
    let got = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    if got != ready.size {
        let _ = fs::remove_file(part);
        return Err(format!("relay size {got} of {}", ready.size));
    }
    let digest = sha256_file(part)?;
    if !digest.eq_ignore_ascii_case(&ready.sha256) {
        let _ = fs::remove_file(part);
        return Err("relay sha256 did not match".into());
    }
    Ok(ready)
}

/// DELETE /files/<id>, after the file has been checked and saved.
pub fn confirm(settings: &Settings, job_id: &str) -> Result<(), String> {
    let reply = call(settings, "DELETE", &format!("/files/{job_id}"), None)?;
    if matches!(reply.status, 200 | 204 | 404) {
        Ok(())
    } else {
        Err(format!("relay delete said {}", reply.status))
    }
}

fn wait_ready(
    settings: &Settings,
    video_id: &str,
    kind: Kind,
    watch: &mut dyn FnMut() -> Watch,
    note: &mut dyn FnMut(&str, u64, u64),
) -> Result<Ready, String> {
    let body = format!(
        "{{\"video_id\":\"{video_id}\",\"kind\":\"{}\"}}",
        kind.name()
    );
    let posted = call(settings, "POST", "/jobs", Some(&body))?;
    if posted.status != 202 && posted.status != 200 {
        return Err(format!("relay said {}", posted.status));
    }
    let text = String::from_utf8_lossy(&posted.body);
    let id = json_string(&text, "id").unwrap_or_else(|| kind.job_id(video_id));
    let mut last = 0u64;
    let mut moved = Instant::now();
    loop {
        match watch() {
            Watch::Stop => return Err("removed".into()),
            Watch::Pause => {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
            Watch::Go => {}
        }
        let reply = call(settings, "GET", &format!("/jobs/{id}"), None)?;
        if reply.status != 200 {
            return Err(format!("relay job said {}", reply.status));
        }
        let text = String::from_utf8_lossy(&reply.body);
        let status = json_string(&text, "status").unwrap_or_default();
        let bytes = json_u64(&text, "bytes").unwrap_or(0);
        let total = json_u64(&text, "total").unwrap_or(0);
        match status.as_str() {
            "ready" => {
                let size = json_u64(&text, "size").unwrap_or(0);
                let sha256 = json_string(&text, "sha256").unwrap_or_default();
                if size == 0 || sha256.len() != 64 {
                    return Err("relay ready file had no checksum".into());
                }
                return Ok(Ready { id, size, sha256 });
            }
            "failed" => {
                let err = json_string(&text, "error").unwrap_or_else(|| "failed".into());
                return Err(format!("relay {err}"));
            }
            _ => {
                if bytes != last {
                    last = bytes;
                    moved = Instant::now();
                } else if moved.elapsed() > QUIET {
                    return Err("relay made no progress".into());
                }
                note("Relay downloading…", bytes, total);
                thread::sleep(POLL);
            }
        }
    }
}

fn transfer(
    settings: &Settings,
    ready: &Ready,
    part: &Path,
    watch: &mut dyn FnMut() -> Watch,
    note: &mut dyn FnMut(&str, u64, u64),
) -> Result<(), String> {
    let url = format!(
        "http://{}:{}/files/{}",
        settings.host, settings.port, ready.id
    );
    let mut have = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    while have < ready.size {
        match watch() {
            Watch::Stop => return Err("removed".into()),
            Watch::Pause => {
                thread::sleep(Duration::from_millis(200));
                continue;
            }
            Watch::Go => {}
        }
        let end = have
            .saturating_add(CHUNK)
            .saturating_sub(1)
            .min(ready.size - 1);
        let ranged = match net::get_range_polled_bearer(
            &url,
            have,
            Some(end),
            net::IDLE_LIMIT,
            watch,
            &settings.token,
        ) {
            Err(e) if e == "paused" => continue,
            other => other?,
        };
        if ranged.from_start && have > 0 {
            return Err("relay ignored the range".into());
        }
        if let Some(sha) = &ranged.sha256 {
            if !sha.eq_ignore_ascii_case(&ready.sha256) {
                return Err("relay sha256 did not match".into());
            }
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
        let mut since = 0u64;
        let got = net::copy_body(
            &mut *reader,
            want,
            net::IDLE_LIMIT,
            watch,
            &mut |bytes| {
                file.write_all(bytes).map_err(|e| e.to_string())?;
                since += bytes.len() as u64;
                if since >= 64 * 1024 {
                    thread::sleep(Duration::from_millis(20));
                    since = 0;
                }
                Ok(())
            },
            &mut |got| note("Transferring…", start + got, ready.size),
        )?;
        file.sync_all().map_err(|e| e.to_string())?;
        let end_at = start + got;
        if end_at <= have {
            return Err("relay sent nothing".into());
        }
        have = end_at;
        note("Transferring…", have, ready.size);
    }
    Ok(())
}

fn call(
    settings: &Settings,
    method: &str,
    path: &str,
    json: Option<&str>,
) -> Result<net::Response, String> {
    let server = format!("{}:{}", settings.host, settings.port);
    net::send(&net::Request {
        tls: false,
        server: &server,
        method,
        path,
        form: None,
        json,
        agent: None,
        bearer: Some(&settings.token),
        limit: 1 << 20,
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA256);
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        ctx.update(&buf[..n]);
    }
    Ok(ctx
        .finish()
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn json_string(body: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let at = body.find(&pat)?;
    let rest = body[at + pat.len()..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    if rest.starts_with("null") {
        return None;
    }
    let rest = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for c in rest.chars() {
        if escaped {
            out.push(c);
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '"' => return Some(out),
            _ => out.push(c),
        }
    }
    None
}

fn json_u64(body: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\"");
    let at = body.find(&pat)?;
    let rest = body[at + pat.len()..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        None
    } else {
        digits.parse().ok()
    }
}

#[cfg(test)]
pub(crate) fn serve_ready(body: Vec<u8>) -> u16 {
    serve_claimed(body, None)
}

#[cfg(test)]
pub(crate) fn serve_claimed(body: Vec<u8>, claimed: Option<String>) -> u16 {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    let sha = claimed.unwrap_or_else(|| sha256_bytes(&body));
    let state = Arc::new(Mutex::new(0u32));
    let deleted = Arc::new(Mutex::new(false));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let body_thread = body.clone();
    let sha_thread = sha.clone();
    std::thread::spawn(move || {
        for _ in 0..16 {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            let body = body_thread.clone();
            let sha = sha_thread.clone();
            let state = state.clone();
            let deleted = deleted.clone();
            std::thread::spawn(move || {
                let _ = sock.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = Vec::new();
                let mut tmp = [0u8; 1024];
                while !buf.windows(4).any(|w| w == b"\r\n\r\n") && buf.len() < 8192 {
                    match sock.read(&mut tmp) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    }
                }
                let req = String::from_utf8_lossy(&buf);
                if !req.contains("Authorization: Bearer secret") {
                    let _ = sock.write_all(http(401, "unauthorized", "{}"));
                    return;
                }
                let path = req.split_whitespace().nth(1).unwrap_or("/");
                let path = path.split('?').next().unwrap_or(path);
                if req.starts_with("POST /jobs") {
                    let _ = sock.write_all(http(
                        202,
                        "accepted",
                        "{\"id\":\"video-abcdefghijk\",\"status\":\"queued\",\"bytes\":0,\"total\":0,\"size\":0,\"sha256\":\"\",\"error\":\"\"}",
                    ));
                } else if path.starts_with("/jobs/") {
                    let n = {
                        let mut n = state.lock().unwrap();
                        *n += 1;
                        *n
                    };
                    let json = if n == 1 {
                        "{\"id\":\"video-abcdefghijk\",\"status\":\"downloading\",\"bytes\":4,\"total\":36,\"size\":0,\"sha256\":\"\",\"error\":\"\"}".to_string()
                    } else {
                        format!(
                            "{{\"id\":\"video-abcdefghijk\",\"status\":\"ready\",\"bytes\":{},\"total\":{},\"size\":{},\"sha256\":\"{sha}\",\"error\":\"\"}}",
                            body.len(),
                            body.len(),
                            body.len()
                        )
                    };
                    let _ = sock.write_all(http(200, "ok", &json));
                } else if req.starts_with("DELETE /files/") {
                    *deleted.lock().unwrap() = true;
                    let _ = sock.write_all(b"HTTP/1.0 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                } else if path.starts_with("/files/") {
                    if *deleted.lock().unwrap() {
                        let _ = sock.write_all(http(404, "missing", "{\"error\":\"not found\"}"));
                        return;
                    }
                    let start = req
                        .split("bytes=")
                        .nth(1)
                        .and_then(|s| s.split(['-', '\r']).next())
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0)
                        .min(body.len());
                    let end = if start == 0 {
                        10.min(body.len())
                    } else {
                        body.len()
                    };
                    let slice = if start >= end { &[] } else { &body[start..end] };
                    let head = format!(
                        "HTTP/1.0 206 Partial Content\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nX-Sha256: {sha}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        slice.len(),
                        start,
                        end.saturating_sub(1),
                        body.len()
                    );
                    let mut out = head.into_bytes();
                    out.extend_from_slice(slice);
                    let _ = sock.write_all(&out);
                } else {
                    let _ = sock.write_all(http(404, "missing", "{\"error\":\"not found\"}"));
                }
            });
        }
    });
    port
}

#[cfg(test)]
fn http(status: u16, reason: &str, body: &str) -> &'static [u8] {
    let owned = format!(
        "HTTP/1.0 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    // The response has to outlive the write. Leak the small test replies.
    Box::leak(owned.into_bytes().into_boxed_slice())
}

#[cfg(test)]
fn sha256_bytes(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_turns_the_relay_on_and_the_host_can_be_an_address() {
        let root = tempfile::tempdir().unwrap();
        assert!(settings(root.path()).is_none());
        fs::create_dir_all(root.path().join("Config")).unwrap();
        fs::write(
            root.path().join(FILE),
            "relay_token=abc123\nrelay_port=8787\n",
        )
        .unwrap();
        let cfg = settings(root.path()).unwrap();
        assert_eq!(cfg.host, DEFAULT_HOST);
        assert_eq!(cfg.port, 8787);
        assert_eq!(cfg.token, "abc123");
        fs::write(
            root.path().join(FILE),
            "relay_host=192.168.1.20\nrelay_port=9000\nrelay_token=abc123\n",
        )
        .unwrap();
        let cfg = settings(root.path()).unwrap();
        assert_eq!(cfg.host, "192.168.1.20");
        assert_eq!(cfg.port, 9000);
    }

    #[test]
    fn a_closed_port_is_an_error_the_download_can_leave() {
        let cfg = Settings {
            host: "127.0.0.1".into(),
            port: 1,
            token: "secret".into(),
        };
        let root = tempfile::tempdir().unwrap();
        let err = fetch_file(
            &cfg,
            "abcdefghijk",
            Kind::Video,
            &root.path().join("x.part"),
            &mut || Watch::Go,
            &mut |_, _, _| {},
        )
        .unwrap_err();
        assert!(
            err.contains("cannot connect") || err.contains("relay said"),
            "{err}"
        );
    }

    #[test]
    fn a_ready_file_is_resumed_in_ranges_and_checked() {
        let body = b"abcdefghijklmnopqrstuvwxyz0123456789".to_vec();
        let port = serve_ready(body.clone());
        let cfg = Settings {
            host: "127.0.0.1".into(),
            port,
            token: "secret".into(),
        };
        let root = tempfile::tempdir().unwrap();
        let part = root.path().join("x.part");
        let mut phases = Vec::new();
        let ready = fetch_file(
            &cfg,
            "abcdefghijk",
            Kind::Video,
            &part,
            &mut || Watch::Go,
            &mut |phase, _, _| phases.push(phase.to_string()),
        )
        .unwrap();
        assert_eq!(fs::read(&part).unwrap(), body);
        assert_eq!(ready.size, body.len() as u64);
        assert!(phases.iter().any(|p| p.contains("Relay downloading")));
        assert!(phases.iter().any(|p| p.contains("Transferring")));
        confirm(&cfg, &ready.id).unwrap();
        let again = call(&cfg, "GET", &format!("/files/{}", ready.id), None).unwrap();
        assert_eq!(again.status, 404);
    }

    #[test]
    fn a_checksum_that_does_not_match_is_not_kept() {
        let body = b"abcdefghijklmnopqrstuvwxyz0123456789".to_vec();
        let port = serve_claimed(body, Some("ab".repeat(32)));
        let cfg = Settings {
            host: "127.0.0.1".into(),
            port,
            token: "secret".into(),
        };
        let root = tempfile::tempdir().unwrap();
        let part = root.path().join("x.part");
        let err = fetch_file(
            &cfg,
            "abcdefghijk",
            Kind::Video,
            &part,
            &mut || Watch::Go,
            &mut |_, _, _| {},
        )
        .unwrap_err();
        assert!(err.contains("sha256"), "{err}");
        assert!(!part.exists());
    }
}
