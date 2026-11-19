//! A small HTTP/1.0 client over std's TcpStream, with TLS from rustls (ring, Mozilla's
//! roots built in, so nothing depends on the firmware's certificate store). HTTP/1.0
//! keeps bodies unchunked; one request per connection.
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// What every request says it is.
pub fn user_agent() -> String {
    format!(
        "SlotBrick/{} (Linux {}) rcheevos/{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        slot_cheevos::version()
    )
}

pub struct Request<'a> {
    pub tls: bool,
    /// host:port
    pub server: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    /// An application/x-www-form-urlencoded body, for POST.
    pub form: Option<&'a str>,
    /// A JSON body, for POST. Used only when `form` is absent.
    pub json: Option<&'a str>,
    /// Replaces Slot's own name on this request.
    pub agent: Option<&'a str>,
    /// The most body accepted.
    pub limit: usize,
}

pub struct Response {
    pub status: u16,
    head: String,
    pub body: Vec<u8>,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<String> {
        self.head.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.trim()
                .eq_ignore_ascii_case(name)
                .then(|| v.trim().to_string())
        })
    }
}

pub fn host_of(server: &str) -> &str {
    server.rsplit_once(':').map_or(server, |(h, _)| h)
}

/// `https://host[:port]/path` (or `http://`) as (tls, "host:port", "/path").
pub fn split_url(url: &str) -> Option<(bool, String, String)> {
    let (tls, rest) = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
        (Some(rest), _) => (true, rest),
        (None, Some(rest)) => (false, rest),
        _ => return None,
    };
    let (host, path) = rest
        .split_once('/')
        .map_or((rest, "/".to_string()), |(h, p)| (h, format!("/{p}")));
    if host.is_empty() {
        return None;
    }
    let server = match host.contains(':') {
        true => host.to_string(),
        false => format!("{host}:{}", if tls { 443 } else { 80 }),
    };
    Some((tls, server, path))
}

/// Request-line parts come from servers (redirects, badge URLs): a space or control
/// character in one would let it add headers or a second request.
fn plain(part: &str) -> bool {
    !part.is_empty() && part.bytes().all(|b| b.is_ascii_graphic())
}

pub fn send(req: &Request) -> Result<Response, String> {
    let host = host_of(req.server);
    if !(plain(req.method) && plain(req.server) && plain(req.path) && req.path.starts_with('/')) {
        return Err("bad request address".into());
    }
    let tcp = dial(req.server)?;
    let mut text = format!(
        "{} {} HTTP/1.0\r\nHost: {host}\r\nUser-Agent: {}\r\nConnection: close\r\n",
        req.method,
        req.path,
        req.agent.unwrap_or(&user_agent())
    );
    if let Some(form) = req.form {
        text.push_str(&format!(
            "Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{form}",
            form.len()
        ));
    } else if let Some(json) = req.json {
        text.push_str(&format!(
            "Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{json}",
            json.len()
        ));
    } else {
        text.push_str("\r\n");
    }
    let cap = req.limit as u64 + 64 * 1024;
    let raw = if req.tls {
        let name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|_| format!("bad host name {host}"))?;
        let conn = rustls::ClientConnection::new(tls_config(), name).map_err(|e| e.to_string())?;
        let mut stream = rustls::StreamOwned::new(conn, tcp);
        // One write: a request split over several packets can be answered and reset early.
        stream
            .write_all(text.as_bytes())
            .map_err(|e| tls_error(&e))?;
        read_all(stream, cap).map_err(|e| tls_error(&e))?
    } else {
        let mut tcp = tcp;
        tcp.write_all(text.as_bytes())
            .map_err(|e| format!("send failed: {e}"))?;
        read_all(tcp, cap).map_err(|e| format!("download failed: {e}"))?
    };
    parse(&raw, req.limit)
}

/// A read that comes back too soon (EAGAIN / WouldBlock, or a timeout reported that way)
/// is not a failure. Callers retry it.
pub fn is_again(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock
            | std::io::ErrorKind::TimedOut
            | std::io::ErrorKind::Interrupted
    ) || error.raw_os_error() == Some(11)
        || {
            let text = error.to_string().to_lowercase();
            text.contains("temporarily unavailable")
                || text.contains("would block")
                || text.contains("eagain")
        }
}

/// A name for the log: what kind of drop this was, not the whole OS sentence.
pub fn error_kind(error: &std::io::Error) -> &'static str {
    if is_again(error) {
        return "again";
    }
    let text = error.to_string().to_lowercase();
    if matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::UnexpectedEof
    ) || text.contains("closed")
        || text.contains("reset")
        || text.contains("broken pipe")
    {
        return "closed";
    }
    "io"
}

fn tune_keepalive(tcp: &TcpStream) {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = tcp.as_raw_fd();
        let yes: libc::c_int = 1;
        let idle: libc::c_int = 30;
        let gap: libc::c_int = 10;
        let n: libc::c_int = 3;
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_KEEPALIVE,
                &yes as *const _ as *const libc::c_void,
                4,
            );
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPIDLE,
                &idle as *const _ as *const libc::c_void,
                4,
            );
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPINTVL,
                &gap as *const _ as *const libc::c_void,
                4,
            );
            libc::setsockopt(
                fd,
                libc::IPPROTO_TCP,
                libc::TCP_KEEPCNT,
                &n as *const _ as *const libc::c_void,
                4,
            );
        }
    }
}

/// Read once, waiting through a brief EAGAIN. A socket timeout is not retried: callers
/// that need a longer wait loop themselves, so six timeouts cannot add up to three minutes.
pub fn read_soft(reader: &mut dyn Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut waits = 0u32;
    loop {
        match reader.read(buf) {
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read timed out",
                ));
            }
            Err(e) if is_again(&e) => {
                waits += 1;
                if waits >= 6 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "read timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(80));
            }
            other => return other,
        }
    }
}

/// What the download should do between reads. Pause and stop have to be seen during a
/// chunk, not only after it, or a hung read ignores Continue and Remove.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Watch {
    Go,
    Pause,
    Stop,
}

/// How long a download will wait, from the last byte, before the connection is abandoned.
pub const IDLE_LIMIT: Duration = Duration::from_secs(30);
/// Socket reads return this often so pause, remove, and the idle limit can be checked.
const READ_SLICE: Duration = Duration::from_millis(500);
/// A proxy this slow is abandoned so the next instance, or YouTube itself, can be tried.
const SLOW_BPS: u64 = 50 * 1024;

fn dial(server: &str) -> Result<TcpStream, String> {
    dial_in(server, Duration::from_secs(30))
}

fn dial_in(server: &str, read_timeout: Duration) -> Result<TcpStream, String> {
    let host = host_of(server);
    let addrs: Vec<_> = server
        .to_socket_addrs()
        .map_err(|_| format!("cannot look up {host} (no internet?)"))?
        .collect();
    let tcp = addrs
        .iter()
        .find_map(|a| TcpStream::connect_timeout(a, Duration::from_secs(8)).ok())
        .ok_or_else(|| format!("cannot connect to {host}"))?;
    let _ = tcp.set_read_timeout(Some(read_timeout));
    let _ = tcp.set_write_timeout(Some(Duration::from_secs(15)));
    tune_keepalive(&tcp);
    Ok(tcp)
}

/// A body being downloaded: its length, if the server said, and the bytes as they come.
pub struct Body {
    pub len: Option<u64>,
    pub reader: Box<dyn Read + Send>,
}

/// GET `url`, following redirects (a GitHub download is one, to its file server), with the
/// body left to be read as it arrives rather than held in memory. Only a 200 is a body.
pub fn get_stream(url: &str) -> Result<Body, String> {
    get_stream_as(url, None, &|_| true)
}

/// Like `get_stream`, but each address, including one a redirect points at, has to pass `allow`.
pub fn get_stream_allowed(url: &str, allow: &dyn Fn(&str) -> bool) -> Result<Body, String> {
    get_stream_as(url, None, allow)
}

fn get_stream_as(
    url: &str,
    agent: Option<&str>,
    allow: &dyn Fn(&str) -> bool,
) -> Result<Body, String> {
    let mut url = url.to_string();
    for _ in 0..5 {
        if !allow(&url) {
            return Err("Refusing a download from anywhere but this Slot's GitHub releases".into());
        }
        let (tls, server, path) = split_url(&url).ok_or("bad download address")?;
        let (status, head, reader) = open_get(
            tls,
            &server,
            &path,
            agent,
            "",
            Duration::from_secs(30),
            IDLE_LIMIT,
            &mut || Watch::Go,
        )?;
        let reply = Response {
            status,
            head,
            body: Vec::new(),
        };
        match status {
            200 => {
                let len = reply.header("content-length").and_then(|v| v.parse().ok());
                return Ok(Body { len, reader });
            }
            301 | 302 | 303 | 307 | 308 => {
                let to = reply.header("location").ok_or("bad redirect from server")?;
                url = match to.starts_with('/') {
                    true => format!("{}://{server}{to}", if tls { "https" } else { "http" }),
                    false => to,
                };
            }
            _ => return Err(format!("server said {status}")),
        }
    }
    Err("too many redirects".into())
}

/// A browser's name. YouTube's pages answer this; they often refuse Slot's own name.
pub const BROWSER: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36";

/// GET `url` whole, following redirects; at most `limit` bytes.

pub fn get(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    get_as(url, limit, None)
}

/// GET `url` whole, saying `agent` instead of Slot's own name.
pub fn get_as(url: &str, limit: usize, agent: Option<&str>) -> Result<Vec<u8>, String> {
    let body = get_stream_as(url, agent, &|_| true)?;
    if body.len.is_some_and(|n| n > limit as u64) {
        return Err("reply too large".into());
    }
    let mut out = Vec::new();
    match body.reader.take(limit as u64 + 1).read_to_end(&mut out) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !out.is_empty() => {}
        Err(e) => return Err(format!("download failed: {e}")),
    }
    if out.len() > limit {
        return Err("reply too large".into());
    }
    if body.len.is_some_and(|n| (out.len() as u64) < n) {
        return Err("download cut short".into());
    }
    Ok(out)
}

/// Sends a GET and reads up to the end of its headers: the status, the headers, and the
/// connection, at the body's first byte.
/// A GET that asks for the bytes from `start` onward. A 206 is the tail. A 200 means the
/// server sent the whole file from the beginning, so the caller should replace what it has.
pub struct Ranged {
    pub from_start: bool,
    pub status: u16,
    /// The whole file, from Content-Range, when the server sent one.
    pub total: Option<u64>,
    /// Bytes in this reply, from Content-Length.
    pub content_length: Option<u64>,
    pub reader: Box<dyn Read + Send>,
}

pub fn get_ranged(url: &str, start: u64) -> Result<Ranged, String> {
    get_range(url, start, None)
}

/// `end` is inclusive, as in `Range: bytes=start-end`. `None` asks for the rest of the file.
pub fn get_range(url: &str, start: u64, end: Option<u64>) -> Result<Ranged, String> {
    range_request(
        url,
        start,
        end,
        Duration::from_secs(30),
        IDLE_LIMIT,
        &mut || Watch::Go,
    )
}

/// A ranged GET whose reads return often. `idle` is the longest stretch with no bytes,
/// covering connect, headers, and a body that has gone quiet. `watch` is checked between
/// those reads so pause and remove are not stuck behind a hung socket.
pub fn get_range_polled(
    url: &str,
    start: u64,
    end: Option<u64>,
    idle: Duration,
    watch: &mut dyn FnMut() -> Watch,
) -> Result<Ranged, String> {
    range_request(url, start, end, READ_SLICE, idle, watch)
}

fn range_request(
    url: &str,
    start: u64,
    end: Option<u64>,
    read_timeout: Duration,
    idle: Duration,
    watch: &mut dyn FnMut() -> Watch,
) -> Result<Ranged, String> {
    let mut url = url.to_string();
    let range = match end {
        Some(end) => format!("Range: bytes={start}-{end}\r\n"),
        None => format!("Range: bytes={start}-\r\n"),
    };
    for _ in 0..5 {
        let (tls, server, path) = split_url(&url).ok_or("bad download address")?;
        let (status, head, reader) =
            open_get(tls, &server, &path, None, &range, read_timeout, idle, watch)?;
        let reply = Response {
            status,
            head,
            body: Vec::new(),
        };
        match status {
            206 => {
                let total = reply
                    .header("content-range")
                    .as_deref()
                    .and_then(range_total);
                let content_length = reply.header("content-length").and_then(|v| v.parse().ok());
                return Ok(Ranged {
                    from_start: false,
                    status,
                    total,
                    content_length,
                    reader,
                });
            }
            200 => {
                let content_length = reply.header("content-length").and_then(|v| v.parse().ok());
                return Ok(Ranged {
                    from_start: true,
                    status,
                    total: content_length,
                    content_length,
                    reader,
                });
            }
            403 => return Err("denied: server said 403".into()),
            416 => return Err("already complete".into()),
            301 | 302 | 303 | 307 | 308 => {
                let to = reply.header("location").ok_or("bad redirect from server")?;
                url = match to.starts_with('/') {
                    true => format!("{}://{server}{to}", if tls { "https" } else { "http" }),
                    false => to,
                };
            }
            _ => return Err(format!("server said {status}")),
        }
    }
    Err("too many redirects".into())
}

fn range_total(header: &str) -> Option<u64> {
    let (_, total) = header.split_once('/')?;
    total.trim().parse().ok()
}

fn open_get(
    tls: bool,
    server: &str,
    path: &str,
    agent: Option<&str>,
    extra: &str,
    read_timeout: Duration,
    idle: Duration,
    watch: &mut dyn FnMut() -> Watch,
) -> Result<(u16, String, Box<dyn Read + Send>), String> {
    let host = host_of(server);
    if !(plain(server) && plain(path) && path.starts_with('/')) {
        return Err("bad request address".into());
    }
    match watch() {
        Watch::Pause => return Err("paused".into()),
        Watch::Stop => return Err("removed".into()),
        Watch::Go => {}
    }
    let started = Instant::now();
    let tcp = dial_in(server, read_timeout)?;
    let text = format!(
        "GET {path} HTTP/1.0\r\nHost: {host}\r\nUser-Agent: {}\r\n{extra}Connection: close\r\n\r\n",
        agent.unwrap_or(&user_agent())
    );
    let mut stream: Box<dyn Read + Send> = if tls {
        let name = rustls::pki_types::ServerName::try_from(host.to_string())
            .map_err(|_| format!("bad host name {host}"))?;
        let conn = rustls::ClientConnection::new(tls_config(), name).map_err(|e| e.to_string())?;
        let mut stream = rustls::StreamOwned::new(conn, tcp);
        stream
            .write_all(text.as_bytes())
            .map_err(|e| tls_error(&e))?;
        Box::new(stream)
    } else {
        let mut tcp = tcp;
        tcp.write_all(text.as_bytes())
            .map_err(|e| format!("send failed: {e}"))?;
        Box::new(tcp)
    };
    // A byte at a time, so nothing of the body is read here: the caller reads all of it.
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        if head.len() > 64 * 1024 {
            return Err("bad reply from server".into());
        }
        if started.elapsed() >= idle {
            return Err("stall: no bytes for 30s".into());
        }
        match watch() {
            Watch::Pause => return Err("paused".into()),
            Watch::Stop => return Err("removed".into()),
            Watch::Go => {}
        }
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            Ok(_) => return Err("bad reply from server".into()),
            Err(e) if is_again(&e) => continue,
            Err(e) if tls => return Err(tls_error(&e)),
            Err(e) => return Err(format!("download failed: {e}")),
        }
    }
    let head = String::from_utf8_lossy(&head[..head.len() - 4]).into_owned();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or("bad reply from server")?;
    Ok((status, head, stream))
}

/// Copy up to `want` bytes. Returns how many were written. A quiet socket, a slow
/// proxy, or pause/remove ends the copy instead of sitting in the read.
pub fn copy_body(
    reader: &mut dyn Read,
    want: u64,
    idle: Duration,
    watch: &mut dyn FnMut() -> Watch,
    write: &mut dyn FnMut(&[u8]) -> Result<(), String>,
    progress: &mut dyn FnMut(u64),
) -> Result<u64, String> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut got = 0u64;
    let mut since_note = 0u64;
    let started = Instant::now();
    let mut last_byte = Instant::now();
    let mut last_note = Instant::now();
    while got < want {
        if last_byte.elapsed() >= idle {
            return Err("stall: no bytes for 30s".into());
        }
        match watch() {
            Watch::Pause => return Err("paused".into()),
            Watch::Stop => return Err("removed".into()),
            Watch::Go => {}
        }
        if started.elapsed() >= Duration::from_secs(2) && got > 0 && got < want {
            let ms = started.elapsed().as_millis().max(1) as u64;
            if got.saturating_mul(1000) / ms < SLOW_BPS {
                return Err("slow: under 50 KB/s".into());
            }
        }
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if is_again(&e) => continue,
            Err(e) => return Err(format!("{}: {e}", error_kind(&e))),
        };
        last_byte = Instant::now();
        let n = n.min((want - got) as usize);
        write(&buf[..n])?;
        got += n as u64;
        since_note += n as u64;
        if since_note >= 256 * 1024 || last_note.elapsed() >= Duration::from_millis(500) {
            progress(got);
            since_note = 0;
            last_note = Instant::now();
        }
    }
    if got > 0 {
        progress(got);
    }
    Ok(got)
}

/// To the end of the stream. A server that closes TLS without a close_notify
/// (common) ends the read with UnexpectedEof; the reply is complete by then.
fn read_all(stream: impl Read, cap: u64) -> std::io::Result<Vec<u8>> {
    let mut raw = Vec::new();
    match stream.take(cap).read_to_end(&mut raw) {
        Ok(_) => Ok(raw),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof && !raw.is_empty() => Ok(raw),
        Err(e) => Err(e),
    }
}

/// `send`'s error for a server it could not reach at all (no internet, wrong network), as
/// opposed to one that answered badly: worth giving up a whole run over.
pub fn unreachable(error: &str) -> bool {
    error.starts_with("cannot look up ") || error.starts_with("cannot connect to ")
}

fn tls_error(e: &std::io::Error) -> String {
    let text = e.to_string();
    let lower = text.to_lowercase();
    // A certificate checked against a wrong clock: the Brick's clock is the usual cause.
    if lower.contains("notvalidyet") || lower.contains("not valid yet") || lower.contains("expired")
    {
        return "secure connection refused: check Date & Time in settings".into();
    }
    format!("secure connection failed: {text}")
}

fn tls_config() -> Arc<rustls::ClientConfig> {
    static CONFIG: OnceLock<Arc<rustls::ClientConfig>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            let roots = rustls::RootCertStore {
                roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
            };
            let provider = Arc::new(rustls::crypto::ring::default_provider());
            let config = rustls::ClientConfig::builder_with_provider(provider)
                .with_safe_default_protocol_versions()
                .expect("ring supports the default TLS versions")
                .with_root_certificates(roots)
                .with_no_client_auth();
            Arc::new(config)
        })
        .clone()
}

/// Status line, headers and body. A body shorter than its Content-Length is an error.
pub fn parse(raw: &[u8], limit: usize) -> Result<Response, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("bad reply from server")?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let body = raw[split + 4..].to_vec();
    let status: u16 = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or("bad reply from server")?;
    let reply = Response { status, head, body };
    if reply.body.len() > limit {
        return Err("reply too large".into());
    }
    if let Some(len) = reply
        .header("content-length")
        .and_then(|v| v.parse::<usize>().ok())
    {
        if reply.body.len() < len {
            return Err("download cut short".into());
        }
    }
    Ok(reply)
}

/// application/x-www-form-urlencoded value.
pub fn form_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_value_escapes_everything_but_unreserved() {
        assert_eq!(form_escape("p@ss word&x=1"), "p%40ss%20word%26x%3D1");
    }

    #[test]
    fn a_reply_shorter_than_its_length_is_refused() {
        assert!(parse(b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nabc", 100).is_err());
        let ok = parse(
            b"HTTP/1.1 401 No\r\nContent-Length: 3\r\nX-A: b\r\n\r\nabc",
            100,
        )
        .unwrap();
        assert_eq!(
            (ok.status, ok.body.as_slice(), ok.header("x-a").as_deref()),
            (401, &b"abc"[..], Some("b"))
        );
    }

    #[test]
    fn urls_split_into_server_and_path() {
        assert_eq!(
            split_url("https://a.org/x?y=1"),
            Some((true, "a.org:443".into(), "/x?y=1".into()))
        );
        assert_eq!(
            split_url("http://h:8080"),
            Some((false, "h:8080".into(), "/".into()))
        );
        assert_eq!(split_url("ftp://h/x"), None);
        assert_eq!(split_url("https:///x"), None);
    }

    #[test]
    fn a_request_line_cannot_carry_extra_headers() {
        for (server, path) in [
            ("h:80", "/a\r\nX: y"),
            ("h:80", "/a b"),
            ("h\r\n:80", "/"),
            ("h:80", "a"),
        ] {
            let req = Request {
                tls: false,
                server,
                method: "GET",
                path,
                form: None,
                json: None,
                agent: None,
                limit: 1,
            };
            assert_eq!(
                send(&req).err().as_deref(),
                Some("bad request address"),
                "{path:?}"
            );
        }
    }

    #[test]
    fn only_an_unreachable_server_counts_as_unreachable() {
        let req = |server| Request {
            tls: false,
            server,
            method: "GET",
            path: "/",
            form: None,
            json: None,
            agent: None,
            limit: 1,
        };
        assert!(unreachable(&send(&req("127.0.0.1:1")).err().unwrap()));
        assert!(unreachable(
            &send(&req("no-such-host.invalid:80")).err().unwrap()
        ));
        assert!(!unreachable("server said 403") && !unreachable("reply too large"));
    }

    #[test]
    fn a_wrong_clock_is_named_as_the_cause() {
        let e = std::io::Error::other("invalid peer certificate: NotValidYet");
        assert!(tls_error(&e).contains("Date & Time"));
    }

    /// `SLOT_LIVE_NET=1 ... live_tls -- --ignored`: a real HTTPS round trip.
    #[test]
    #[ignore = "needs internet"]
    fn live_tls() {
        if std::env::var_os("SLOT_LIVE_NET").is_none() {
            return;
        }
        let reply = send(&Request {
            tls: true,
            server: "retroachievements.org:443",
            method: "POST",
            path: "/dorequest.php",
            form: Some("r=login2"),
            json: None,
            agent: None,
            limit: 1 << 16,
        })
        .unwrap();
        assert_eq!(
            reply.status,
            422,
            "{}",
            String::from_utf8_lossy(&reply.body)
        );
    }
}
