//! Finding a YouTube channel and a video file, on the Brick, over Wi-Fi.
//!
//! Piped's public API is asked first: it returns plain JSON and, when it can, a
//! combined 360p MP4. YouTube's own Innertube player is the fallback, and it only
//! helps when the reply carries a direct address (a `url`, not a signature that
//! would need YouTube's player script). Sign-in is deliberately not implemented;
//! see [`auth_status`].
use std::path::Path;

use crate::net;

/// Public Piped instances, tried in order. `SLOT_PIPED` names one instead.
const PIPED: [&str; 3] = [
    "https://pipedapi.kavin.rocks",
    "https://pipedapi.adminforge.de",
    "https://api.piped.private.coffee",
];

/// A public Android client key, the same one the YouTube app sends. It is not a secret.
const INNERTUBE_KEY: &str = "AIzaSyA8eiZmM1FaDVjRy-df2KTyQ_vz_yYM39w";
const INNERTUBE: &str = "www.youtube.com:443";

/// The picture we ask for. 360p H.264 is comfortable for the H700's Cortex-A53 cores,
/// and Slot draws it into a 480×360 window on its 640×480 screen.
pub const MAX_HEIGHT: u32 = 360;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub subscribers: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Video {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub duration_ms: u64,
    /// A date or "3 days ago", when the service sent one.
    pub published: String,
}

/// A file to download. `audio_url` is set when the picture has no sound of its own
/// and a separate AAC stream was offered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub video_url: String,
    pub audio_url: Option<String>,
    pub height: u32,
    pub muxed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthState {
    /// No sign-in is stored. Search does not need one.
    SignedOut,
    /// Config/youtube-auth.txt holds a token a future Slot might use. This one does not.
    SavedUnused,
}

/// Why account login is not wired up, for the Account screen and the docs.
pub fn auth_reason() -> &'static str {
    "YouTube sign-in is not available. Subscriptions need a Google account, and a device-code login would need an OAuth client of Slot's own. A token from Google still does not unlock video addresses: those come from YouTube's player, which this screen does not log into. Search and downloads work without an account."
}

/// `Config/youtube-auth.txt` is reserved. A non-comment `token=` line means a later
/// Slot saved a sign-in. Nothing here writes one.
pub fn auth_status(root: &Path) -> AuthState {
    let Ok(text) = std::fs::read_to_string(root.join("Config/youtube-auth.txt")) else {
        return AuthState::SignedOut;
    };
    let saved = text.lines().any(|line| {
        let line = line.trim();
        !line.is_empty()
            && !line.starts_with('#')
            && line.starts_with("token=")
            && line.len() > "token=".len()
    });
    if saved {
        AuthState::SavedUnused
    } else {
        AuthState::SignedOut
    }
}

pub fn search_channels(query: &str) -> Result<Vec<Channel>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Type a channel name".into());
    }
    let mut errors = Vec::new();
    for base in piped_bases() {
        let url = format!("{base}/search?q={}&filter=channels", percent(query));
        match get_text(&url, 2 << 20) {
            Ok(body) => {
                let found = parse_piped_search(&body);
                if !found.is_empty() {
                    return Ok(found);
                }
                errors.push(format!("{base} returned no channels"));
            }
            Err(e) => errors.push(format!("{base}: {e}")),
        }
    }
    match innertube("search", &search_body(query)) {
        Ok(body) => {
            let found = parse_innertube_channels(&body);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube returned no channels".into());
        }
        Err(e) => errors.push(format!("YouTube: {e}")),
    }
    Err(errors
        .pop()
        .unwrap_or_else(|| "no video service answered".into()))
}

pub fn channel_videos(id: &str, name: &str) -> Result<Vec<Video>, String> {
    if !valid_id(id) && !id.starts_with("UC") {
        return Err("That channel address cannot be used".into());
    }
    let mut errors = Vec::new();
    for base in piped_bases() {
        let url = format!("{base}/channel/{}", percent(id));
        match get_text(&url, 3 << 20) {
            Ok(body) => {
                let found = parse_piped_channel(&body, name);
                if !found.is_empty() {
                    return Ok(found);
                }
                errors.push(format!("{base} listed no videos"));
            }
            Err(e) => errors.push(format!("{base}: {e}")),
        }
    }
    match innertube("browse", &browse_body(id)) {
        Ok(body) => {
            let found = parse_innertube_videos(&body, name);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube listed no videos".into());
        }
        Err(e) => errors.push(format!("YouTube: {e}")),
    }
    Err(errors
        .pop()
        .unwrap_or_else(|| "no video service answered".into()))
}

pub fn media_for(video_id: &str) -> Result<Media, String> {
    if !valid_id(video_id) {
        return Err("That video address cannot be used".into());
    }
    let mut errors = Vec::new();
    for base in piped_bases() {
        let url = format!("{base}/streams/{video_id}");
        match get_text(&url, 2 << 20) {
            Ok(body) => match parse_piped_streams(&body) {
                Some(media) => return Ok(media),
                None => errors.push(format!("{base} had no 360p MP4")),
            },
            Err(e) => errors.push(format!("{base}: {e}")),
        }
    }
    match innertube("player", &player_body(video_id)) {
        Ok(body) => match parse_innertube_player(&body) {
            Some(media) => return Ok(media),
            None => errors.push("YouTube sent no direct 360p address. It often hides those behind a signature this player cannot make.".into()),
        },
        Err(e) => errors.push(format!("YouTube: {e}")),
    }
    Err(errors
        .pop()
        .unwrap_or_else(|| "no video service answered".into()))
}

fn piped_bases() -> Vec<String> {
    match std::env::var("SLOT_PIPED") {
        Ok(one) if one.is_empty() => Vec::new(),
        Ok(one) => vec![one.trim_end_matches('/').to_string()],
        Err(_) => PIPED.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn get_text(url: &str, limit: usize) -> Result<String, String> {
    let bytes = net::get(url, limit)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn innertube(method: &str, body: &str) -> Result<String, String> {
    let path = format!("/youtubei/v1/{method}?key={INNERTUBE_KEY}&prettyPrint=false");
    let reply = net::send(&net::Request {
        tls: true,
        server: INNERTUBE,
        method: "POST",
        path: &path,
        form: None,
        json: Some(body),
        limit: 4 << 20,
    })?;
    if reply.status != 200 {
        return Err(format!("server said {}", reply.status));
    }
    Ok(String::from_utf8_lossy(&reply.body).into_owned())
}

fn search_body(query: &str) -> String {
    format!(
        r#"{{"context":{{"client":{{"clientName":"WEB","clientVersion":"2.20241126.01.00","hl":"en"}}}},"query":"{}","params":"EgIQAg=="}}"#,
        json_escape(query)
    )
}

fn browse_body(channel_id: &str) -> String {
    // The videos tab of a channel.
    format!(
        r#"{{"context":{{"client":{{"clientName":"WEB","clientVersion":"2.20241126.01.00","hl":"en"}}}},"browseId":"{}","params":"EgZ2aWRlb3PyBgQKAjoA"}}"#,
        json_escape(channel_id)
    )
}

fn player_body(video_id: &str) -> String {
    format!(
        r#"{{"context":{{"client":{{"clientName":"ANDROID","clientVersion":"19.17.34","androidSdkVersion":30,"hl":"en","gl":"US"}}}},"videoId":"{}","contentCheckOk":true,"racyCheckOk":true}}"#,
        json_escape(video_id)
    )
}

pub fn parse_piped_search(body: &str) -> Vec<Channel> {
    let Some(value) = parse_json(body) else {
        return Vec::new();
    };
    let items = match &value {
        J::Arr(items) => items.as_slice(),
        J::Obj(_) => value
            .get("items")
            .and_then(|v| match v {
                J::Arr(items) => Some(items.as_slice()),
                _ => None,
            })
            .unwrap_or(&[]),
        _ => &[],
    };
    let mut out = Vec::new();
    for item in items {
        let kind = item.get("type").and_then(J::as_str).unwrap_or("");
        if kind != "channel"
            && item.get("subscribers").is_none()
            && !item
                .get("url")
                .and_then(J::as_str)
                .unwrap_or("")
                .contains("/channel/")
        {
            continue;
        }
        let Some(url) = item.get("url").and_then(J::as_str) else {
            continue;
        };
        let Some(id) = url.rsplit('/').find(|s| s.starts_with("UC") || valid_id(s)) else {
            continue;
        };
        let name = item
            .get("name")
            .and_then(J::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let subscribers = match item.get("subscribers").and_then(J::as_u64) {
            Some(n) if n > 0 => format!("{n} subscribers"),
            _ => String::new(),
        };
        push_channel(&mut out, id, name, subscribers);
    }
    out
}

pub fn parse_piped_channel(body: &str, channel: &str) -> Vec<Video> {
    let Some(value) = parse_json(body) else {
        return Vec::new();
    };
    let Some(items) = value.get("relatedStreams").and_then(|v| match v {
        J::Arr(items) => Some(items),
        _ => None,
    }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let Some(url) = item.get("url").and_then(J::as_str) else {
            continue;
        };
        let Some(id) = watch_id(url) else {
            continue;
        };
        let title = item
            .get("title")
            .and_then(J::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if title.is_empty() {
            continue;
        }
        let duration_ms = item
            .get("duration")
            .and_then(J::as_u64)
            .unwrap_or(0)
            .saturating_mul(1000);
        let published = item
            .get("uploadedDate")
            .and_then(J::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        push_video(&mut out, id, title, channel, duration_ms, published);
    }
    out
}

pub fn parse_piped_streams(body: &str) -> Option<Media> {
    let value = parse_json(body)?;
    if value.get("error").is_some() {
        return None;
    }
    let videos = streams_of(value.get("videoStreams"));
    let audios = audios_of(value.get("audioStreams"));
    pick_media(&videos, &audios)
}

pub fn parse_innertube_channels(body: &str) -> Vec<Channel> {
    let Some(value) = parse_json(body) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    objects_with(&value, "channelId", &mut found);
    let mut out = Vec::new();
    for obj in found {
        let Some(id) = obj.get("channelId").and_then(J::as_str) else {
            continue;
        };
        let Some(name) = text_field(obj, "title").or_else(|| text_field(obj, "name")) else {
            continue;
        };
        let subscribers = text_field(obj, "subscriberCountText").unwrap_or_default();
        push_channel(&mut out, id, name, subscribers);
    }
    out
}

pub fn parse_innertube_videos(body: &str, channel: &str) -> Vec<Video> {
    let Some(value) = parse_json(body) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    objects_with(&value, "videoId", &mut found);
    let mut out = Vec::new();
    for obj in found {
        let Some(id) = obj.get("videoId").and_then(J::as_str) else {
            continue;
        };
        if !valid_id(id) {
            continue;
        }
        let Some(title) = text_field(obj, "title") else {
            continue;
        };
        let duration_ms = text_field(obj, "lengthText")
            .map(|t| clock_ms(&t))
            .unwrap_or(0);
        let published = text_field(obj, "publishedTimeText").unwrap_or_default();
        push_video(&mut out, id, title, channel, duration_ms, published);
    }
    out
}

pub fn parse_innertube_player(body: &str) -> Option<Media> {
    let value = parse_json(body)?;
    let data = value.get("streamingData")?;
    let mut videos = Vec::new();
    let mut audios = Vec::new();
    for key in ["formats", "adaptiveFormats"] {
        let Some(J::Arr(items)) = data.get(key) else {
            continue;
        };
        for item in items {
            let Some(url) = item.get("url").and_then(J::as_str) else {
                continue;
            };
            if url.is_empty() {
                continue;
            }
            let mime = item.get("mimeType").and_then(J::as_str).unwrap_or("");
            let height = item.get("height").and_then(J::as_u32).unwrap_or(0);
            if mime.starts_with("audio/") {
                audios.push(AudioOffer {
                    url: url.to_string(),
                    aac: mime.contains("mp4a") || mime.contains("aac"),
                    bitrate: item.get("bitrate").and_then(J::as_u32).unwrap_or(u32::MAX),
                });
            } else if mime.starts_with("video/") || height > 0 {
                let h264 = mime.contains("avc1")
                    || mime.contains("h264")
                    || (!mime.contains("vp9") && !mime.contains("av01") && mime.contains("mp4"));
                let muxed =
                    mime.contains("mp4a") || item.get("audioQuality").is_some() || key == "formats";
                videos.push(StreamOffer {
                    url: url.to_string(),
                    height,
                    h264,
                    muxed,
                });
            }
        }
    }
    pick_media(&videos, &audios)
}

struct StreamOffer {
    url: String,
    height: u32,
    h264: bool,
    muxed: bool,
}

struct AudioOffer {
    url: String,
    aac: bool,
    bitrate: u32,
}

fn streams_of(value: Option<&J>) -> Vec<StreamOffer> {
    let Some(J::Arr(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let url = item.get("url").and_then(J::as_str)?.to_string();
            if url.is_empty() {
                return None;
            }
            let mime = item.get("mimeType").and_then(J::as_str).unwrap_or("");
            let codec = item.get("codec").and_then(J::as_str).unwrap_or("");
            let quality = item.get("quality").and_then(J::as_str).unwrap_or("");
            let height = item
                .get("height")
                .and_then(J::as_u32)
                .unwrap_or_else(|| height_of(quality));
            let h264 = codec.contains("avc1")
                || codec.contains("h264")
                || mime.contains("avc1")
                || (!codec.contains("vp9") && !codec.contains("av01") && mime.contains("mp4"));
            let muxed = !item.get("videoOnly").and_then(J::as_bool).unwrap_or(true)
                || mime.contains("mp4a");
            Some(StreamOffer {
                url,
                height,
                h264,
                muxed,
            })
        })
        .collect()
}

fn audios_of(value: Option<&J>) -> Vec<AudioOffer> {
    let Some(J::Arr(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let url = item.get("url").and_then(J::as_str)?.to_string();
            if url.is_empty() {
                return None;
            }
            let mime = item.get("mimeType").and_then(J::as_str).unwrap_or("");
            let codec = item.get("codec").and_then(J::as_str).unwrap_or("");
            let aac = codec.contains("mp4a")
                || codec.contains("aac")
                || mime.contains("mp4a")
                || mime.contains("audio/mp4");
            let bitrate = item.get("bitrate").and_then(J::as_u32).unwrap_or(u32::MAX);
            Some(AudioOffer { url, aac, bitrate })
        })
        .collect()
}

fn pick_media(videos: &[StreamOffer], audios: &[AudioOffer]) -> Option<Media> {
    let fits = |s: &&StreamOffer| s.h264 && s.height > 0 && s.height <= MAX_HEIGHT;
    if let Some(best) = videos
        .iter()
        .filter(|s| s.muxed && fits(s))
        .max_by_key(|s| s.height)
    {
        return Some(Media {
            video_url: best.url.clone(),
            audio_url: None,
            height: best.height,
            muxed: true,
        });
    }
    let video = videos
        .iter()
        .filter(|s| !s.muxed && fits(s))
        .max_by_key(|s| s.height)?;
    let audio = audios
        .iter()
        .filter(|a| a.aac)
        .min_by_key(|a| a.bitrate)
        .or_else(|| audios.iter().min_by_key(|a| a.bitrate));
    Some(Media {
        video_url: video.url.clone(),
        audio_url: audio.map(|a| a.url.clone()),
        height: video.height,
        muxed: false,
    })
}

fn height_of(quality: &str) -> u32 {
    quality.trim().trim_end_matches('p').parse().unwrap_or(0)
}

fn push_channel(out: &mut Vec<Channel>, id: &str, name: String, subscribers: String) {
    if out.iter().any(|c| c.id == id) || out.len() >= 30 {
        return;
    }
    out.push(Channel {
        id: id.to_string(),
        name,
        subscribers,
    });
}

fn push_video(
    out: &mut Vec<Video>,
    id: &str,
    title: String,
    channel: &str,
    duration_ms: u64,
    published: String,
) {
    if !valid_id(id) || out.iter().any(|v| v.id == id) || out.len() >= 40 {
        return;
    }
    out.push(Video {
        id: id.to_string(),
        title,
        channel: channel.to_string(),
        duration_ms,
        published,
    });
}

/// The five newest videos a channel list gave us, in the order they arrived.
pub fn latest_five(videos: Vec<Video>) -> Vec<Video> {
    videos.into_iter().take(5).collect()
}

/// A channel link or id typed on the Brick. A video link is refused.
pub fn channel_ref(input: &str) -> Result<ChannelRef, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err("Type a channel link or id".into());
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("watch?") || lower.contains("/watch") || lower.contains("youtu.be/") {
        return Err("That is a video link. Paste the channel's own link.".into());
    }
    if let Some(id) = text
        .split(['/', '?', '&', ' '])
        .find(|part| part.starts_with("UC") && valid_channel_id(part))
    {
        return Ok(ChannelRef::Id(id.to_string()));
    }
    let name = text.trim_start_matches('@').trim();
    if name.len() < 2 || name.chars().any(|c| c.is_control()) {
        return Err("That channel link does not look right".into());
    }
    Ok(ChannelRef::Name(name.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelRef {
    Id(String),
    Name(String),
}

pub fn valid_channel_id(id: &str) -> bool {
    id.starts_with("UC")
        && (10..=40).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn watch_id(url: &str) -> Option<&str> {
    let id = url.split("v=").nth(1)?.split(['&', '#']).next()?;
    valid_id(id).then_some(id)
}

pub fn valid_id(id: &str) -> bool {
    let n = id.len();
    (6..=16).contains(&n)
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn clock_ms(text: &str) -> u64 {
    if let Ok(secs) = text.trim().parse::<u64>() {
        return secs.saturating_mul(1000);
    }
    let parts: Vec<u64> = text
        .split(':')
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    let secs = match parts.as_slice() {
        [m, s] => m * 60 + s,
        [h, m, s] => h * 3600 + m * 60 + s,
        _ => 0,
    };
    secs.saturating_mul(1000)
}

fn text_field(obj: &J, key: &str) -> Option<String> {
    let value = obj.get(key)?;
    match value {
        J::Str(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        J::Obj(_) => simple_or_runs(value),
        _ => None,
    }
}

fn simple_or_runs(value: &J) -> Option<String> {
    if let Some(text) = value.get("simpleText").and_then(J::as_str) {
        let text = text.trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    let J::Arr(runs) = value.get("runs")? else {
        return None;
    };
    let mut text = String::new();
    for run in runs {
        if let Some(part) = run.get("text").and_then(J::as_str) {
            text.push_str(part);
        }
    }
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

fn objects_with<'a>(value: &'a J, key: &str, out: &mut Vec<&'a J>) {
    match value {
        J::Obj(pairs) => {
            if pairs.iter().any(|(k, _)| k == key) {
                out.push(value);
            }
            for (_, child) in pairs {
                objects_with(child, key, out);
            }
        }
        J::Arr(items) => {
            for child in items {
                objects_with(child, key, out);
            }
        }
        _ => {}
    }
}

fn percent(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

#[derive(Debug)]
enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            J::Str(s) => Some(s),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            J::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn as_u32(&self) -> Option<u32> {
        self.as_u64().and_then(|n| u32::try_from(n).ok())
    }

    fn as_u64(&self) -> Option<u64> {
        match self {
            J::Num(n) if n.is_finite() && *n >= 0.0 && *n < (u64::MAX as f64) => Some(*n as u64),
            _ => None,
        }
    }
}

fn parse_json(text: &str) -> Option<J> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        depth: 0,
    };
    let value = p.value()?;
    Some(value)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: u32,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.skip();
        if self.i < self.s.len() && self.s[self.i] == c {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Option<J> {
        self.skip();
        if self.depth > 64 {
            return None;
        }
        let b = *self.s.get(self.i)?;
        match b {
            b'n' => self.lit(b"null", J::Null),
            b't' => self.lit(b"true", J::Bool(true)),
            b'f' => self.lit(b"false", J::Bool(false)),
            b'"' => self.string().map(J::Str),
            b'[' => self.array(),
            b'{' => self.object(),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn lit(&mut self, word: &[u8], value: J) -> Option<J> {
        if self.s.get(self.i..self.i + word.len()) == Some(word) {
            self.i += word.len();
            Some(value)
        } else {
            None
        }
    }

    fn number(&mut self) -> Option<J> {
        let start = self.i;
        if self.s[self.i] == b'-' {
            self.i += 1;
        }
        let digits = |p: &mut Self| {
            let at = p.i;
            while p.i < p.s.len() && p.s[p.i].is_ascii_digit() {
                p.i += 1;
            }
            p.i > at
        };
        if !digits(self) {
            return None;
        }
        if self.i < self.s.len() && self.s[self.i] == b'.' {
            self.i += 1;
            if !digits(self) {
                return None;
            }
        }
        if self.i < self.s.len() && matches!(self.s[self.i], b'e' | b'E') {
            self.i += 1;
            if self.i < self.s.len() && matches!(self.s[self.i], b'+' | b'-') {
                self.i += 1;
            }
            if !digits(self) {
                return None;
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        text.parse::<f64>().ok().map(J::Num)
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat(b'"') {
            return None;
        }
        let mut out = String::new();
        while self.i < self.s.len() {
            let c = self.s[self.i];
            if c == b'"' {
                self.i += 1;
                return Some(out);
            }
            if c == b'\\' {
                self.i += 1;
                let e = *self.s.get(self.i)?;
                self.i += 1;
                match e {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = self.s.get(self.i..self.i + 4)?;
                        self.i += 4;
                        let u = u32::from_str_radix(std::str::from_utf8(hex).ok()?, 16).ok()?;
                        out.push(char::from_u32(u).unwrap_or('\u{FFFD}'));
                    }
                    _ => return None,
                }
                continue;
            }
            if c < 128 {
                out.push(c as char);
                self.i += 1;
                continue;
            }
            let rest = std::str::from_utf8(&self.s[self.i..]).ok()?;
            let ch = rest.chars().next()?;
            self.i += ch.len_utf8();
            out.push(ch);
        }
        None
    }

    fn array(&mut self) -> Option<J> {
        if !self.eat(b'[') {
            return None;
        }
        let mut items = Vec::new();
        self.depth += 1;
        if self.eat(b']') {
            self.depth -= 1;
            return Some(J::Arr(items));
        }
        loop {
            items.push(self.value()?);
            if self.eat(b']') {
                self.depth -= 1;
                return Some(J::Arr(items));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }

    fn object(&mut self) -> Option<J> {
        if !self.eat(b'{') {
            return None;
        }
        let mut pairs = Vec::new();
        self.depth += 1;
        if self.eat(b'}') {
            self.depth -= 1;
            return Some(J::Obj(pairs));
        }
        loop {
            self.skip();
            let key = self.string()?;
            if !self.eat(b':') {
                return None;
            }
            pairs.push((key, self.value()?));
            if self.eat(b'}') {
                self.depth -= 1;
                return Some(J::Obj(pairs));
            }
            if !self.eat(b',') {
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn piped_search_and_channel_and_streams_pick_a_muxed_360p() {
        let channels = parse_piped_search(
            r#"[{"type":"channel","name":"Nintendo","url":"/channel/UCabc1234567","subscribers":1200},{"type":"video","name":"nope","url":"/watch?v=abcdefghijk"}]"#,
        );
        assert_eq!(channels.len(), 1);
        assert_eq!(channels[0].id, "UCabc1234567");
        assert_eq!(channels[0].subscribers, "1200 subscribers");

        let videos = parse_piped_channel(
            r#"{"name":"Nintendo","relatedStreams":[{"url":"/watch?v=abcdefghijk","title":"A game","duration":90,"uploaderName":"Nintendo"},{"url":"/watch?v=abcdefghijk","title":"again","duration":1}]}"#,
            "Nintendo",
        );
        assert_eq!(videos.len(), 1);
        assert_eq!(videos[0].duration_ms, 90_000);

        let media = parse_piped_streams(
            r#"{"title":"A game","videoStreams":[
                {"url":"https://cdn.example/hi.mp4","quality":"1080p","mimeType":"video/mp4","codec":"avc1","videoOnly":true,"height":1080},
                {"url":"https://cdn.example/360.mp4","quality":"360p","mimeType":"video/mp4","codec":"avc1.4d401e","videoOnly":false,"height":360},
                {"url":"https://cdn.example/vp9.webm","quality":"240p","mimeType":"video/webm","codec":"vp9","videoOnly":false,"height":240}
            ],"audioStreams":[{"url":"https://cdn.example/a.m4a","mimeType":"audio/mp4","codec":"mp4a.40.2","bitrate":128000}]}"#,
        )
        .unwrap();
        assert!(media.muxed);
        assert_eq!(media.height, 360);
        assert!(media.audio_url.is_none());
        assert!(media.video_url.contains("360"));
    }

    #[test]
    fn a_video_only_stream_keeps_the_smallest_aac() {
        let media = parse_piped_streams(
            r#"{"videoStreams":[
                {"url":"https://cdn.example/v.mp4","mimeType":"video/mp4","codec":"avc1","videoOnly":true,"height":240}
            ],"audioStreams":[
                {"url":"https://cdn.example/big.m4a","codec":"mp4a","bitrate":160000},
                {"url":"https://cdn.example/small.m4a","codec":"mp4a","bitrate":48000}
            ]}"#,
        )
        .unwrap();
        assert!(!media.muxed);
        assert_eq!(
            media.audio_url.as_deref(),
            Some("https://cdn.example/small.m4a")
        );
    }

    #[test]
    fn innertube_replies_yield_a_channel_a_video_and_itag_18() {
        let channels = parse_innertube_channels(
            r#"{"contents":{"sectionListRenderer":{"contents":[{"itemSectionRenderer":{"contents":[{"channelRenderer":{"channelId":"UCabc1234567","title":{"simpleText":"Nintendo"},"subscriberCountText":{"simpleText":"1M subscribers"}}}]}}]}}}"#,
        );
        assert_eq!(channels[0].name, "Nintendo");
        assert_eq!(channels[0].subscribers, "1M subscribers");

        let videos = parse_innertube_videos(
            r#"{"videoRenderer":{"videoId":"abcdefghijk","title":{"runs":[{"text":"A "},{"text":"game"}]},"lengthText":{"simpleText":"1:30"}}}"#,
            "Nintendo",
        );
        assert_eq!(videos[0].title, "A game");
        assert_eq!(videos[0].duration_ms, 90_000);

        let media = parse_innertube_player(
            r#"{"streamingData":{"formats":[{"itag":18,"mimeType":"video/mp4; codecs=\"avc1.42001E, mp4a.40.2\"","height":360,"url":"https://cdn.example/360.mp4"}],"adaptiveFormats":[{"itag":136,"mimeType":"video/mp4; codecs=\"avc1.4d401f\"","height":720,"url":"https://cdn.example/720.mp4"},{"itag":140,"mimeType":"audio/mp4; codecs=\"mp4a.40.2\"","bitrate":128000,"url":"https://cdn.example/a.m4a"}]}}"#,
        )
        .unwrap();
        assert!(media.muxed && media.height == 360);
        assert!(media.video_url.ends_with("360.mp4"));
    }

    #[test]
    fn a_signature_instead_of_a_url_is_not_a_stream_we_can_fetch() {
        assert!(parse_innertube_player(
            r#"{"streamingData":{"formats":[{"itag":18,"mimeType":"video/mp4","height":360,"signatureCipher":"s=abc&url=https%3A%2F%2Fexample"}]}}"#,
        )
        .is_none());
    }

    #[test]
    fn auth_file_is_a_hook_and_stays_unused() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(auth_status(d.path()), AuthState::SignedOut);
        std::fs::create_dir_all(d.path().join("Config")).unwrap();
        std::fs::write(d.path().join("Config/youtube-auth.txt"), "# reserved\n").unwrap();
        assert_eq!(auth_status(d.path()), AuthState::SignedOut);
        std::fs::write(
            d.path().join("Config/youtube-auth.txt"),
            "# reserved\ntoken=abc\n",
        )
        .unwrap();
        assert_eq!(auth_status(d.path()), AuthState::SavedUnused);
        assert!(auth_reason().contains("not available"));
    }

    #[test]
    fn a_channel_link_is_accepted_and_a_video_link_is_not() {
        assert_eq!(
            channel_ref("https://www.youtube.com/channel/UCabcdefghijklmnopqrstuv"),
            Ok(ChannelRef::Id("UCabcdefghijklmnopqrstuv".into()))
        );
        assert!(channel_ref("https://youtu.be/abcdefghijk").is_err());
        assert!(channel_ref("https://www.youtube.com/watch?v=abcdefghijk").is_err());
        assert_eq!(
            channel_ref("@Nintendo").unwrap(),
            ChannelRef::Name("Nintendo".into())
        );
        assert!(matches!(channel_ref("   "), Err(_)));
    }

    #[test]
    fn ids_reject_path_pieces() {
        assert!(valid_id("abcdefghijk"));
        assert!(!valid_id("../etc"));
        assert!(!valid_id("abc"));
        assert!(!valid_id("has space"));
    }
}
