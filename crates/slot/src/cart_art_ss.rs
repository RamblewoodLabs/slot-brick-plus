//! ScreenScraper: where every cart label comes from, for every shelf (HOME → Cart Art, and a
//! game's own label search in its cart menu).
//!
//! For a cart with no label, the game is looked up once (`jeuInfos`, by the ROM's name, and its
//! CRC32 and size when that is cheap to know), then its pictures are fetched one at a time in
//! the order the system's profile asks for (label scan, cart photo, box, logo) and handed to
//! `slot_cartart`, which turns them into the label the shelf draws. The first picture that
//! makes a good label wins, so most games cost one lookup and one download. The label is
//! written to `Labels/<system>/<game>.png` like every other: this runs at fetch time only.
//!
//! On by default. It needs the player's own ScreenScraper account, entered on the device
//! (HOME → Cart Art → ScreenScraper) and kept in `Config/screenscraper.txt` (`user=`,
//! `password=`): every Slot shares one developer id, whose quota is small, so nothing is looked
//! up without a login. `cartart=off` in that file switches it off. The developer id comes from
//! the build (`SLOT_SS_DEVID`, `SLOT_SS_DEVPASSWORD`) or, for a home build, from `devid=` and
//! `devpassword=` in the same file; without one there is no lookup at all.
//!
//! Every request carries the credentials in its query string, so nothing here ever logs a
//! URL: errors pass through `redact` first, and a media address is kept only with the
//! credentials taken out. No password is ever logged or shown.
use std::path::Path;
use std::time::{Duration, Instant};

use slot_cartart::Source;
use slot_store::{atomic_write, Cart, Platform};

pub const CONFIG: &str = "Config/screenscraper.txt";
const API: &str = "https://api.screenscraper.fr";
const SOFTNAME: &str = "Slotted";
/// One request at a time, a little apart: developer access allows one thread.
const PAUSE: Duration = Duration::from_millis(1200);
/// The gap between calls: PAUSE, but next to nothing against the tests' stand-in server.
fn pause() -> Duration {
    if cfg!(test) && std::env::var_os("SLOT_SS_LIVE").is_none() {
        Duration::from_millis(5)
    } else {
        PAUSE
    }
}
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
    /// The developer id and the account saved on the card, unless Cart Art is switched off
    /// (`cartart=off`) or this Slot has no developer id. The account may still be empty.
    pub fn load(root: &Path) -> Option<Creds> {
        let text = std::fs::read_to_string(root.join(CONFIG)).unwrap_or_default();
        let c = Self::parse(&text, built_id(), built_password())?;
        (!switched_off(&text)).then_some(c)
    }

    /// The developer id with `user` and `password` in place of any saved account, to test a
    /// login before it is kept. Works with Cart Art switched off too.
    pub fn trying(root: &Path, user: &str, password: &str) -> Option<Creds> {
        let text = std::fs::read_to_string(root.join(CONFIG)).unwrap_or_default();
        let mut c = Self::parse(&text, built_id(), built_password())?;
        c.user = user.trim().to_string();
        c.password = password.to_string();
        Some(c)
    }

    fn parse(text: &str, built_id: Option<&str>, built_pw: Option<&str>) -> Option<Creds> {
        let get = |key: &str| setting(text, key);
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

    /// Host tools: developer id from `dev` (and/or `SLOT_SS_DEVID` / `SLOT_SS_DEVPASSWORD`)
    /// and an optional user login from `user`. Missing or empty user fields are allowed
    /// (developer-only). File contents are never included in the error.
    pub fn from_files(dev: Option<&Path>, user: Option<&Path>) -> Result<Creds, String> {
        let mut text = String::new();
        if let Some(path) = dev {
            text.push_str(
                &std::fs::read_to_string(path)
                    .map_err(|e| format!("cannot read the developer properties file: {e}"))?,
            );
            text.push('\n');
        }
        if let Some(path) = user {
            if path.exists() {
                text.push_str(
                    &std::fs::read_to_string(path)
                        .map_err(|e| format!("cannot read the user login file: {e}"))?,
                );
                text.push('\n');
            }
        }
        Self::parse(&text, built_id(), built_password()).ok_or_else(|| {
            "no ScreenScraper developer id (screenscraper.properties or SLOT_SS_DEVID)"
                .into()
        })
    }

    /// A ScreenScraper account is saved: lookups may be made.
    pub fn logged_in(&self) -> bool {
        !self.user.is_empty() && !self.password.is_empty()
    }

    pub fn user(&self) -> &str {
        &self.user
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

/// The developer id this Slot was built with. Tests never use it, so no test can send it
/// anywhere or print it.
fn built_id() -> Option<&'static str> {
    if cfg!(test) {
        None
    } else {
        option_env!("SLOT_SS_DEVID")
    }
}

fn built_password() -> Option<&'static str> {
    if cfg!(test) {
        None
    } else {
        option_env!("SLOT_SS_DEVPASSWORD")
    }
}

/// This Slot has a developer id (built in, or in the config for a home build).
pub fn has_developer(root: &Path) -> bool {
    Creds::trying(root, "", "").is_some()
}

/// A `key=value` line of the config, comments skipped; None when missing or empty.
fn setting(text: &str, key: &str) -> Option<String> {
    text.lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case(key))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn switched_off(text: &str) -> bool {
    setting(text, "cartart").is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "off" | "0" | "no" | "false"))
}

/// The ScreenScraper user name saved on the card, if any (never the password).
pub fn saved_user(root: &Path) -> Option<String> {
    let text = std::fs::read_to_string(root.join(CONFIG)).ok()?;
    setting(&text, "password")?;
    setting(&text, "user")
}

/// Keeps the account in the config, every other line of it as it was.
pub fn save_account(root: &Path, user: &str, password: &str) -> std::io::Result<()> {
    if user.contains(['\n', '\r']) || password.contains(['\n', '\r']) {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "a line break in the login"));
    }
    let mut lines = other_lines(root);
    lines.push(format!("user={}", user.trim()));
    lines.push(format!("password={password}"));
    write_config(root, &lines)
}

/// Signs out: the account is taken out of the config, the rest of it kept.
pub fn forget_account(root: &Path) -> std::io::Result<()> {
    let lines = other_lines(root);
    write_config(root, &lines)
}

fn other_lines(root: &Path) -> Vec<String> {
    std::fs::read_to_string(root.join(CONFIG))
        .unwrap_or_default()
        .lines()
        .filter(|l| {
            let key = l.split_once('=').map(|(k, _)| k.trim().to_ascii_lowercase());
            !matches!(key.as_deref(), Some("user" | "password"))
        })
        .map(String::from)
        .collect()
}

fn write_config(root: &Path, lines: &[String]) -> std::io::Result<()> {
    let path = root.join(CONFIG);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = lines.join("\n");
    text.push('\n');
    atomic_write(&path, text.as_bytes())
}

/// What ScreenScraper says about an account (`ssuserInfos`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserInfo {
    pub user: String,
    pub level: String,
    pub threads: u64,
    pub today: u64,
    pub per_day: u64,
}

impl UserInfo {
    /// One line for the screen.
    pub fn line(&self) -> String {
        let mut s = format!("Logged in as {}", self.user);
        if !self.level.is_empty() {
            s.push_str(&format!(", level {}", self.level));
        }
        if self.threads > 0 {
            s.push_str(&format!(", {} thread{}", self.threads, if self.threads == 1 { "" } else { "s" }));
        }
        if self.per_day > 0 {
            s.push_str(&format!(". {} of {} lookups used today", self.today, self.per_day));
        }
        s
    }
}

/// Tries a login against ScreenScraper without keeping it: the account's details, or why not.
pub fn test_login(root: &Path, user: &str, password: &str) -> Result<UserInfo, String> {
    let creds = Creds::trying(root, user, password)
        .ok_or("This Slot was built without a ScreenScraper developer id")?;
    if !creds.logged_in() {
        return Err("Enter your ScreenScraper username and password first".into());
    }
    Client::new(creds).user_info()
}

/// Tests the login saved on the card.
pub fn test_saved(root: &Path) -> Result<UserInfo, String> {
    let creds = Creds::trying(root, "", "").ok_or("This Slot was built without a ScreenScraper developer id")?;
    let saved = Creds::load(root).filter(Creds::logged_in);
    let creds = saved.unwrap_or(creds);
    if !creds.logged_in() {
        return Err("Enter your ScreenScraper username and password first".into());
    }
    Client::new(creds).user_info()
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
    objects(json, "medias")
        .into_iter()
        .filter_map(|obj| {
            Some(Media {
                kind: field(obj, "type")?,
                region: field(obj, "region").unwrap_or_default(),
                url: strip_auth(&field(obj, "url")?),
            })
        })
        .collect()
}

/// The games of a `jeuRecherche` reply: each one's name and pictures.
pub fn parse_games(json: &str) -> Vec<Found> {
    objects(json, "jeux")
        .into_iter()
        .filter_map(|obj| {
            let medias = parse_medias(obj);
            let name = obj.find("\"noms\"").and_then(|at| field(&obj[at..], "text"))?;
            (!medias.is_empty()).then_some(Found { name, medias })
        })
        .collect()
}

/// A game a search found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub name: String,
    pub medias: Vec<Media>,
}

/// The objects, as text, of the first array named `key`.
fn objects<'a>(json: &'a str, key: &str) -> Vec<&'a str> {
    let Some(at) = json.find(&format!("\"{key}\"")) else {
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
            '{' | '[' => {
                if depth == 0 && c == '{' {
                    start = Some(i);
                }
                depth += 1;
            }
            '}' | ']' if depth > 0 => {
                depth -= 1;
                if depth == 0 && c == '}' {
                    if let Some(s) = start.take() {
                        out.push(&rest[open + 1 + s..open + 1 + i + 1]);
                    }
                }
            }
            ']' => break,
            _ => {}
        }
    }
    out
}

/// A number field, written as a number or a string.
fn number(obj: &str, key: &str) -> u64 {
    if let Some(v) = field(obj, key) {
        return v.trim().parse().unwrap_or(0);
    }
    let pat = format!("\"{key}\"");
    obj.find(&pat)
        .and_then(|at| obj[at + pat.len()..].trim_start().strip_prefix(':'))
        .map(|r| r.trim_start().chars().take_while(char::is_ascii_digit).collect::<String>())
        .and_then(|d| d.parse().ok())
        .unwrap_or(0)
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

    /// Another address for the API (a stand-in, in tests). Only ScreenScraper's own hosts and
    /// this machine are ever sent the account, whatever it says.
    pub fn with_api(creds: Creds, api: &str) -> Self {
        Client {
            creds,
            last: None,
            api: api.to_string(),
        }
    }

    fn get(&mut self, url: &str, limit: usize) -> Result<(u16, Vec<u8>), String> {
        if let Some(last) = self.last {
            let since = last.elapsed();
            if since < pause() {
                std::thread::sleep(pause() - since);
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
                Err("ScreenScraper refused the login: check the username and password under HOME → Cart Art".into())
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

    /// The account's details (`ssuserInfos`): the way to test a login.
    pub fn user_info(&mut self) -> Result<UserInfo, String> {
        let url = format!("{}/api2/ssuserInfos.php?{}&output=json", self.api, self.creds.query());
        let (status, body) = self.get(&url, JSON_MAX)?;
        if matches!(status, 401 | 403) {
            return Err(format!(
                "ScreenScraper refused that username or password (HTTP {status})"
            ));
        }
        Self::check(status)?;
        if status != 200 {
            return Err(format!("ScreenScraper said {status}"));
        }
        let text = String::from_utf8_lossy(&body);
        let Some(at) = text.find("\"ssuser\"") else {
            let lower = text.to_ascii_lowercase();
            let kind = if text.trim_start().starts_with('<') {
                "html"
            } else if lower.contains("erreur de login") || lower.contains("identifiant") {
                "login-error"
            } else if text.contains("\"header\"") {
                "json-without-account"
            } else {
                "unrecognized"
            };
            return Err(format!(
                "ScreenScraper reply had no account block (HTTP {status}, {kind}, {} bytes)",
                body.len()
            ));
        };
        let obj = &text[at..];
        Ok(UserInfo {
            user: field(obj, "id").unwrap_or_else(|| self.creds.user.clone()),
            level: field(obj, "niveau").unwrap_or_default(),
            threads: number(obj, "maxthreads"),
            today: number(obj, "requeststoday"),
            per_day: number(obj, "maxrequestsperday"),
        })
    }

    /// The games of a system whose names match `query` (`jeuRecherche`), best first.
    pub fn search(&mut self, platform: Platform, query: &str) -> Result<Vec<Found>, String> {
        let url = format!(
            "{}/api2/jeuRecherche.php?{}&output=json&systemeid={}&recherche={}",
            self.api,
            self.creds.query(),
            system_id(platform),
            enc(query.trim())
        );
        let (status, body) = self.get(&url, JSON_MAX)?;
        Self::check(status)?;
        if status == 404 {
            return Ok(Vec::new());
        }
        if status != 200 {
            return Err(format!("ScreenScraper said {status}"));
        }
        Ok(parse_games(&String::from_utf8_lossy(&body)))
    }
}

/// The size a cart's label is made at: twice its window, no more than LABEL_MAX across.
pub fn label_out(cart: &Cart) -> (u32, u32) {
    label_out_at(cart, 2)
}

/// `scale` times the label window, no more than LABEL_MAX across. The RG280V preload uses 1.
pub fn label_out_at(cart: &Cart, scale: u32) -> (u32, u32) {
    let (lw, lh) = slot_ui::label_size(cart);
    let cap = scale.max(1) as f32;
    let k = (LABEL_MAX as f32 / lw.max(lh).max(1) as f32).min(cap);
    (((lw as f32 * k).round() as u32).max(1), ((lh as f32 * k).round() as u32).max(1))
}

/// Labels from a game's pictures, one tier at a time in the system's order: each label made
/// is handed to `offer` (the PNG and how it was made), which says whether to go on. With
/// `every` false the first label ends it. Returns the kinds of picture tried.
pub fn fit_from(
    client: &mut Client,
    cart: &Cart,
    medias: &[Media],
    every: bool,
    offer: &mut dyn FnMut(Vec<u8>, String) -> bool,
) -> Result<Vec<&'static str>, String> {
    fit_from_at(client, cart, medias, every, 2, offer)
}

/// Like `fit_from`, but at `scale` times the window (1 for the RG280V preload).
pub fn fit_from_at(
    client: &mut Client,
    cart: &Cart,
    medias: &[Media],
    every: bool,
    scale: u32,
    offer: &mut dyn FnMut(Vec<u8>, String) -> bool,
) -> Result<Vec<&'static str>, String> {
    let dir = cart.platform.dir_name();
    let (ow, oh) = label_out_at(cart, scale);
    let prefer = regions(&cart.stem);
    let mut sources: Vec<Source> = Vec::new();
    let mut tried = Vec::new();
    for &kind in slot_cartart::profile(dir).order {
        let Some(media) = kind.media_types().iter().find_map(|t| pick(medias, t, &prefer)) else {
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
            if !offer(fitted.png(), how) || !every {
                break;
            }
        }
    }
    Ok(tried)
}

/// The label for `cart` from ScreenScraper, made on the spot.
pub fn label_for(client: &mut Client, cart: &Cart, crc: Option<u32>) -> Result<Got, String> {
    label_for_at(client, cart, crc, 2)
}

/// Like `label_for`, but at `scale` times the window (1 for the RG280V preload).
pub fn label_for_at(
    client: &mut Client,
    cart: &Cart,
    crc: Option<u32>,
    scale: u32,
) -> Result<Got, String> {
    let size = std::fs::metadata(&cart.rom).ok().map(|m| m.len());
    let crc = crc.filter(|_| size.is_some_and(|s| s <= CRC_MAX));
    let Some(medias) = client.lookup(cart, crc, size)? else {
        return Ok(Got::NoMatch);
    };
    let mut got = None;
    let tried = fit_from_at(client, cart, &medias, false, scale, &mut |png, how| {
        got = Some((png, how));
        false
    })?;
    Ok(match got {
        Some((png, how)) => Got::Label(png, how),
        None if tried.is_empty() => Got::NoArt("no usable pictures".into()),
        None => Got::NoArt(format!("tried {}", tried.join(", "))),
    })
}

/// A refusal every other game would get too (login, quota, the API closed): the run stops.
pub fn stops_the_run(error: &str) -> bool {
    error.starts_with("ScreenScraper")
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
    fn on_by_default_off_by_the_line_and_never_without_an_id() {
        assert!(Creds::parse("user=a\npassword=b", None, None).is_none(), "no developer id");
        let c = Creds::parse("user=a\npassword=b\ndevid=1\ndevpassword=2", None, None).unwrap();
        assert_eq!((c.devid.as_str(), c.user.as_str()), ("1", "a"));
        assert!(c.logged_in());
        let c = Creds::parse("", Some("9"), Some("x")).unwrap();
        assert_eq!(c.devid, "9");
        assert!(!c.logged_in());
        assert!(c.query().contains("softname=Slotted") && !c.query().contains("ssid"));
        assert!(switched_off("cartart = off") && switched_off("cartart=NO"));
        assert!(!switched_off("cartart=on") && !switched_off(""));
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Config")).unwrap();
        std::fs::write(dir.path().join(CONFIG), "devid=1\ndevpassword=2\n").unwrap();
        assert!(Creds::load(dir.path()).is_some(), "on without cartart=on");
        std::fs::write(dir.path().join(CONFIG), "cartart=off\ndevid=1\ndevpassword=2\n").unwrap();
        assert!(Creds::load(dir.path()).is_none());
        assert!(Creds::trying(dir.path(), "a", "b").is_some(), "a login can still be tested");
    }

    #[test]
    fn the_account_is_saved_and_forgotten_and_the_rest_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        assert_eq!(saved_user(root), None);
        save_account(root, " jeff ", "p=w d").unwrap();
        assert_eq!(saved_user(root).as_deref(), Some("jeff"));
        let c = Creds::trying(root, "", "");
        assert!(c.is_none(), "no developer id in a test");
        std::fs::write(root.join(CONFIG), "# mine\ndevid=1\ndevpassword=2\nuser=old\npassword=old\n").unwrap();
        save_account(root, "jeff", "p=w d").unwrap();
        let c = Creds::load(root).unwrap();
        assert_eq!((c.user.as_str(), c.password.as_str()), ("jeff", "p=w d"));
        let text = std::fs::read_to_string(root.join(CONFIG)).unwrap();
        assert!(text.contains("# mine") && text.contains("devid=1") && !text.contains("old"));
        forget_account(root).unwrap();
        assert_eq!(saved_user(root), None);
        assert!(!Creds::load(root).unwrap().logged_in());
        assert!(save_account(root, "a\nb", "x").is_err());
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
        let mut client = Client::with_api(creds(), &base);
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
            label_for(&mut Client::with_api(creds(), &base), &cart, None),
            Ok(Got::NoMatch)
        ));
        let (base, _) = fake(vec![("/api2/jeuInfos.php", 430, b"quota".to_vec())]);
        let err = label_for(&mut Client::with_api(creds(), &base), &cart, None)
            .err()
            .unwrap();
        assert!(err.contains("quota"), "{err}");
    }

    #[test]
    fn only_screenscraper_is_ever_sent_the_account() {
        let mut c = Client::with_api(creds(), "https://example.com");
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

    #[test]
    fn a_login_is_tested_and_its_quota_shown() {
        let json = br#"{"header":{"success":"true"},"response":{"serveurs":{"cpu1":"10"},"ssuser":{"id":"jeff","numid":"42","niveau":"1","maxthreads":"1","requeststoday":"12","maxrequestsperday":20000}}}"#;
        let (base, seen) = fake(vec![("/api2/ssuserInfos.php", 200, json.to_vec())]);
        let info = Client::with_api(creds(), &base).user_info().unwrap();
        assert_eq!(info, UserInfo { user: "jeff".into(), level: "1".into(), threads: 1, today: 12, per_day: 20000 });
        assert_eq!(info.line(), "Logged in as jeff, level 1, 1 thread. 12 of 20000 lookups used today");
        assert!(seen.lock().unwrap()[0].contains("ssid=jeff"));
        let (base, _) = fake(vec![("/api2/ssuserInfos.php", 200, b"Erreur de login : Verifier vos identifiants !".to_vec())]);
        let err = Client::with_api(creds(), &base).user_info().unwrap_err();
        assert!(err.contains("refused") && !err.contains("pw"), "{err}");
        let (base, _) = fake(vec![("/api2/ssuserInfos.php", 403, Vec::new())]);
        assert!(Client::with_api(creds(), &base).user_info().unwrap_err().contains("refused"));
        let dir = tempfile::tempdir().unwrap();
        assert!(test_login(dir.path(), "jeff", "pw").unwrap_err().contains("developer id"));
    }

    #[test]
    fn a_search_gives_each_game_with_its_pictures() {
        let json = br#"{"response":{"ssuser":{"id":"jeff"},"jeux":[
            {"id":"1","noms":[{"region":"us","text":"Sonic the Hedgehog"},{"region":"jp","text":"Sonic"}],"systeme":{"id":"1"},
             "medias":[{"type":"support-texture","url":"{BASE}/api2/mediaJeu.php?devid=1&media=support-texture(us)","region":"us"}]},
            {"id":"2","noms":[{"region":"wor","text":"Sonic 2"}],"medias":[]},
            {"id":"3","noms":[{"region":"wor","text":"Sonic Spinball"}],"medias":[{"type":"box-2D","url":"{BASE}/api2/mediaJeu.php?media=box-2D(wor)","region":"wor"}]}]}}"#;
        let (base, seen) = fake(vec![("/api2/jeuRecherche.php", 200, json.to_vec()), ("/api2/mediaJeu.php", 200, label_png())]);
        let mut client = Client::with_api(creds(), &base);
        let found = client.search(Platform::Md, "sonic hedgehog").unwrap();
        assert_eq!(found.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["Sonic the Hedgehog", "Sonic Spinball"]);
        assert!(seen.lock().unwrap()[0].contains("systemeid=1&recherche=sonic%20hedgehog"));
        let dir = tempfile::tempdir().unwrap();
        let cart = md_cart(dir.path());
        let mut labels = Vec::new();
        fit_from(&mut client, &cart, &found[0].medias, true, &mut |png, how| {
            labels.push((png, how));
            true
        })
        .unwrap();
        assert_eq!(labels.len(), 1);
        assert!(labels[0].1.contains("support-texture"));
        assert!(stops_the_run("ScreenScraper's daily quota is used up") && !stops_the_run("timed out"));
    }

    /// The real thing, by hand: `SLOT_SS_LIVE=<properties with devid= / devpassword=>`, and
    /// `SLOT_SS_LIVE_OUT=<dir>` for the labels. Uses the developer account as the login when no
    /// `ssid=` / `sspassword=` are in the file. Prints nothing secret: the user name is masked.
    #[test]
    #[ignore = "talks to ScreenScraper"]
    fn live_login_lookup_and_search() {
        let props = std::fs::read_to_string(std::env::var("SLOT_SS_LIVE").unwrap()).unwrap();
        let get = |k: &str| setting(&props, k).unwrap_or_default();
        let (devid, devpw) = (get("devid"), get("devpassword"));
        let (user, pw) = if get("ssid").is_empty() { (devid.clone(), devpw.clone()) } else { (get("ssid"), get("sspassword")) };
        let account = if get("ssid").is_empty() { "the developer account" } else { "a user account" };
        let mask = |t: String| redact(&t).replace(&user, "<user>").replace(&devid, "<dev>");
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("Config")).unwrap();
        std::fs::write(root.join(CONFIG), format!("devid={devid}\ndevpassword={devpw}\n")).unwrap();
        assert!(crate::cart_art::ready(root).is_err(), "no login yet: Cart Art asks for one");
        println!("LIVE login with {account}");
        let login = test_login(root, &user, &pw);
        println!("LIVE test login: {}", mask(match &login { Ok(i) => format!("OK {}", i.line()), Err(e) => format!("FAILED {e}") }));
        let bad = test_login(root, &user, "definitely-not-the-password");
        println!("LIVE wrong password: {}", mask(match bad { Ok(i) => format!("OK?! {}", i.line()), Err(e) => format!("refused as expected: {e}") }));
        let creds = if login.is_ok() {
            save_account(root, &user, &pw).unwrap();
            println!("LIVE saved; ready(): {}", crate::cart_art::ready(root).is_ok());
            println!("LIVE test saved login: {}", mask(test_saved(root).map(|i| i.line()).unwrap_or_else(|e| e)));
            Creds::load(root).unwrap()
        } else {
            println!("LIVE no login: looking up with the developer id alone");
            Creds::trying(root, "", "").unwrap()
        };
        let out = std::env::var("SLOT_SS_LIVE_OUT").map(std::path::PathBuf::from).unwrap_or_else(|_| root.join("out"));
        std::fs::create_dir_all(&out).unwrap();
        let mut client = Client::new(creds);
        for (dir, rom) in [("GB", "Tetris (World) (Rev 1).gb"), ("SNES", "Super Mario World (USA).sfc"), ("MD", "Sonic The Hedgehog (USA, Europe).md"), ("NES", "Mega Man 2 (USA).nes")] {
            let path = root.join("Games").join(dir).join(rom);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, [0u8; 16]).unwrap();
            let cart = slot_store::scan(root).unwrap().into_iter().find(|c| c.rom == path).unwrap();
            let t = std::time::Instant::now();
            let medias = client.lookup(&cart, None, None);
            let line = match medias {
                Ok(Some(m)) => {
                    let mut got = None;
                    let tried = fit_from(&mut client, &cart, &m, false, &mut |png, how| { got = Some((png, how)); false });
                    match (got, tried) {
                        (Some((png, how)), _) => {
                            let file = out.join(format!("live-{dir}-{}.png", cart.stem));
                            std::fs::write(&file, &png).unwrap();
                            let (w, h) = crate::cart_art::png_size(&png).unwrap();
                            let (lw, lh) = slot_ui::label_size(&cart);
                            format!("{} medias; label {w}x{h} (window {lw}x{lh}); {how}", m.len())
                        }
                        (None, t) => format!("{} medias; no label ({t:?})", m.len()),
                    }
                }
                Ok(None) => "not found".into(),
                Err(e) => format!("error {e}"),
            };
            println!("LIVE {dir} {rom}: {} [{} ms]", mask(line), t.elapsed().as_millis());
        }
        match client.search(Platform::Gb, "tetris") {
            Ok(games) => println!("LIVE search GB 'tetris': {} games, first {:?}", games.len(), games.iter().take(3).map(|g| (g.name.clone(), g.medias.len())).collect::<Vec<_>>()),
            Err(e) => println!("LIVE search failed: {}", mask(e)),
        }
        forget_account(root).unwrap();
        println!("LIVE signed out; saved_user: {:?}; ready: {}", saved_user(root), crate::cart_art::ready(root).err().unwrap_or_default());
    }


    #[test]
    fn from_files_loads_dev_and_optional_user() {
        let dir = tempfile::tempdir().unwrap();
        let dev = dir.path().join("dev.properties");
        let user = dir.path().join("user.properties");
        std::fs::write(&dev, "devid=abc\ndevpassword=xyz\n").unwrap();
        std::fs::write(&user, "user=jeff\npassword=secret\n").unwrap();
        let c = Creds::from_files(Some(&dev), Some(&user)).unwrap();
        assert!(c.logged_in());
        assert_eq!(c.user(), "jeff");
        let c = Creds::from_files(Some(&dev), None).unwrap();
        assert!(!c.logged_in());
    }

    #[test]
    fn a_one_x_label_is_the_window_itself() {
        let dir = tempfile::tempdir().unwrap();
        let cart = md_cart(dir.path());
        let (w, h) = slot_ui::label_size(&cart);
        assert_eq!(label_out_at(&cart, 1), (w, h));
        let (w2, h2) = label_out(&cart);
        assert!(w2 >= w && h2 >= h);
        assert!(w2 <= LABEL_MAX && h2 <= LABEL_MAX);
    }
}
