//! ScreenScraper as a source of cart labels, for every shelf. Off unless switched on.
//!
//! For a cart with no label, the game is looked up once (`jeuInfos`, by the ROM's name, and its
//! CRC32 and size when that is cheap to know), then its pictures are fetched one at a time in
//! the order the system's profile asks for (label scan, cart photo, box, logo) and handed to
//! `slot_cartart`, which turns them into the label the shelf draws. The first picture that
//! makes a good label wins, so most games cost one lookup and one download. The label is
//! written to `Labels/<system>/<game>.png` like every other: this runs at fetch time only.
//!
//! Switched on by `Config/screenscraper.txt` holding `cartart=on` and the player's own
//! ScreenScraper account (`user=`, `password=`). The app's developer id comes from the build
//! (`SLOT_SS_DEVID`, `SLOT_SS_DEVPASSWORD`) or, for a home build, from `devid=` and
//! `devpassword=` in the same file; without one there is no lookup at all.
//!
//! Every request carries the credentials in its query string, so nothing here ever logs a
//! URL: errors pass through `redact` first, and a media address is kept only with the
//! credentials taken out.
use std::path::Path;
use std::time::{Duration, Instant};

use slot_cartart::Source;
use slot_store::{Cart, Platform};

pub const CONFIG: &str = "Config/screenscraper.txt";
const API: &str = "https://api.screenscraper.fr";
const SOFTNAME: &str = "Slotted";
/// One request at a time, a little apart: developer access allows one thread.
const PAUSE: Duration = Duration::from_millis(1200);
const JSON_MAX: usize = 4 << 20;
const MEDIA_MAX: usize = 8 << 20;
/// Pictures are asked for no larger than this: plenty for a label, a fraction of the bytes.
const MEDIA_WIDTH: u32 = 1000;
/// A label's longer side at most, as for the other sources (`cart_art::LABEL_MAX`).
const LABEL_MAX: u32 = 512;
/// The ROM's CRC32 is sent only when it is quick to read.
const CRC_MAX: u64 = 64 << 20;
const AUTH_KEYS: [&str; 5] = ["devid", "devpassword", "softname", "ssid", "sspassword"];

#[derive(Clone)]
pub struct Creds {
    devid: String,
    devpassword: String,
    user: String,
    password: String,
}

impl Creds {
    /// The account, when ScreenScraper art is switched on and a developer id is known.
    pub fn load(root: &Path) -> Option<Creds> {
        let text = std::fs::read_to_string(root.join(CONFIG)).ok()?;
        Self::parse(
            &text,
            option_env!("SLOT_SS_DEVID"),
            option_env!("SLOT_SS_DEVPASSWORD"),
        )
    }

    fn parse(text: &str, built_id: Option<&str>, built_pw: Option<&str>) -> Option<Creds> {
        let get = |key: &str| {
            text.lines()
                .filter(|l| !l.trim_start().starts_with('#'))
                .filter_map(|l| l.split_once('='))
                .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
                .map(|(_, v)| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        let on = get("cartart").is_some_and(|v| {
            matches!(v.to_ascii_lowercase().as_str(), "on" | "1" | "yes" | "true")
        });
        if !on {
            return None;
        }
        let devid = built_id
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .or_else(|| get("devid"))?;
        let devpassword = built_pw
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .or_else(|| get("devpassword"))?;
        Some(Creds {
            devid,
            devpassword,
            user: get("user").unwrap_or_default(),
            password: get("password").unwrap_or_default(),
        })
    }

    fn query(&self) -> String {
        let mut q = format!(
            "devid={}&devpassword={}&softname={SOFTNAME}",
            enc(&self.devid),
            enc(&self.devpassword)
        );
        if !self.user.is_empty() {
            q.push_str(&format!(
                "&ssid={}&sspassword={}",
                enc(&self.user),
                enc(&self.password)
            ));
        }
        q
    }
}

/// `text` with every credential's value replaced, for a log or an error shown on screen.
pub fn redact(text: &str) -> String {
    let mut out = text.to_string();
    for key in AUTH_KEYS {
        let pat = format!("{key}=");
        let mut at = 0;
        while let Some(i) = out[at..].find(&pat) {
            let start = at + i + pat.len();
            let end = out[start..]
                .find(|c: char| c == '&' || c.is_whitespace() || c == '"')
                .map_or(out.len(), |e| start + e);
            out.replace_range(start..end, "***");
            at = start + 3;
        }
    }
    out
}

fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// ScreenScraper's number for each shelf.
pub fn system_id(platform: Platform) -> u32 {
    match platform {
        Platform::Gba => 12,
        Platform::Gb => 9,
        Platform::Gbc => 10,
        Platform::Nes => 3,
        Platform::Snes => 4,
        Platform::Ps1 => 57,
        Platform::Md => 1,
        Platform::Sms => 2,
        Platform::Gg => 21,
        Platform::Arcade => 75,
        Platform::Pce => 31,
        Platform::A2600 => 26,
        Platform::Intv => 115,
        Platform::Lynx => 28,
        Platform::A7800 => 41,
        Platform::Gw => 52,
        Platform::Coleco => 48,
        Platform::A5200 => 40,
        Platform::A800 => 43,
        Platform::Vectrex => 102,
        Platform::S32x => 19,
        Platform::N64 => 14,
        Platform::Nds => 15,
        Platform::Psp => 61,
    }
}

/// One picture ScreenScraper has of a game; `url` has the credentials taken out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Media {
    pub kind: String,
    pub region: String,
    pub url: String,
}

/// The `medias` of a `jeuInfos` reply. A small scanner, not a JSON parser: each media is a flat
/// object of string fields.
pub fn parse_medias(json: &str) -> Vec<Media> {
    let Some(at) = json.find("\"medias\"") else {
        return Vec::new();
    };
    let rest = &json[at..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = None;
    let mut in_str = false;
    let mut esc = false;
    for (i, c) in rest[open + 1..].char_indices() {
        if in_str {
            match (esc, c) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '{' => {
                if depth == 0 {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(s) = start.take() {
                        let obj = &rest[open + 1 + s..open + 1 + i + 1];
                        if let (Some(kind), Some(url)) = (field(obj, "type"), field(obj, "url")) {
                            out.push(Media {
                                kind,
                                region: field(obj, "region").unwrap_or_default(),
                                url: strip_auth(&url),
                            });
                        }
                    }
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    out
}

/// A string field of a flat JSON object, unescaped.
fn field(obj: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let at = obj.find(&pat)? + pat.len();
    let rest = obj[at..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?).unwrap_or('?'));
                }
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            },
            c => out.push(c),
        }
    }
    None
}

/// The address without any credential in its query.
fn strip_auth(url: &str) -> String {
    let Some((base, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let kept: Vec<&str> = query
        .split('&')
        .filter(|kv| !AUTH_KEYS.iter().any(|k| kv.split('=').next() == Some(k)))
        .collect();
    format!("{base}?{}", kept.join("&"))
}

/// The regions to prefer for a ROM, from the tags in its name, then ScreenScraper's defaults.
pub fn regions(stem: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let tags: String = stem.split('(').skip(1).collect::<Vec<_>>().join(" ");
    for (word, region) in [
        ("USA", "us"),
        ("World", "wor"),
        ("Europe", "eu"),
        ("Japan", "jp"),
    ] {
        if tags.contains(word) {
            out.push(region);
        }
    }
    for region in ["us", "wor", "ss", "eu", "jp"] {
        if !out.contains(&region) {
            out.push(region);
        }
    }
    out
}

fn pick<'a>(medias: &'a [Media], kind: &str, prefer: &[&str]) -> Option<&'a Media> {
    let of: Vec<&Media> = medias.iter().filter(|m| m.kind == kind).collect();
    prefer
        .iter()
        .find_map(|r| of.iter().find(|m| m.region == *r).copied())
        .or(of.first().copied())
}

/// What a lookup gave.
pub enum Got {
    /// The label (PNG) and how it was made, for the report.
    Label(Vec<u8>, String),
    /// ScreenScraper doesn't know the game.
    NoMatch,
    /// It does, but none of its pictures makes a label.
    NoArt(String),
}

/// Talks to ScreenScraper one request at a time.
pub struct Client {
    creds: Creds,
    last: Option<Instant>,
    api: String,
}

impl Client {
    pub fn new(creds: Creds) -> Self {
        Client {
            creds,
            last: None,
            api: API.to_string(),
        }
    }

    #[cfg(test)]
    fn at(creds: Creds, api: &str) -> Self {
        Client {
            creds,
            last: None,
            api: api.to_string(),
        }
    }

    fn get(&mut self, url: &str, limit: usize) -> Result<(u16, Vec<u8>), String> {
        if let Some(last) = self.last {
            let since = last.elapsed();
            if since < PAUSE {
                std::thread::sleep(PAUSE - since);
            }
        }
        let result = (|| {
            let (tls, server, path) = crate::net::split_url(url).ok_or("bad address")?;
            // Only ScreenScraper's own hosts are ever sent the account.
            let host = crate::net::host_of(&server);
            if !(host == "screenscraper.fr"
                || host.ends_with(".screenscraper.fr")
                || host == "127.0.0.1")
            {
                return Err(format!("not a ScreenScraper address: {host}"));
            }
            let reply = crate::net::send(&crate::net::Request {
                tls,
                server: &server,
                method: "GET",
                path: &path,
                form: None,
                json: None,
                agent: Some("Slotted-cartart/1"),
                bearer: None,
                limit,
            })?;
            Ok((reply.status, reply.body))
        })();
        self.last = Some(Instant::now());
        result.map_err(|e: String| redact(&e))
    }

    /// Stop the whole run on a quota or login refusal: every other game would get the same.
    fn check(status: u16) -> Result<(), String> {
        match status {
            401 | 403 => {
                Err("ScreenScraper refused the account (check Config/screenscraper.txt)".into())
            }
            423 => Err("ScreenScraper's API is closed for now".into()),
            426 => Err("ScreenScraper asks for a newer Slot".into()),
            429 => Err("ScreenScraper: too many requests at once".into()),
            430 | 431 => Err("ScreenScraper's daily quota is used up; try again tomorrow".into()),
            _ => Ok(()),
        }
    }

    /// The game's pictures, or None when ScreenScraper doesn't know it.
    pub fn lookup(
        &mut self,
        cart: &Cart,
        crc: Option<u32>,
        size: Option<u64>,
    ) -> Result<Option<Vec<Media>>, String> {
        let rom = cart
            .rom
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| cart.stem.clone());
        let mut url = format!(
            "{}/api2/jeuInfos.php?{}&output=json&systemeid={}&romtype=rom&romnom={}",
            self.api,
            self.creds.query(),
            system_id(cart.platform),
            enc(&rom)
        );
        if let Some(crc) = crc {
            url.push_str(&format!("&crc={crc:08X}"));
        }
        if let Some(size) = size {
            url.push_str(&format!("&romtaille={size}"));
        }
        let (status, body) = self.get(&url, JSON_MAX)?;
        Self::check(status)?;
        if status == 404 {
            return Ok(None);
        }
        if status != 200 {
            return Err(format!("ScreenScraper said {status}"));
        }
        let medias = parse_medias(&String::from_utf8_lossy(&body));
        Ok((!medias.is_empty()).then_some(medias))
    }

    pub fn download(&mut self, media: &Media) -> Result<Option<Vec<u8>>, String> {
        let sep = if media.url.contains('?') { '&' } else { '?' };
        let url = format!(
            "{}{sep}{}&maxwidth={MEDIA_WIDTH}&maxheight={MEDIA_WIDTH}",
            media.url,
            self.creds.query()
        );
        let (status, body) = self.get(&url, MEDIA_MAX)?;
        Self::check(status)?;
        Ok((status == 200 && body.starts_with(b"\x89PNG")).then_some(body))
    }
}

/// The label for `cart` from ScreenScraper, made on the spot.
pub fn label_for(client: &mut Client, cart: &Cart, crc: Option<u32>) -> Result<Got, String> {
    let size = std::fs::metadata(&cart.rom).ok().map(|m| m.len());
    let crc = crc.filter(|_| size.is_some_and(|s| s <= CRC_MAX));
    let Some(medias) = client.lookup(cart, crc, size)? else {
        return Ok(Got::NoMatch);
    };
    let dir = cart.platform.dir_name();
    let (lw, lh) = slot_ui::label_size(cart);
    // Twice the window for the larger views, no more than LABEL_MAX across.
    let k = (LABEL_MAX as f32 / lw.max(lh).max(1) as f32).min(2.0);
    let (ow, oh) = (
        ((lw as f32 * k).round() as u32).max(1),
        ((lh as f32 * k).round() as u32).max(1),
    );
    let prefer = regions(&cart.stem);
    let mut sources: Vec<Source> = Vec::new();
    let mut tried = Vec::new();
    for &kind in slot_cartart::profile(dir).order {
        let Some(media) = kind
            .media_types()
            .iter()
            .find_map(|t| pick(&medias, t, &prefer))
        else {
            continue;
        };
        let Some(png) = client.download(media)? else {
            continue;
        };
        let Some(img) = slot_cartart::decode_png(&png) else {
            continue;
        };
        sources.push(Source { kind, img });
        tried.push(kind.name());
        if let Some(fitted) = slot_cartart::fit_tier(dir, ow, oh, 1, &sources, kind) {
            let how = format!(
                "ScreenScraper {} {}: {} ({} ms)",
                media.kind,
                media.region,
                fitted.how,
                fitted.micros / 1000
            );
            return Ok(Got::Label(fitted.png(), how));
        }
    }
    Ok(Got::NoArt(if tried.is_empty() {
        "no usable pictures".into()
    } else {
        format!("tried {}", tried.join(", "))
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn creds() -> Creds {
        Creds {
            devid: "dev123".into(),
            devpassword: "s3cret&x".into(),
            user: "jeff".into(),
            password: "pw".into(),
        }
    }

    #[test]
    fn switched_off_without_the_line_or_an_id() {
        assert!(Creds::parse("user=a\npassword=b\ndevid=1\ndevpassword=2", None, None).is_none());
        assert!(Creds::parse("cartart=on\nuser=a", None, None).is_none());
        let c = Creds::parse(
            "cartart = on\nuser=a\npassword=b\ndevid=1\ndevpassword=2",
            None,
            None,
        )
        .unwrap();
        assert_eq!((c.devid.as_str(), c.user.as_str()), ("1", "a"));
        let c = Creds::parse("cartart=yes", Some("9"), Some("x")).unwrap();
        assert_eq!(c.devid, "9");
        assert!(c.query().contains("softname=Slotted") && !c.query().contains("ssid"));
    }

    #[test]
    fn credentials_never_survive_redaction() {
        let q = creds().query();
        let url = format!("https://api.screenscraper.fr/api2/jeuInfos.php?{q}&romnom=x");
        let r = redact(&format!("failed: {url} (timeout)"));
        for secret in ["dev123", "s3cret", "%26x", "jeff", "=pw"] {
            assert!(!r.contains(secret), "{secret} in {r}");
        }
        assert!(r.contains("romnom=x"));
    }

    #[test]
    fn medias_are_read_and_their_addresses_cleaned() {
        let json = r#"{"response":{"jeu":{"id":"5","noms":[{"region":"ss","text":"Sonic"}],
            "medias":[{"type":"wheel","parent":"jeu","url":"https:\/\/neoclone.screenscraper.fr\/api2\/mediaJeu.php?devid=1&devpassword=2&softname=Slotted&ssid=&sspassword=&systemeid=1&jeuid=5&media=wheel(eu)","region":"eu","format":"png"},
            {"type":"support-texture","parent":"jeu","url":"https:\/\/neoclone.screenscraper.fr\/api2\/mediaJeu.php?devid=1&devpassword=2&systemeid=1&jeuid=5&media=support-texture(us)","region":"us","format":"png"}]}}}"#;
        let m = parse_medias(json);
        assert_eq!(m.len(), 2);
        assert_eq!(m[1].kind, "support-texture");
        assert_eq!(m[1].region, "us");
        assert_eq!(m[1].url, "https://neoclone.screenscraper.fr/api2/mediaJeu.php?systemeid=1&jeuid=5&media=support-texture(us)");
        assert!(!m[0].url.contains("devid") && !m[0].url.contains("ssid"));
    }

    #[test]
    fn regions_follow_the_rom_name() {
        assert_eq!(regions("Rockman 2 (Japan)")[0], "jp");
        assert_eq!(&regions("Sonic (USA, Europe)")[..2], &["us", "eu"]);
        assert_eq!(regions("Homebrew")[0], "us");
        let medias = vec![
            Media {
                kind: "box-2D".into(),
                region: "jp".into(),
                url: "j".into(),
            },
            Media {
                kind: "box-2D".into(),
                region: "eu".into(),
                url: "e".into(),
            },
        ];
        assert_eq!(
            pick(&medias, "box-2D", &regions("X (Europe)")).unwrap().url,
            "e"
        );
        assert_eq!(
            pick(&medias, "box-2D", &regions("X (USA)")).unwrap().url,
            "e"
        );
        assert_eq!(
            pick(&medias, "box-2D", &regions("X (Japan)")).unwrap().url,
            "j"
        );
    }

    /// A stand-in ScreenScraper: answers by path (the query ignored), and keeps every request line.
    fn fake(
        routes: Vec<(&'static str, u16, Vec<u8>)>,
    ) -> (String, std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", listener.local_addr().unwrap());
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let log = seen.clone();
        let base = addr.clone();
        std::thread::spawn(move || {
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
                let line = String::from_utf8_lossy(&raw)
                    .lines()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let target = line.split_whitespace().nth(1).unwrap_or("").to_string();
                log.lock().unwrap().push(target.clone());
                let path = target.split('?').next().unwrap_or("");
                let (status, body) = routes
                    .iter()
                    .find(|(p, _, _)| *p == path)
                    .map(|(_, s, b)| match b.starts_with(b"\x89PNG") {
                        true => (*s, b.clone()),
                        false => (
                            *s,
                            String::from_utf8_lossy(b)
                                .replace("{BASE}", &base)
                                .into_bytes(),
                        ),
                    })
                    .unwrap_or((404, Vec::new()));
                let _ = write!(
                    conn,
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                );
                let _ = conn.write_all(&body);
            }
        });
        (addr, seen)
    }

    fn label_png() -> Vec<u8> {
        let (w, h) = (300u32, 245u32);
        let mut px = Vec::new();
        for y in 0..h {
            for x in 0..w {
                px.extend_from_slice(&[
                    ((x * 7 + y * 3) % 200) as u8,
                    (y % 256) as u8,
                    (x % 256) as u8,
                    255,
                ]);
            }
        }
        slot_cartart::encode_png_rgb(&slot_cartart::Image::from_rgba(w, h, px).unwrap())
    }

    fn md_cart(dir: &Path) -> Cart {
        let rom = dir.join("Sonic The Hedgehog (USA, Europe).md");
        std::fs::write(&rom, [0u8; 512]).unwrap();
        Cart {
            platform: Platform::Md,
            stem: "Sonic The Hedgehog (USA, Europe)".into(),
            rom,
            label: None,
            famicom_label: None,
            title: "Sonic".into(),
            code: String::new(),
            shell: None,
        }
    }

    #[test]
    fn a_game_gets_its_label_from_one_lookup_and_one_picture() {
        let json = br#"{"response":{"jeu":{"medias":[
            {"type":"box-2D","url":"{BASE}/api2/mediaJeu.php?devid=1&media=box-2D(us)","region":"us"},
            {"type":"support-texture","url":"{BASE}/api2/mediaJeu.php?devid=1&devpassword=2&media=support-texture(us)","region":"us"}]}}}"#;
        let (base, seen) = fake(vec![
            ("/api2/jeuInfos.php", 200, json.to_vec()),
            ("/api2/mediaJeu.php", 200, label_png()),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let cart = md_cart(dir.path());
        let mut client = Client::at(creds(), &base);
        let Got::Label(png, how) = label_for(&mut client, &cart, Some(0x1234ABCD)).unwrap() else {
            panic!("no label")
        };
        assert!(
            how.contains("support-texture us") && how.contains("texture"),
            "{how}"
        );
        let img = slot_cartart::decode_png(&png).unwrap();
        let (lw, lh) = slot_ui::label_size(&cart);
        assert!((img.w as f32 / img.h as f32 - lw as f32 / lh as f32).abs() < 0.02);
        assert!(img.w.max(img.h) <= LABEL_MAX);
        let seen = seen.lock().unwrap();
        // The label scan was enough: the box was never fetched.
        assert_eq!(seen.len(), 2, "{seen:?}");
        assert!(
            seen[0].contains("romnom=Sonic%20The%20Hedgehog%20%28USA%2C%20Europe%29.md")
                && seen[0].contains("crc=1234ABCD")
        );
        assert!(seen[1].contains("media=support-texture(us)") && seen[1].contains("maxwidth=1000"));
        assert!(
            seen[1].contains("devid=dev123"),
            "the stripped credentials are put back from the config"
        );
    }

    #[test]
    fn an_unknown_game_and_a_spent_quota() {
        let (base, _) = fake(vec![]);
        let dir = tempfile::tempdir().unwrap();
        let cart = md_cart(dir.path());
        assert!(matches!(
            label_for(&mut Client::at(creds(), &base), &cart, None),
            Ok(Got::NoMatch)
        ));
        let (base, _) = fake(vec![("/api2/jeuInfos.php", 430, b"quota".to_vec())]);
        let err = label_for(&mut Client::at(creds(), &base), &cart, None)
            .err()
            .unwrap();
        assert!(err.contains("quota"), "{err}");
    }

    #[test]
    fn only_screenscraper_is_ever_sent_the_account() {
        let mut c = Client::at(creds(), "https://example.com");
        let cart = Cart {
            platform: Platform::Md,
            stem: "Sonic".into(),
            rom: "/nonexistent/Sonic.md".into(),
            label: None,
            famicom_label: None,
            title: "Sonic".into(),
            code: String::new(),
            shell: None,
        };
        let err = c.lookup(&cart, None, None).unwrap_err();
        assert!(err.contains("not a ScreenScraper address"), "{err}");
        assert!(!err.contains("s3cret"));
    }
}
