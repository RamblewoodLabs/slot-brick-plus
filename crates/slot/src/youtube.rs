//! Finding a YouTube channel and a video file, on the Brick, over Wi-Fi.
//!
//! A typed name or `@handle` is resolved from YouTube's own pages. The latest
//! videos come from the channel's RSS feed, which needs no key. Piped and
//! Invidious are fallbacks for the list and for a direct file address. YouTube's
//! player is asked last, and only a direct `url` is used (not a signature).
use crate::net;

/// Public Piped instances, tried in order. The one that answered recently is first.
/// `SLOT_PIPED` names one instead.
const PIPED: [&str; 3] = [
    "https://api.piped.private.coffee",
    "https://pipedapi.adminforge.de",
    "https://pipedapi.leptons.xyz",
];

/// Public Invidious instances with an API, tried when Piped has no file address.
const INVIDIOUS: [&str; 3] = [
    "https://yt.chocolatemoo53.com",
    "https://invidious.nerdvpn.de",
    "https://invidious.tiekoetter.com",
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

/// Turn a name, `@handle`, or channel link into channels the player can follow.
pub fn find_channels(query: &str) -> Result<Vec<Channel>, String> {
    match channel_ref(query)? {
        ChannelRef::Id(id) => Ok(vec![Channel {
            id,
            name: query.trim().to_string(),
            subscribers: String::new(),
        }]),
        ChannelRef::Handle(handle) => match resolve_handle(&handle) {
            Ok(channel) => Ok(vec![channel]),
            Err(page) => match search_channels(&handle) {
                Ok(found) if !found.is_empty() => Ok(found),
                Ok(_) => Err(page),
                Err(search) => Err(format!("{page} · {search}")),
            },
        },
        ChannelRef::Name(name) => search_channels(&name),
    }
}

pub fn search_channels(query: &str) -> Result<Vec<Channel>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("Type a channel name".into());
    }
    let mut errors = Vec::new();
    let page = format!(
        "https://www.youtube.com/results?search_query={}&sp=EgIQAg%3D%3D",
        percent(query)
    );
    match get_text(&page, 2 << 20) {
        Ok(body) => {
            let found = parse_youtube_search(&body);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube's search page had no channels".into());
        }
        Err(e) => errors.push(format!("YouTube's search page: {e}")),
    }
    match innertube("search", &search_body(query)) {
        Ok(body) => {
            let found = parse_innertube_channels(&body);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube's search had no channels".into());
        }
        Err(e) => errors.push(format!("YouTube's search: {e}")),
    }
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
    Err(join_errors(&errors))
}

pub fn channel_videos(id: &str, name: &str) -> Result<Vec<Video>, String> {
    if !valid_channel_id(id) {
        return Err("That channel address cannot be used".into());
    }
    let mut errors = Vec::new();
    let feed = format!("https://www.youtube.com/feeds/videos.xml?channel_id={id}");
    match get_text(&feed, 1 << 20) {
        Ok(body) => {
            let found = parse_rss(&body, name);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube's video feed had no videos".into());
        }
        Err(e) => errors.push(format!("YouTube's video feed: {e}")),
    }
    match innertube("browse", &browse_body(id)) {
        Ok(body) => {
            let found = parse_innertube_videos(&body, name);
            if !found.is_empty() {
                return Ok(found);
            }
            errors.push("YouTube's channel page had no video titles".into());
        }
        Err(e) => errors.push(format!("YouTube's channel page: {e}")),
    }
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
    Err(join_errors(&errors))
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
    for base in invidious_bases() {
        let url = format!("{base}/api/v1/videos/{video_id}");
        match get_text(&url, 2 << 20) {
            Ok(body) => match parse_invidious_video(&body) {
                Some(media) => return Ok(media),
                None => errors.push(format!("{base} had no 360p MP4")),
            },
            Err(e) => errors.push(format!("{base}: {e}")),
        }
    }
    // IOS and ANDROID_VR still hand out progressive MP4 itags 18 and 22 with a
    // direct url. ANDROID often only offers a signature, which we do not decipher.
    for (client, body) in player_clients(video_id) {
        match innertube("player", &body) {
            Ok(body) => match progressive_mp4(&body).or_else(|| parse_innertube_player(&body)) {
                Some(media) => return Ok(media),
                None => errors.push(
                    player_failure(&body).unwrap_or_else(|| {
                        format!("{client} sent no direct MP4 address")
                    }),
                ),
            },
            Err(e) => errors.push(format!("{client}: {e}")),
        }
    }
    let watch = format!("https://www.youtube.com/watch?v={video_id}");
    match get_text(&watch, 2 << 20) {
        Ok(body) => match parse_watch_page(&body) {
            Some(media) => return Ok(media),
            None => errors.push("YouTube's watch page had no direct file address".into()),
        },
        Err(e) => errors.push(format!("YouTube's watch page: {e}")),
    }
    Err(join_errors(&errors))
}

fn resolve_handle(handle: &str) -> Result<Channel, String> {
    let url = format!("https://www.youtube.com/@{handle}");
    let html = get_text(&url, 2 << 20).map_err(|e| format!("@{handle}: {e}"))?;
    parse_handle_page(&html, handle)
        .ok_or_else(|| format!("YouTube's page for @{handle} had no channel id"))
}

fn piped_bases() -> Vec<String> {
    match std::env::var("SLOT_PIPED") {
        Ok(one) if one.is_empty() => Vec::new(),
        Ok(one) => vec![one.trim_end_matches('/').to_string()],
        Err(_) => PIPED.iter().map(|s| (*s).to_string()).collect(),
    }
}

fn invidious_bases() -> Vec<String> {
    INVIDIOUS.iter().map(|s| (*s).to_string()).collect()
}

fn get_text(url: &str, limit: usize) -> Result<String, String> {
    let bytes = net::get_as(url, limit, Some(net::BROWSER))?;
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
        agent: Some(net::BROWSER),
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

fn player_clients(video_id: &str) -> Vec<(&'static str, String)> {
    let id = json_escape(video_id);
    vec![
        (
            "IOS",
            format!(
                r#"{{"context":{{"client":{{"clientName":"IOS","clientVersion":"19.45.4","deviceMake":"Apple","deviceModel":"iPhone16,2","hl":"en","gl":"US"}}}},"videoId":"{id}","contentCheckOk":true,"racyCheckOk":true}}"#
            ),
        ),
        (
            "ANDROID_VR",
            format!(
                r#"{{"context":{{"client":{{"clientName":"ANDROID_VR","clientVersion":"1.60.19","deviceMake":"Oculus","deviceModel":"Quest 3","androidSdkVersion":32,"hl":"en","gl":"US"}}}},"videoId":"{id}","contentCheckOk":true,"racyCheckOk":true}}"#
            ),
        ),
        (
            "TV",
            format!(
                r#"{{"context":{{"client":{{"clientName":"TVHTML5_SIMPLY_EMBEDDED_PLAYER","clientVersion":"2.0","hl":"en","gl":"US"}}}},"videoId":"{id}","contentCheckOk":true,"racyCheckOk":true}}"#
            ),
        ),
        (
            "MWEB",
            format!(
                r#"{{"context":{{"client":{{"clientName":"MWEB","clientVersion":"2.20241202.07.00","hl":"en","gl":"US"}}}},"videoId":"{id}","contentCheckOk":true,"racyCheckOk":true}}"#
            ),
        ),
        (
            "ANDROID",
            format!(
                r#"{{"context":{{"client":{{"clientName":"ANDROID","clientVersion":"19.17.34","androidSdkVersion":30,"hl":"en","gl":"US"}}}},"videoId":"{id}","contentCheckOk":true,"racyCheckOk":true}}"#
            ),
        ),
    ]
}

/// One stream address in the rotation a download uses after a drop: Innertube clients
/// first (IOS, ANDROID_VR, the TV embed, MWEB, ANDROID), then Piped, then Invidious.
/// `None` when `index` is past the end.
pub fn media_at(video_id: &str, index: usize) -> Option<Result<(String, Media), String>> {
    if !valid_id(video_id) {
        return Some(Err("That video address cannot be used".into()));
    }
    let clients = player_clients(video_id);
    if let Some((name, body)) = clients.get(index) {
        return Some(match innertube("player", body) {
            Ok(body) => progressive_mp4(&body)
                .or_else(|| parse_innertube_player(&body))
                .map(|media| ((*name).to_string(), media))
                .ok_or_else(|| format!("{name} sent no direct MP4 address")),
            Err(e) => Err(format!("{name}: {e}")),
        });
    }
    let mut at = index - clients.len();
    for base in piped_bases() {
        if at == 0 {
            let url = format!("{base}/streams/{video_id}");
            let name = format!("Piped {base}");
            return Some(match get_text(&url, 2 << 20) {
                Ok(body) => parse_piped_streams(&body)
                    .map(|media| (name.clone(), media))
                    .ok_or_else(|| format!("{base} had no 360p MP4")),
                Err(e) => Err(format!("{base}: {e}")),
            });
        }
        at -= 1;
    }
    for base in invidious_bases() {
        if at == 0 {
            let url = format!("{base}/api/v1/videos/{video_id}");
            let name = format!("Invidious {base}");
            return Some(match get_text(&url, 2 << 20) {
                Ok(body) => parse_invidious_video(&body)
                    .map(|media| (name.clone(), media))
                    .ok_or_else(|| format!("{base} had no 360p MP4")),
                Err(e) => Err(format!("{base}: {e}")),
            });
        }
        at -= 1;
    }
    None
}

/// A direct progressive MP4 (itag 18, else 22) from a player response. Signature
/// ciphers are left alone: this scan only accepts a `"url"` field.
pub fn progressive_mp4(body: &str) -> Option<Media> {
    let text = body
        .replace("\\u0026", "&")
        .replace("\\u002F", "/")
        .replace("\\/", "/");
    let mut itag18 = None;
    let mut itag22 = None;
    let mut rest = text.as_str();
    while let Some(at) = rest.find("\"url\"") {
        rest = &rest[at + 5..];
        let Some(colon) = rest.find(':') else {
            break;
        };
        let val = rest[colon + 1..].trim_start();
        if !val.starts_with('"') {
            continue;
        }
        let val = &val[1..];
        let end = val.find('"').unwrap_or(val.len());
        let url = &val[..end];
        rest = &val[end..];
        if !url.contains("googlevideo.com/videoplayback") {
            continue;
        }
        if url.contains("itag=18") {
            itag18 = Some(url.to_string());
        } else if url.contains("itag=22") && itag22.is_none() {
            itag22 = Some(url.to_string());
        }
    }
    let (height, video_url) = if let Some(url) = itag18 {
        (360, url)
    } else {
        (720, itag22?)
    };
    Some(Media {
        video_url,
        audio_url: None,
        height,
        muxed: true,
    })
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
    let mut out = Vec::new();
    let mut found = Vec::new();
    objects_with(&value, "videoId", &mut found);
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
    let mut locks = Vec::new();
    objects_with(&value, "lockupViewModel", &mut locks);
    for obj in locks {
        let Some(vm) = obj.get("lockupViewModel") else {
            continue;
        };
        let Some(id) = vm.get("contentId").and_then(J::as_str) else {
            continue;
        };
        let Some(title) = vm
            .get("metadata")
            .and_then(|m| m.get("lockupMetadataViewModel"))
            .and_then(|m| m.get("title"))
            .and_then(|t| t.get("content"))
            .and_then(J::as_str)
        else {
            continue;
        };
        let duration_ms = badge_clock(vm);
        let published = recent_label(vm);
        push_video(
            &mut out,
            id,
            title.to_string(),
            channel,
            duration_ms,
            published,
        );
    }
    out
}

fn badge_clock(vm: &J) -> u64 {
    let mut found = Vec::new();
    objects_with(vm, "thumbnailBadgeViewModel", &mut found);
    for obj in found {
        let Some(text) = obj
            .get("thumbnailBadgeViewModel")
            .and_then(|b| b.get("text"))
            .and_then(J::as_str)
        else {
            continue;
        };
        if text.contains(':') {
            return clock_ms(text);
        }
    }
    0
}

fn recent_label(vm: &J) -> String {
    let mut found = Vec::new();
    objects_with(vm, "content", &mut found);
    let mut last = String::new();
    for obj in found {
        let Some(text) = obj.get("content").and_then(J::as_str) else {
            continue;
        };
        let text = text.trim();
        if text.contains("ago") || text.contains("Premier") {
            last = text.to_string();
        }
    }
    last
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
                let h264 = is_h264(mime);
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
            let h264 = is_h264(&format!("{codec} {mime}"));
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
    let audio = audios.iter().filter(|a| a.aac).min_by_key(|a| a.bitrate);
    Some(Media {
        video_url: video.url.clone(),
        audio_url: audio.map(|a| a.url.clone()),
        height: video.height,
        muxed: false,
    })
}

/// H.264 only. An MP4 container is not enough: itag 396 is AV1 in an MP4 box and will not play.
fn itag_num(item: &J) -> u32 {
    item.get("itag")
        .and_then(|v| v.as_u32().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
        .unwrap_or(0)
}

fn is_h264(text: &str) -> bool {
    let text = text.to_lowercase();
    if text.contains("av01") || text.contains("vp9") || text.contains("vp09") {
        return false;
    }
    text.contains("avc1") || text.contains("h264")
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

/// A channel link, `@handle`, name, or id typed on the Brick. A video link is refused.
pub fn channel_ref(input: &str) -> Result<ChannelRef, String> {
    let text = input.trim();
    if text.is_empty() {
        return Err("Type a channel name, @handle, or link".into());
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
    if let Some(handle) = handle_in(text) {
        return Ok(ChannelRef::Handle(handle));
    }
    let name = text.trim();
    if name.len() < 2 || name.chars().any(|c| c.is_control()) {
        return Err("That channel link does not look right".into());
    }
    Ok(ChannelRef::Name(name.to_string()))
}

fn handle_in(text: &str) -> Option<String> {
    let piece = text
        .split(['/', '?', '&', ' '])
        .find_map(|part| part.strip_prefix('@'))?;
    let handle = piece.trim();
    valid_handle(handle).then(|| handle.to_string())
}

fn valid_handle(handle: &str) -> bool {
    (2..=40).contains(&handle.len())
        && handle
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelRef {
    Id(String),
    Handle(String),
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

fn join_errors(errors: &[String]) -> String {
    if errors.is_empty() {
        return "no video service answered".into();
    }
    let mut ranked: Vec<&String> = errors.iter().collect();
    ranked.sort_by_key(|e| {
        let low = e.to_lowercase();
        if low.contains("date & time")
            || low.contains("certificate")
            || low.contains("secure connection")
        {
            0
        } else if low.contains("player") || low.contains("sign in") || low.contains("bot") {
            1
        } else {
            2
        }
    });
    let mut text = ranked
        .iter()
        .map(|e| e.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    if text.len() > 240 {
        let mut end = 237;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("...");
    }
    text
}

pub fn parse_youtube_search(html: &str) -> Vec<Channel> {
    let mut out = Vec::new();
    let key = "\"channelRenderer\":{\"channelId\":\"";
    let mut rest = html;
    while let Some(at) = rest.find(key) {
        rest = &rest[at + key.len()..];
        let Some(id_end) = rest.find('"') else { break };
        let id = &rest[..id_end];
        // 800 bytes, but not through the middle of a character such as 気.
        let window = &rest[..rest.floor_char_boundary(800.min(rest.len()))];
        let name = between(window, "\"simpleText\":\"", "\"").unwrap_or("");
        let handle = between(window, "\"canonicalBaseUrl\":\"/", "\"").unwrap_or("");
        if valid_channel_id(id) && !name.is_empty() {
            push_channel(&mut out, id, unescape(name), handle.to_string());
        }
        if out.len() >= 20 {
            break;
        }
    }
    out
}

pub fn parse_handle_page(html: &str, handle: &str) -> Option<Channel> {
    let id = between(
        html,
        "rel=\"canonical\" href=\"https://www.youtube.com/channel/",
        "\"",
    )
    .or_else(|| between(html, "youtube.com/channel/", "\""))?;
    let id = id.trim_end_matches('/');
    if !valid_channel_id(id) {
        return None;
    }
    let title = between(html, "<title>", "</title>").unwrap_or(handle);
    let name = title
        .trim()
        .trim_end_matches("- YouTube")
        .trim()
        .trim_end_matches("— YouTube")
        .trim();
    let name = if name.is_empty() || name.eq_ignore_ascii_case("youtube") {
        handle
    } else {
        name
    };
    Some(Channel {
        id: id.to_string(),
        name: unescape(name),
        subscribers: format!("@{handle}"),
    })
}

/// A playlist link or a bare `PL…` id. Playlists are how a lot of music is kept.
pub fn playlist_id(text: &str) -> Option<String> {
    let text = text.trim();
    if let Some(at) = text.find("list=") {
        let id = text[at + 5..]
            .split(['&', '#', ' ', '?'])
            .next()
            .unwrap_or("");
        return valid_playlist(id).then(|| id.to_string());
    }
    valid_playlist(text).then(|| text.to_string())
}

fn valid_playlist(id: &str) -> bool {
    (10..=80).contains(&id.len())
        && (id.starts_with("PL") || id.starts_with("OL") || id.starts_with("UU"))
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn playlist_videos(id: &str, name: &str) -> Result<Vec<Video>, String> {
    let Some(id) = playlist_id(id).or_else(|| valid_playlist(id).then(|| id.to_string())) else {
        return Err("That playlist address cannot be used".into());
    };
    let feed = format!("https://www.youtube.com/feeds/videos.xml?playlist_id={id}");
    let body = get_text(&feed, 1 << 20).map_err(|e| format!("YouTube's playlist: {e}"))?;
    let found = parse_rss(&body, name);
    if found.is_empty() {
        return Err("That playlist has no videos".into());
    }
    Ok(found)
}

/// Audio only: AAC 128k (itag 140) when it is there, otherwise Opus (itag 251).
pub fn pick_audio_url(body: &str) -> Option<(u32, String)> {
    let text = body
        .replace("\\u0026", "&")
        .replace("\\u002F", "/")
        .replace("\\/", "/");
    let mut aac = None;
    let mut opus = None;
    let mut rest = text.as_str();
    while let Some(at) = rest.find("https://") {
        rest = &rest[at..];
        let end = rest.find(['"', '\\', ' ']).unwrap_or(rest.len());
        let end = rest.floor_char_boundary(end);
        let url = &rest[..end];
        rest = &rest[end.max(1)..];
        if !url.contains("googlevideo.com/videoplayback") && !url.contains("mime=audio") {
            if !url.contains("itag=140") && !url.contains("itag=251") {
                continue;
            }
        }
        if url.contains("itag=140") {
            aac = Some(url.to_string());
        } else if url.contains("itag=251") && opus.is_none() {
            opus = Some(url.to_string());
        }
    }
    if let Some(url) = aac {
        Some((140, url))
    } else {
        opus.map(|url| (251, url))
    }
}

/// One audio address in the same client rotation as a video download.
pub fn audio_at(video_id: &str, index: usize) -> Option<Result<(String, String), String>> {
    if !valid_id(video_id) {
        return Some(Err("That video address cannot be used".into()));
    }
    let clients = player_clients(video_id);
    if let Some((name, body)) = clients.get(index) {
        return Some(match innertube("player", body) {
            Ok(body) => pick_audio_url(&body)
                .map(|(_, url)| ((*name).to_string(), url))
                .ok_or_else(|| format!("{name} sent no audio address")),
            Err(e) => Err(format!("{name}: {e}")),
        });
    }
    let mut at = index - clients.len();
    for base in piped_bases() {
        if at == 0 {
            let url = format!("{base}/streams/{video_id}");
            let name = format!("Piped {base}");
            return Some(match get_text(&url, 2 << 20) {
                Ok(body) => parse_piped_audio(&body)
                    .map(|audio| (name.clone(), audio))
                    .ok_or_else(|| format!("{base} had no audio")),
                Err(e) => Err(format!("{base}: {e}")),
            });
        }
        at -= 1;
    }
    None
}

fn parse_piped_audio(body: &str) -> Option<String> {
    let value = parse_json(body)?;
    let J::Arr(items) = value.get("audioStreams")? else {
        return None;
    };
    let mut best: Option<(u32, String)> = None;
    for item in items {
        let url = item.get("url").and_then(J::as_str)?.to_string();
        if url.is_empty() {
            continue;
        }
        let mime = item.get("mimeType").and_then(J::as_str).unwrap_or("");
        let rank = if mime.contains("mp4a") || mime.contains("audio/mp4") {
            0
        } else if mime.contains("opus") || mime.contains("webm") {
            1
        } else {
            2
        };
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, url));
        }
    }
    best.map(|(_, url)| url)
}

pub fn parse_rss(xml: &str, channel: &str) -> Vec<Video> {
    let mut out = Vec::new();
    for entry in xml.split("<entry>").skip(1) {
        let Some(id) = between(entry, "<yt:videoId>", "</yt:videoId>") else {
            continue;
        };
        let title = between(entry, "<title>", "</title>").unwrap_or("");
        let published = between(entry, "<published>", "</published>")
            .unwrap_or("")
            .chars()
            .take(10)
            .collect::<String>();
        push_video(&mut out, id, unescape(title), channel, 0, published);
    }
    out
}

pub fn parse_invidious_video(body: &str) -> Option<Media> {
    let value = parse_json(body)?;
    let mut videos = Vec::new();
    let mut audios = Vec::new();
    if let Some(J::Arr(items)) = value.get("formatStreams") {
        for item in items {
            let url = item.get("url").and_then(J::as_str)?.to_string();
            if url.is_empty() {
                continue;
            }
            let quality = item.get("qualityLabel").and_then(J::as_str).unwrap_or("");
            let encoding = item.get("encoding").and_then(J::as_str).unwrap_or("");
            let itag = itag_num(item);
            videos.push(StreamOffer {
                url,
                height: height_of(quality),
                h264: is_h264(encoding) || matches!(itag, 18 | 22),
                muxed: true,
            });
        }
    }
    if let Some(J::Arr(items)) = value.get("adaptiveFormats") {
        for item in items {
            let url = item.get("url").and_then(J::as_str)?.to_string();
            if url.is_empty() {
                continue;
            }
            let kind = item.get("type").and_then(J::as_str).unwrap_or("");
            let encoding = item.get("encoding").and_then(J::as_str).unwrap_or("");
            let itag = itag_num(item);
            if kind.starts_with("audio/") {
                audios.push(AudioOffer {
                    url,
                    aac: encoding.contains("mp4a") || itag == 140,
                    bitrate: item.get("bitrate").and_then(J::as_u32).unwrap_or(u32::MAX),
                });
            } else {
                let quality = item.get("qualityLabel").and_then(J::as_str).unwrap_or("");
                videos.push(StreamOffer {
                    url,
                    height: height_of(quality),
                    h264: is_h264(&format!("{encoding} {kind}")) || matches!(itag, 134 | 135 | 136),
                    muxed: false,
                });
            }
        }
    }
    pick_media(&videos, &audios)
}

pub fn parse_watch_page(html: &str) -> Option<Media> {
    let text = html.replace("\\u0026", "&").replace("\\/", "/");
    let mut best: Option<(u32, String)> = None;
    let mut rest = text.as_str();
    while let Some(at) = rest.find("https://") {
        rest = &rest[at..];
        let end = rest.find(['"', '\\', ' ']).unwrap_or(rest.len());
        let url = &rest[..end];
        rest = &rest[end.max(1)..];
        if !url.contains("googlevideo.com/videoplayback") {
            continue;
        }
        let itag = url
            .split("itag=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        // 18 is the muxed 360p H.264 file. 22 is 720p, which is more than we play.
        let height = match itag {
            18 => 360,
            36 => 240,
            17 => 144,
            _ => 0,
        };
        if height == 0 || height > MAX_HEIGHT {
            continue;
        }
        if best.as_ref().is_none_or(|(h, _)| height > *h) {
            best = Some((height, url.to_string()));
        }
    }
    best.map(|(height, video_url)| Media {
        video_url,
        audio_url: None,
        height,
        muxed: true,
    })
}

fn player_failure(body: &str) -> Option<String> {
    let value = parse_json(body)?;
    let status = value.get("playabilityStatus")?;
    let name = status.get("status").and_then(J::as_str).unwrap_or("");
    if name.is_empty() || name == "OK" {
        return None;
    }
    let reason = status
        .get("reason")
        .and_then(J::as_str)
        .unwrap_or("")
        .trim();
    Some(match reason.is_empty() {
        true => format!("YouTube's player said {name}"),
        false => format!("YouTube's player: {reason}"),
    })
}

fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let at = text.find(start)? + start.len();
    let rest = &text[at..];
    let stop = rest.find(end)?;
    Some(&rest[..stop])
}

fn unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("\\u0026", "&")
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
    fn an_av1_mp4_is_not_used_when_a_muxed_h264_exists() {
        let body = r#"{"formatStreams":[
            {"url":"https://ex/videoplayback?itag=396","qualityLabel":"360p","encoding":"av01","container":"mp4","itag":"396"},
            {"url":"https://ex/videoplayback?itag=18","qualityLabel":"360p","encoding":"avc1.4d401e","container":"mp4","itag":"18"}
        ]}"#;
        let media = parse_invidious_video(body).unwrap();
        assert!(media.video_url.contains("itag=18"));
        assert!(media.muxed);
        assert!(media.audio_url.is_none());
    }

    #[test]
    fn video_only_av1_is_refused_and_h264_is_muxed_with_aac() {
        let only = r#"{"adaptiveFormats":[
            {"url":"https://ex/videoplayback?itag=396","type":"video/mp4","encoding":"av01","qualityLabel":"360p","itag":"396"}
        ]}"#;
        assert!(parse_invidious_video(only).is_none());
        let pair = r#"{"adaptiveFormats":[
            {"url":"https://ex/videoplayback?itag=134","type":"video/mp4","encoding":"avc1.4d401e","qualityLabel":"360p","itag":"134"},
            {"url":"https://ex/audio?itag=140","type":"audio/mp4","encoding":"mp4a.40.2","itag":"140","bitrate":128000}
        ]}"#;
        let media = parse_invidious_video(pair).unwrap();
        assert!(media.video_url.contains("itag=134"));
        assert!(!media.muxed);
        assert!(media.audio_url.unwrap().contains("itag=140"));
    }

    #[test]
    fn audio_prefers_aac_then_opus() {
        let aac = pick_audio_url(
            r#"{"url":"https://rr.googlevideo.com/videoplayback?itag=251&x=1","url":"https://rr.googlevideo.com/videoplayback?itag=140&x=2"}"#,
        );
        assert_eq!(aac.unwrap().0, 140);
        let opus =
            pick_audio_url(r#"{"url":"https://rr.googlevideo.com/videoplayback?itag=251&x=1"}"#);
        assert_eq!(opus.unwrap().0, 251);
        assert!(
            pick_audio_url(r#"{"url":"https://rr.googlevideo.com/videoplayback?itag=18"}"#)
                .is_none()
        );
    }

    #[test]
    fn a_playlist_link_keeps_only_the_id() {
        assert_eq!(
            playlist_id("https://www.youtube.com/playlist?list=PLabc_def-12&si=x"),
            Some("PLabc_def-12".into())
        );
        assert!(playlist_id("not a playlist").is_none());
    }

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
    fn a_search_snippet_does_not_split_a_multibyte_character() {
        let key = "\"channelRenderer\":{\"channelId\":\"";
        let mut rest = String::from("UCabcdefghij\"\"simpleText\":\"気になる\"");
        while rest.len() < 799 {
            rest.push('x');
        }
        rest.push('気');
        let html = format!("{key}{rest}");
        let found = parse_youtube_search(&html);
        assert!(found.iter().any(|c| c.id == "UCabcdefghij"));
    }

    #[test]
    fn a_direct_itag_18_is_kept_and_a_cipher_is_not() {
        let media = progressive_mp4(
            r#"{"streamingData":{"formats":[{"itag":22,"signatureCipher":"s=abc\u0026url=https%3A%2F%2Frr.googlevideo.com%2Fvideoplayback%3Fitag%3D22"},{"itag":18,"url":"https://rr.googlevideo.com/videoplayback?itag=18\u0026id=1"},{"itag":22,"url":"https://rr.googlevideo.com/videoplayback?itag=22\u0026id=2"}]}}"#,
        )
        .unwrap();
        assert_eq!(media.height, 360);
        assert!(media.muxed);
        assert!(media.video_url.contains("itag=18"));
        assert!(media.audio_url.is_none());
        let only22 = progressive_mp4(
            r#"{"formats":[{"itag":22,"url":"https://rr.googlevideo.com/videoplayback?itag=22&id=2"}]}"#,
        )
        .unwrap();
        assert_eq!(only22.height, 720);
    }

    #[test]
    fn a_signature_instead_of_a_url_is_not_a_stream_we_can_fetch() {
        assert!(parse_innertube_player(
            r#"{"streamingData":{"formats":[{"itag":18,"mimeType":"video/mp4","height":360,"signatureCipher":"s=abc&url=https%3A%2F%2Fexample"}]}}"#,
        )
        .is_none());
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
            channel_ref("@breakingpoints").unwrap(),
            ChannelRef::Handle("breakingpoints".into())
        );
        assert_eq!(
            channel_ref("https://www.youtube.com/@RetroGameCorps/videos").unwrap(),
            ChannelRef::Handle("RetroGameCorps".into())
        );
        assert_eq!(
            channel_ref("Breaking Points").unwrap(),
            ChannelRef::Name("Breaking Points".into())
        );
        assert!(matches!(channel_ref("   "), Err(_)));
    }

    #[test]
    fn a_search_page_a_handle_page_and_a_feed_become_channels_and_videos() {
        let html = r#""channelRenderer":{"channelId":"UCDRIjKy6eZOvKtOELtTdeUA","title":{"simpleText":"Breaking Points"},"browseEndpoint":{"browseId":"UCDRIjKy6eZOvKtOELtTdeUA","canonicalBaseUrl":"/@breakingpoints"}}"#;
        let found = parse_youtube_search(html);
        assert_eq!(found[0].name, "Breaking Points");
        assert_eq!(found[0].subscribers, "@breakingpoints");
        let page = r#"<title>Breaking Points - YouTube</title><link rel="canonical" href="https://www.youtube.com/channel/UCDRIjKy6eZOvKtOELtTdeUA">"#;
        let channel = parse_handle_page(page, "breakingpoints").unwrap();
        assert_eq!(channel.id, "UCDRIjKy6eZOvKtOELtTdeUA");
        assert_eq!(channel.name, "Breaking Points");
        let rss = r#"<feed><entry><yt:videoId>8fVV8qauHRc</yt:videoId><title>Stocks &amp; PLUNGE</title><published>2026-11-18T17:00:09+00:00</published></entry></feed>"#;
        let videos = parse_rss(rss, "Breaking Points");
        assert_eq!(videos[0].id, "8fVV8qauHRc");
        assert_eq!(videos[0].title, "Stocks & PLUNGE");
        assert_eq!(videos[0].published, "2026-11-18");
    }

    #[test]
    fn a_lockup_and_a_watch_page_url_are_readable() {
        let body = r#"{"lockupViewModel":{"contentId":"abcdefghijk","metadata":{"lockupMetadataViewModel":{"title":{"content":"A game"}}},"contentImage":{"thumbnailBadgeViewModel":{"text":"1:30"}}}}"#;
        let videos = parse_innertube_videos(body, "Band");
        assert_eq!(videos[0].title, "A game");
        assert_eq!(videos[0].duration_ms, 90_000);
        let page = r#"<script>var u="https://rr.googlevideo.com/videoplayback?itag=18\u0026id=1";</script>"#;
        let media = parse_watch_page(page).unwrap();
        assert!(media.muxed && media.height == 360);
        assert!(media.video_url.contains("itag=18&id=1"));
    }

    #[test]
    #[ignore = "needs internet"]
    fn live_handles_and_feeds_list_real_videos() {
        let points = find_channels("@breakingpoints").unwrap();
        assert_eq!(points[0].id, "UCDRIjKy6eZOvKtOELtTdeUA");
        assert!(points[0].name.to_ascii_lowercase().contains("breaking"));
        let videos = channel_videos(&points[0].id, &points[0].name).unwrap();
        assert!(videos.len() >= 5, "{videos:?}");
        assert!(videos.iter().all(|v| valid_id(&v.id)));
        let corps = find_channels("Retro Game Corps").unwrap();
        assert!(
            corps.iter().any(|c| c.id == "UCoZQiN0o7f36H7PaW4fVhFw"),
            "{corps:?}"
        );
        let typed = find_channels("Breaking Point").unwrap();
        assert!(
            typed.iter().any(|c| c.name == "Breaking Points"),
            "{typed:?}"
        );
        let zone = find_channels("@RetroGamingCore").unwrap();
        assert_eq!(zone[0].id, "UCFKxT8oaisBZMTYTUgMHDUQ", "{zone:?}");
        let listed = channel_videos(&zone[0].id, &zone[0].name).unwrap();
        assert!(!listed.is_empty(), "{listed:?}");
        match media_for(&videos[0].id) {
            Ok(media) => {
                let got = crate::net::get(&media.video_url, 256 * 1024).expect("file bytes");
                assert!(got.len() > 1000, "short file {}", got.len());
            }
            Err(e) => assert!(
                e.to_lowercase().contains("player")
                    || e.to_lowercase().contains("sign in")
                    || e.to_lowercase().contains("bot")
                    || e.to_lowercase().contains("direct"),
                "the failure should name YouTube's answer, not a bare 'no videos': {e}"
            ),
        }
    }

    #[test]
    fn ids_reject_path_pieces() {
        assert!(valid_id("abcdefghijk"));
        assert!(!valid_id("../etc"));
        assert!(!valid_id("abc"));
        assert!(!valid_id("has space"));
    }
}
