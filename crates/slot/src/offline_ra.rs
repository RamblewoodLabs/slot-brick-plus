//! Offline RetroAchievements, on top of the rcheevos client Slot already runs.
//!
//! rcheevos (MIT, vendored) evaluates a set locally and retries an unlock in memory, with
//! the same backoff the server expects, including how long ago it unlocked. That queue
//! dies when Slot exits. NextUI's offline ledger cannot be copied in: that project is
//! PolyForm Noncommercial, which Slot's GPL does not accept. RAOfflineProxy is GPL and is
//! the tool Knulli, muOS and Batocera route RetroArch through, but it is a separate proxy
//! process; Slot speaks to the server itself. RetroArch's own queue is the in-memory one.
//!
//! So this file keeps rcheevos' requests and its signature, and adds what it does not:
//! a copy of each set and session on the card, and a checksummed unlock queue that
//! survives power loss. Unlocks are softcore only. A hardcore award is never queued.
//! The server's "already unlocked" answer counts as sent. Any other refusal is kept and
//! not submitted again.
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use slot_cheevos::Console;
use slot_store::{Cart, Library, Platform};

const QUEUE: &str = "Config/Achievements/queue.txt";
const SETS: &str = "Config/Achievements/sets";
const SESSIONS: &str = "Config/Achievements/sessions";
const LOGIN: &str = "Config/Achievements/login.json";
const CACHE_QUEUE: &str = "Config/Achievements/cache-queue.txt";
const CACHE_MISSES: &str = "Config/Achievements/cache-misses.txt";
const CACHE_PICK: &str = "Config/Achievements/cache-pick.txt";

/// Recently played games included in the set cache, on top of every favorite.
pub const RECENT_SET_CACHE: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Unlock {
    id: u32,
    hash: String,
    when: i64,
    attempts: u32,
    /// The server refused this one. It stays on the card and is not sent again.
    refused: bool,
    why: String,
}

/// How long to wait before trying an unlock again, after this many failed sends.
/// The first failure is immediate, then 1s, 2s, 4s, and so on up to two minutes.
/// That is the curve rcheevos uses for its own in-memory retries.
pub fn backoff_secs(failures: u32) -> u64 {
    if failures <= 1 {
        0
    } else if failures >= 9 {
        120
    } else {
        1 << (failures - 2)
    }
}

fn queue_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn queue_path(root: &Path) -> PathBuf {
    root.join(QUEUE)
}

fn queue_bak(path: &Path) -> PathBuf {
    path.with_file_name("queue.txt.bak")
}

fn sound(bytes: &[u8]) -> Option<Vec<Unlock>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut body = String::new();
    let mut sum = None;
    for line in text.lines() {
        if let Some(hex) = line.trim().strip_prefix("sum=") {
            sum = u64::from_str_radix(hex.trim(), 16).ok();
        } else {
            body.push_str(line);
            body.push('\n');
        }
    }
    if checksum(body.as_bytes()) != sum? {
        return None;
    }
    if body.lines().next()?.trim() != "slot-unlocks 1" {
        return None;
    }
    Some(parse_unlocks(&body))
}

fn parse_unlocks(text: &str) -> Vec<Unlock> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("u=") else {
            continue;
        };
        let mut f = rest.split('\t');
        let Some(id) = f.next().and_then(|v| v.parse().ok()) else {
            continue;
        };
        let hash = f.next().unwrap_or("").to_string();
        let when = f.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let attempts = f.next().and_then(|v| v.parse().ok()).unwrap_or(0);
        let state = f.next().unwrap_or("wait");
        let why = f.next().unwrap_or("").replace("\\n", " ");
        out.push(Unlock {
            id,
            hash,
            when,
            attempts,
            refused: state == "refused",
            why,
        });
    }
    out
}

fn render_unlocks(rows: &[Unlock]) -> String {
    let mut out = String::from("slot-unlocks 1\n");
    for row in rows {
        let why = row.why.replace(['\n', '\t'], " ");
        out.push_str(&format!(
            "u={}\t{}\t{}\t{}\t{}\t{}\n",
            row.id,
            row.hash,
            row.when,
            row.attempts,
            if row.refused { "refused" } else { "wait" },
            why
        ));
    }
    let sum = checksum(out.as_bytes());
    out.push_str(&format!("sum={sum:016x}\n"));
    out
}

fn load_queue(root: &Path) -> Vec<Unlock> {
    let _guard = queue_lock();
    load_queue_locked(root)
}

fn load_queue_locked(root: &Path) -> Vec<Unlock> {
    let path = queue_path(root);
    let bak = queue_bak(&path);
    if let Some(rows) = std::fs::read(&path).ok().and_then(|b| sound(&b)) {
        if !bak.is_file() {
            let _ = slot_store::atomic_write(&bak, &std::fs::read(&path).unwrap_or_default());
        }
        return rows;
    }
    if let Some(rows) = std::fs::read(&bak).ok().and_then(|b| sound(&b)) {
        let text = render_unlocks(&rows);
        let _ = write_queue_bytes(&path, text.as_bytes());
        return rows;
    }
    Vec::new()
}

fn write_queue_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bak = queue_bak(path);
    if let Ok(current) = std::fs::read(path) {
        if sound(&current).is_some() {
            slot_store::atomic_write(&bak, &current)?;
        }
    }
    slot_store::atomic_write(path, bytes)?;
    if !bak.is_file() {
        slot_store::atomic_write(&bak, bytes)?;
    }
    Ok(())
}

fn save_queue_locked(root: &Path, rows: &[Unlock]) -> std::io::Result<()> {
    write_queue_bytes(&queue_path(root), render_unlocks(rows).as_bytes())
}

fn mutate_queue(root: &Path, f: impl FnOnce(&mut Vec<Unlock>)) {
    let _guard = queue_lock();
    let mut rows = load_queue_locked(root);
    f(&mut rows);
    let _ = save_queue_locked(root, &rows);
}

/// How many unlocks are still waiting to be sent.
pub fn waiting(root: &Path) -> usize {
    load_queue(root).iter().filter(|u| !u.refused).count()
}

fn remember_unlock(root: &Path, id: u32, hash: &str, when: i64) {
    if id == 0 {
        return;
    }
    let hash = if hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        hash.to_string()
    } else {
        String::new()
    };
    mutate_queue(root, |rows| {
        if let Some(row) = rows.iter_mut().find(|u| u.id == id) {
            if row.hash.is_empty() {
                row.hash = hash;
            }
            if row.when == 0 {
                row.when = when;
            }
            return;
        }
        rows.push(Unlock {
            id,
            hash,
            when,
            attempts: 0,
            refused: false,
            why: String::new(),
        });
    });
}

fn settle_unlock(root: &Path, id: u32, judge: Judge, why: &str) {
    mutate_queue(root, |rows| match judge {
        Judge::Synced => rows.retain(|u| u.id != id),
        Judge::Retry => {
            if let Some(row) = rows.iter_mut().find(|u| u.id == id) {
                row.attempts = row.attempts.saturating_add(1);
            }
        }
        Judge::Refused => {
            if let Some(row) = rows.iter_mut().find(|u| u.id == id) {
                row.refused = true;
                row.why = why.to_string();
            }
        }
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Judge {
    Synced,
    Retry,
    Refused,
}

fn judge(status: u16, body: &str) -> Judge {
    if status == 200 && (json_success(body) || already_unlocked(body)) {
        return Judge::Synced;
    }
    match status {
        0 | 408 | 429 | 500 | 502 | 503 | 504 | 521 | 522 => Judge::Retry,
        200 => Judge::Refused,
        s if (400..500).contains(&s) => Judge::Refused,
        _ => Judge::Retry,
    }
}

fn already_unlocked(body: &str) -> bool {
    body.to_ascii_lowercase().contains("user already has")
}

/// One achievement in a cached set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ach {
    pub id: u32,
    pub points: u32,
    pub badge: String,
    pub title: String,
    pub description: String,
    pub unlocked: bool,
    pub waiting: bool,
    pub refused: bool,
}

/// A game whose set is on the card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameCard {
    pub key: String,
    pub title: String,
    pub game_id: u32,
    pub place: String,
    pub achievements: Vec<Ach>,
}

/// Every cached set, with unlocks from the session and the queue filled in.
pub fn games(root: &Path) -> Vec<GameCard> {
    let queue = load_queue(root);
    let dir = root.join(SETS);
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .map(|d| d.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    names.sort();
    let mut out = Vec::new();
    for path in names {
        if path.extension().is_some_and(|e| e == "list") {
            if let Some(card) = read_list(&path, &queue, root) {
                out.push(card);
            }
        }
    }
    out.sort_by(|a, b| {
        a.title
            .to_ascii_lowercase()
            .cmp(&b.title.to_ascii_lowercase())
    });
    out
}

fn read_list(path: &Path, queue: &[Unlock], root: &Path) -> Option<GameCard> {
    let text = std::fs::read_to_string(path).ok()?;
    let key = path.file_stem()?.to_str()?.to_string();
    let mut title = String::new();
    let mut game_id = 0u32;
    let mut place = String::new();
    let mut achievements = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("game=") {
            title = rest.to_string();
        } else if let Some(rest) = line.strip_prefix("id=") {
            game_id = rest.parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("where=") {
            place = rest.replace('\t', "  ");
        } else if let Some(rest) = line.strip_prefix("a=") {
            let mut f = rest.splitn(5, '\t');
            let id = f.next()?.parse().ok()?;
            let points = f.next()?.parse().ok()?;
            let badge = f.next()?.to_string();
            let ach_title = f.next()?.to_string();
            let description = f.next().unwrap_or("").to_string();
            let queued = queue.iter().find(|u| u.id == id);
            achievements.push(Ach {
                id,
                points,
                badge,
                title: ach_title,
                description,
                unlocked: false,
                waiting: queued.is_some_and(|u| !u.refused),
                refused: queued.is_some_and(|u| u.refused),
            });
        }
    }
    if title.is_empty() {
        return None;
    }
    let known = session_ids(root, game_id);
    for ach in &mut achievements {
        if known.contains(&ach.id) || ach.waiting || ach.refused {
            ach.unlocked = true;
        }
    }
    Some(GameCard {
        key,
        title,
        game_id,
        place,
        achievements,
    })
}

fn session_ids(root: &Path, game_id: u32) -> Vec<u32> {
    if game_id == 0 {
        return Vec::new();
    }
    let Ok(text) = std::fs::read_to_string(root.join(SESSIONS).join(format!("{game_id}.json")))
    else {
        return Vec::new();
    };
    unlock_ids(&text)
}

fn unlock_ids(body: &str) -> Vec<u32> {
    let Some(at) = find_key(body, "Unlocks") else {
        return Vec::new();
    };
    let Some(start) = body[at..].find('[').map(|i| at + i) else {
        return Vec::new();
    };
    let Some(end) = matching_bracket(&body, start) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    let slice = &body[start..=end];
    let mut from = 0;
    while let Some(rel) = slice[from..].find("\"ID\"") {
        let at = from + rel;
        from = at + 4;
        let Some(num) = slice[from..].split(':').nth(1) else {
            continue;
        };
        let num = num.trim_start();
        let end = num.find(|c: char| !c.is_ascii_digit()).unwrap_or(num.len());
        if let Ok(id) = num[..end].parse() {
            ids.push(id);
        }
    }
    ids
}

/// Inserts queued softcore unlocks into a start-session body so a later load shows them earned.
pub fn merge_unlocks(body: &str, extra: &[(u32, i64)]) -> String {
    if extra.is_empty() {
        return body.to_string();
    }
    let have = unlock_ids(body);
    let add: Vec<_> = extra
        .iter()
        .filter(|(id, _)| !have.contains(id))
        .copied()
        .collect();
    if add.is_empty() {
        return body.to_string();
    }
    let items: Vec<String> = add
        .iter()
        .map(|(id, when)| format!("{{\"ID\":{id},\"When\":{when}}}"))
        .collect();
    let joined = items.join(",");
    if let Some(at) = find_key(body, "Unlocks") {
        if let Some(bracket) = body[at..].find('[') {
            let start = at + bracket;
            if let Some(end) = matching_bracket(body, start) {
                let inside = body[start + 1..end].trim();
                let insert = if inside.is_empty() {
                    joined
                } else {
                    format!("{inside},{joined}")
                };
                return format!("{}[{}]{}", &body[..start], insert, &body[end + 1..]);
            }
        }
    }
    let trimmed = body.trim_end();
    if let Some(rest) = trimmed.strip_suffix('}') {
        return format!("{rest},\"Unlocks\":[{joined}]}}");
    }
    format!("{{\"Success\":true,\"Unlocks\":[{joined}],\"HardcoreUnlocks\":[],\"ServerNow\":0}}")
}

fn matching_bracket(body: &str, open: usize) -> Option<usize> {
    let bytes = body.as_bytes();
    if bytes.get(open) != Some(&b'[') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = open;
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
        } else if c == b'[' {
            depth += 1;
        } else if c == b']' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn find_key(body: &str, key: &str) -> Option<usize> {
    let needle = format!("\"{key}\"");
    body.find(&needle).map(|at| at + needle.len())
}

/// What rcheevos asked for, answered from the network or from the card.
pub fn exchange(
    root: &Path,
    post: Option<&str>,
    live: Result<(u16, Vec<u8>), String>,
    now: i64,
) -> Result<(u16, Vec<u8>), String> {
    let Some(post) = post else {
        return live;
    };
    let kind = classify(post);
    match kind {
        Kind::Award { hardcore: true, .. } => {
            // Hardcore is not queued. The server does not accept an offline hardcore unlock,
            // and Slot does not enable hardcore.
            live
        }
        Kind::Award {
            id,
            hardcore: false,
            hash,
        } => {
            remember_unlock(root, id, &hash, now);
            match &live {
                Ok((status, body)) => {
                    let text = String::from_utf8_lossy(body);
                    let judged = judge(*status, &text);
                    let why = json_error(&text).unwrap_or_default();
                    // A failure while the game is open is retried by rcheevos itself.
                    // Counting it here would stretch the on-card backoff during that retry.
                    if judged != Judge::Retry {
                        settle_unlock(root, id, judged, &why);
                    }
                }
                Err(_) => {}
            }
            live
        }
        Kind::Ping => live.or_else(|_| Ok((200, br#"{"Success":true}"#.to_vec()))),
        Kind::Login { user, password } => {
            if let Ok((status, body)) = &live {
                if *status == 200 && json_success(&String::from_utf8_lossy(body)) {
                    let _ = save_login(root, body);
                }
                // The server answered. A rejected login is not replaced with a saved one.
                return live;
            }
            if password {
                return live;
            }
            if let Some(b) = cached_login(root, &user) {
                return Ok((200, b));
            }
            // The token is already on the card. A dropped network can still let rcheevos
            // start, and the set cache can load. A quote in the name is left for the server.
            let token = form(post, "t");
            if user.is_empty()
                || token.is_empty()
                || user.contains(['"', '\\'])
                || token.contains(['"', '\\'])
            {
                return live;
            }
            let body = format!(
                "{{\"Success\":true,\"User\":\"{user}\",\"Token\":\"{token}\",\"Score\":0,\"SoftcoreScore\":0}}"
            );
            let _ = save_login(root, body.as_bytes());
            Ok((200, body.into_bytes()))
        }
        Kind::Sets { key } => {
            if let Ok((status, body)) = &live {
                if *status == 200 && json_success(&String::from_utf8_lossy(body)) {
                    let _ = remember_set(root, &key, body, "");
                    return live;
                }
            }
            match read_bytes(&set_json(root, &key)) {
                Some(b) => Ok((200, b)),
                None => live,
            }
        }
        Kind::Session { game } => {
            let pending = pending_for(root, game);
            if let Ok((status, body)) = &live {
                if *status == 200 && json_success(&String::from_utf8_lossy(body)) {
                    let _ = save_bytes(&session_path(root, game), body);
                    let merged = merge_unlocks(&String::from_utf8_lossy(body), &pending);
                    return Ok((200, merged.into_bytes()));
                }
            }
            if let Some(saved) = read_bytes(&session_path(root, game)) {
                let merged = merge_unlocks(&String::from_utf8_lossy(&saved), &pending);
                return Ok((200, merged.into_bytes()));
            }
            if live.is_err() {
                let merged = merge_unlocks(
                    r#"{"Success":true,"Unlocks":[],"HardcoreUnlocks":[]}"#,
                    &pending,
                );
                return Ok((200, merged.into_bytes()));
            }
            live
        }
        Kind::Other => live,
    }
}

fn pending_for(root: &Path, _game: u32) -> Vec<(u32, i64)> {
    load_queue(root)
        .into_iter()
        .filter(|u| !u.refused)
        .map(|u| (u.id, u.when))
        .collect()
}

#[derive(Debug)]
enum Kind {
    Login {
        user: String,
        password: bool,
    },
    Sets {
        key: String,
    },
    Session {
        game: u32,
    },
    Award {
        id: u32,
        hardcore: bool,
        hash: String,
    },
    Ping,
    Other,
}

fn classify(post: &str) -> Kind {
    let r = form(post, "r");
    match r.as_str() {
        "login2" => Kind::Login {
            user: form_decode(&form(post, "u")),
            password: form_has(post, "p"),
        },
        "achievementsets" => Kind::Sets { key: set_key(post) },
        "startsession" => Kind::Session {
            game: form(post, "g").parse().unwrap_or(0),
        },
        "awardachievement" => Kind::Award {
            id: form(post, "a").parse().unwrap_or(0),
            hardcore: form(post, "h") == "1",
            hash: form(post, "m"),
        },
        "ping" => Kind::Ping,
        _ => Kind::Other,
    }
}

fn set_key(post: &str) -> String {
    let hash = form(post, "m");
    if hash.len() == 32 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        hash.to_ascii_lowercase()
    } else {
        format!(
            "g{}",
            form(post, "g")
                .chars()
                .filter(|c| c.is_ascii_digit())
                .collect::<String>()
        )
    }
}

fn form_has(post: &str, key: &str) -> bool {
    post.split('&')
        .any(|pair| pair.split_once('=').is_some_and(|(k, _)| k == key))
}

fn form(post: &str, key: &str) -> String {
    post.split('&')
        .find_map(|pair| {
            pair.split_once('=')
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| form_decode(v))
        })
        .unwrap_or_default()
}

fn form_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(n) = u8::from_str_radix(hex, 16) {
                    out.push(n);
                    i += 3;
                    continue;
                }
                out.push(bytes[i]);
            }
            c => out.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn key_ok(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 40
        && key.bytes().all(|b| b.is_ascii_alphanumeric())
        && !key.starts_with('.')
}

fn set_json(root: &Path, key: &str) -> PathBuf {
    root.join(SETS).join(format!("{key}.json"))
}

fn set_list(root: &Path, key: &str) -> PathBuf {
    root.join(SETS).join(format!("{key}.list"))
}

fn session_path(root: &Path, game: u32) -> PathBuf {
    root.join(SESSIONS).join(format!("{game}.json"))
}

fn save_login(root: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let path = root.join(LOGIN);
    save_bytes(&path, bytes)?;
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    Ok(())
}

fn save_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    slot_store::atomic_write(path, bytes)
}

fn read_bytes(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    (!bytes.is_empty()).then_some(bytes)
}

fn cached_login(root: &Path, user: &str) -> Option<Vec<u8>> {
    let bytes = read_bytes(&root.join(LOGIN))?;
    let text = String::from_utf8(bytes.clone()).ok()?;
    let saved = json_string(&text, "User")?;
    (saved.eq_ignore_ascii_case(user)).then_some(bytes)
}

fn remember_set(root: &Path, key: &str, body: &[u8], place: &str) -> Result<(), String> {
    if !key_ok(key) {
        return Err("bad set name".into());
    }
    let text = String::from_utf8_lossy(body);
    save_bytes(&set_json(root, key), body).map_err(|e| e.to_string())?;
    let (title, game_id, achs) = summarize(&text);
    let mut list = format!("game={}\nid={game_id}\n", flat(&title));
    if !place.is_empty() {
        list.push_str(&format!("where={place}\n"));
    }
    for ach in &achs {
        list.push_str(&format!(
            "a={}\t{}\t{}\t{}\t{}\n",
            ach.id,
            ach.points,
            flat(&ach.badge),
            flat(&ach.title),
            flat(&ach.description)
        ));
    }
    save_bytes(&set_list(root, key), list.as_bytes()).map_err(|e| e.to_string())
}

fn flat(s: &str) -> String {
    s.replace(['\n', '\r', '\t'], " ")
}

struct RawAch {
    id: u32,
    points: u32,
    badge: String,
    title: String,
    description: String,
}

fn summarize(body: &str) -> (String, u32, Vec<RawAch>) {
    let title = json_string(body, "Title").unwrap_or_else(|| "Achievements".into());
    let game_id = json_number(body, "GameId").unwrap_or(0) as u32;
    (title, game_id, achievements_in(body))
}

fn achievements_in(body: &str) -> Vec<RawAch> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = body[from..].find("\"Achievements\"") {
        let at = from + rel + "\"Achievements\"".len();
        from = at;
        let Some(bracket) = body[at..].find('[').map(|i| at + i) else {
            continue;
        };
        let Some(end) = matching_bracket(body, bracket) else {
            continue;
        };
        from = end + 1;
        let mut i = bracket + 1;
        while i < end {
            let rest = body[i..end].trim_start();
            let skipped = body[i..end].len() - rest.len();
            i += skipped;
            if rest.starts_with('{') {
                if let Some((fields, next)) = read_object(body, i) {
                    if let Some(ach) = ach_from(&fields) {
                        out.push(ach);
                    }
                    i = next;
                    continue;
                }
            }
            if rest.starts_with(']') || rest.is_empty() {
                break;
            }
            i += rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    out
}

fn ach_from(fields: &BTreeMap<String, String>) -> Option<RawAch> {
    let badge = fields.get("BadgeName")?.clone();
    if badge.is_empty() {
        return None;
    }
    Some(RawAch {
        id: fields.get("ID")?.parse().ok()?,
        points: fields
            .get("Points")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0),
        badge,
        title: fields.get("Title").cloned().unwrap_or_default(),
        description: fields.get("Description").cloned().unwrap_or_default(),
    })
}

/// Unlocked and locked badge URLs for a badge name.
pub fn badge_urls(name: &str) -> Vec<String> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Vec::new();
    }
    let base = "https://media.retroachievements.org/Badge";
    vec![
        format!("{base}/{name}.png"),
        format!("{base}/{name}_lock.png"),
    ]
}

fn read_object(body: &str, start: usize) -> Option<(BTreeMap<String, String>, usize)> {
    let bytes = body.as_bytes();
    if bytes.get(start) != Some(&b'{') {
        return None;
    }
    let mut fields = BTreeMap::new();
    let mut i = start + 1;
    loop {
        i = skip_ws(body, i)?;
        if bytes.get(i) == Some(&b'}') {
            return Some((fields, i + 1));
        }
        if bytes.get(i) == Some(&b',') {
            i += 1;
            continue;
        }
        let (key, next) = read_string(body, i)?;
        i = skip_ws(body, next)?;
        if bytes.get(i) != Some(&b':') {
            return None;
        }
        i = skip_ws(body, i + 1)?;
        let (value, next) = read_scalar(body, i)?;
        if let Some(value) = value {
            fields.insert(key, value);
        }
        i = next;
    }
}

fn read_scalar(body: &str, i: usize) -> Option<(Option<String>, usize)> {
    match body.as_bytes().get(i)? {
        b'"' => {
            let (s, n) = read_string(body, i)?;
            Some((Some(s), n))
        }
        b'-' | b'0'..=b'9' => {
            let rest = &body[i..];
            let end = rest
                .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.'))
                .unwrap_or(rest.len());
            let n = if rest[..end].contains('.') {
                rest[..end]
                    .parse::<f64>()
                    .ok()
                    .map(|n| n as i64)
                    .unwrap_or(0)
                    .to_string()
            } else {
                rest[..end].to_string()
            };
            Some((Some(n), i + end))
        }
        _ => {
            let n = skip_value(body, i)?;
            Some((None, n))
        }
    }
}

fn read_string(body: &str, i: usize) -> Option<(String, usize)> {
    let bytes = body.as_bytes();
    if bytes.get(i) != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut j = i + 1;
    while j < bytes.len() {
        match bytes[j] {
            b'"' => return Some((out, j + 1)),
            b'\\' => {
                j += 1;
                match bytes.get(j)? {
                    b'n' => out.push('\n'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(bytes.get(j + 1..j + 5)?).ok()?;
                        out.push(char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('?'));
                        j += 4;
                    }
                    c => out.push(*c as char),
                }
            }
            c => out.push(c as char),
        }
        j += 1;
    }
    None
}

fn skip_ws(body: &str, mut i: usize) -> Option<usize> {
    while body
        .as_bytes()
        .get(i)
        .is_some_and(|c| c.is_ascii_whitespace())
    {
        i += 1;
    }
    Some(i)
}

fn skip_value(body: &str, i: usize) -> Option<usize> {
    let bytes = body.as_bytes();
    match bytes.get(i)? {
        b'"' => read_string(body, i).map(|(_, n)| n),
        b'{' | b'[' => {
            let open = bytes[i];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 0i32;
            let mut j = i;
            let mut in_str = false;
            let mut esc = false;
            while j < bytes.len() {
                let c = bytes[j];
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
                } else if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(j + 1);
                    }
                }
                j += 1;
            }
            None
        }
        _ => {
            let rest = &body[i..];
            let end = rest.find([',', '}', ']']).unwrap_or(rest.len());
            Some(i + end)
        }
    }
}

fn json_success(body: &str) -> bool {
    let Some(at) = find_key(body, "Success") else {
        return false;
    };
    body[at..]
        .trim_start()
        .trim_start_matches(':')
        .trim_start()
        .starts_with("true")
}

fn json_error(body: &str) -> Option<String> {
    json_string(body, "Error")
}

fn json_string(body: &str, key: &str) -> Option<String> {
    let at = find_key(body, key)?;
    let rest = body[at..].trim_start().strip_prefix(':')?.trim_start();
    let start = body.len() - rest.len();
    read_string(body, start).map(|(s, _)| s)
}

fn json_number(body: &str, key: &str) -> Option<i64> {
    let at = find_key(body, key)?;
    let rest = body[at..].trim_start().strip_prefix(':')?.trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// A game whose set should be cached: a favorite, a recent play, or an explicit queue entry.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub platform: Platform,
    pub stem: String,
}

fn platform_from_dir(name: &str) -> Option<Platform> {
    Platform::ALL
        .into_iter()
        .find(|p| name.eq_ignore_ascii_case(p.dir_name()))
}

fn cache_key(platform: Platform, stem: &str) -> CacheKey {
    CacheKey {
        platform,
        stem: stem.to_string(),
    }
}

fn place_of(platform: Platform, stem: &str) -> String {
    format!("{}\t{}", platform.dir_name(), flat(stem))
}

fn load_key_lines(root: &Path, rel: &str) -> Vec<CacheKey> {
    let text = std::fs::read_to_string(root.join(rel)).unwrap_or_default();
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((dir, stem)) = line.split_once('\t') else {
            continue;
        };
        let Some(platform) = platform_from_dir(dir.trim()) else {
            continue;
        };
        let stem = stem.trim();
        if stem.is_empty() {
            continue;
        }
        let key = cache_key(platform, stem);
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

fn save_key_lines(root: &Path, rel: &str, keys: &[CacheKey]) {
    let mut text = String::new();
    for key in keys {
        text.push_str(&format!("{}\t{}\n", key.platform.dir_name(), key.stem));
    }
    let path = root.join(rel);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = slot_store::atomic_write(&path, text.as_bytes());
}

/// Games waiting to be cached because they were starred or launched.
pub fn cache_queue(root: &Path) -> Vec<CacheKey> {
    load_key_lines(root, CACHE_QUEUE)
}

/// Remember this game for the next background cache pass.
pub fn queue_set(root: &Path, platform: Platform, stem: &str) {
    if stem.is_empty() {
        return;
    }
    let mut keys = load_key_lines(root, CACHE_QUEUE);
    let key = cache_key(platform, stem);
    if !keys.contains(&key) {
        keys.push(key);
        save_key_lines(root, CACHE_QUEUE, &keys);
    }
    // A fresh star or play should try again even if an earlier pass found no set.
    let misses: Vec<CacheKey> = load_key_lines(root, CACHE_MISSES)
        .into_iter()
        .filter(|k| k != &cache_key(platform, stem))
        .collect();
    save_key_lines(root, CACHE_MISSES, &misses);
}

/// Stop prioritizing this game. An already cached set is left on the card.
pub fn unqueue_set(root: &Path, platform: Platform, stem: &str) {
    let keys: Vec<CacheKey> = load_key_lines(root, CACHE_QUEUE)
        .into_iter()
        .filter(|k| k.platform != platform || k.stem != stem)
        .collect();
    save_key_lines(root, CACHE_QUEUE, &keys);
}

fn cache_misses(root: &Path) -> HashSet<CacheKey> {
    load_key_lines(root, CACHE_MISSES).into_iter().collect()
}

fn remember_miss(root: &Path, key: &CacheKey) {
    let mut keys = load_key_lines(root, CACHE_MISSES);
    if !keys.contains(key) {
        keys.push(key.clone());
        save_key_lines(root, CACHE_MISSES, &keys);
    }
}

fn forget_queued(root: &Path, key: &CacheKey) {
    let keys: Vec<CacheKey> = load_key_lines(root, CACHE_QUEUE)
        .into_iter()
        .filter(|k| k != key)
        .collect();
    save_key_lines(root, CACHE_QUEUE, &keys);
}

/// Favorites first, then the most recently played games up to `RECENT_SET_CACHE`,
/// then any leftover queue entries that are still in `carts`. Each game once.
pub fn cache_targets(carts: &[Cart], library: &Library, queued: &[CacheKey]) -> Vec<Cart> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for cart in carts {
        if library.get(cart.platform, &cart.stem).favorite
            && seen.insert(cache_key(cart.platform, &cart.stem))
        {
            out.push(cart.clone());
        }
    }
    let mut recent: Vec<&Cart> = carts
        .iter()
        .filter(|c| library.get(c.platform, &c.stem).last_played > 0)
        .collect();
    recent.sort_by_key(|c| std::cmp::Reverse(library.get(c.platform, &c.stem).last_played));
    for cart in recent.into_iter().take(RECENT_SET_CACHE) {
        if seen.insert(cache_key(cart.platform, &cart.stem)) {
            out.push(cart.clone());
        }
    }
    for key in queued {
        if seen.contains(key) {
            continue;
        }
        if let Some(cart) = carts
            .iter()
            .find(|c| c.platform == key.platform && c.stem == key.stem)
        {
            seen.insert(key.clone());
            out.push(cart.clone());
        }
    }
    out
}

/// Keys that the next cache pass should consider, from library.txt and the queue.
pub fn cache_wanted(library: &Library, queued: &[CacheKey]) -> Vec<CacheKey> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (platform, stem, record) in library.records() {
        if record.favorite && seen.insert(cache_key(platform, stem)) {
            out.push(cache_key(platform, stem));
        }
    }
    let mut recent: Vec<(i64, CacheKey)> = library
        .records()
        .filter(|(_, _, r)| r.last_played > 0)
        .map(|(p, s, r)| (r.last_played, cache_key(p, s)))
        .collect();
    recent.sort_by_key(|(t, _)| std::cmp::Reverse(*t));
    for (_, key) in recent.into_iter().take(RECENT_SET_CACHE) {
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    for key in queued {
        if seen.insert(key.clone()) {
            out.push(key.clone());
        }
    }
    out
}

fn cached_places(root: &Path) -> HashSet<CacheKey> {
    let mut out = HashSet::new();
    let dir = root.join(SETS);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "list") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("where=") else {
                continue;
            };
            let Some((dir, stem)) = rest.split_once('\t') else {
                continue;
            };
            let Some(platform) = platform_from_dir(dir) else {
                continue;
            };
            out.insert(cache_key(platform, stem));
        }
    }
    out
}

fn set_cached_hash(root: &Path, hash: &str) -> bool {
    let key = hash.to_ascii_lowercase();
    set_json(root, &key).is_file()
}

/// One game on a system that can be cached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheGame {
    pub key: CacheKey,
    pub title: String,
    pub cached: bool,
    pub size: Option<u64>,
}

/// A system present on the card, with the games that can have a RetroAchievements set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheSystem {
    pub platform: Platform,
    pub games: Vec<CacheGame>,
}

/// Systems on the card that RetroAchievements supports, and the average size of
/// sets already cached, when that is known.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct CacheCatalog {
    pub systems: Vec<CacheSystem>,
    pub avg_size: Option<u64>,
}

/// A remembered picker line: every game on a system, or one game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickLine {
    All(Platform),
    Game(CacheKey),
}

/// Systems on the card that can be cached, built from a prepared cart list.
pub fn cache_catalog_from(carts: &[Cart], root: &Path) -> CacheCatalog {
    let cached = cached_places(root);
    let sizes = cached_set_sizes(root);
    let mut by_platform: Vec<(Platform, Vec<CacheGame>)> = Vec::new();
    for cart in carts {
        if console_of(cart.platform).is_none() {
            continue;
        }
        let key = cache_key(cart.platform, &cart.stem);
        let game = CacheGame {
            cached: cached.contains(&key),
            size: sizes.get(&key).copied(),
            title: cart.shown_name().into_owned(),
            key,
        };
        match by_platform.iter_mut().find(|(p, _)| *p == cart.platform) {
            Some((_, games)) => games.push(game),
            None => by_platform.push((cart.platform, vec![game])),
        }
    }
    by_platform.sort_by_key(|(p, _)| Platform::ALL.iter().position(|x| *x == *p).unwrap_or(99));
    for (_, games) in &mut by_platform {
        games.sort_by(|a, b| a.title.to_ascii_lowercase().cmp(&b.title.to_ascii_lowercase()).then(a.key.stem.cmp(&b.key.stem)));
    }
    let systems = by_platform
        .into_iter()
        .map(|(platform, games)| CacheSystem { platform, games })
        .collect::<Vec<_>>();
    let known: Vec<u64> = systems
        .iter()
        .flat_map(|s| s.games.iter())
        .filter_map(|g| g.size)
        .collect();
    let avg_size = if known.is_empty() {
        None
    } else {
        Some(known.iter().sum::<u64>() / known.len() as u64)
    };
    CacheCatalog { systems, avg_size }
}

/// Systems on the card that can be cached.
pub fn cache_catalog(root: &Path) -> CacheCatalog {
    cache_catalog_from(&slot_store::scan(root).unwrap_or_default(), root)
}

fn cached_set_sizes(root: &Path) -> std::collections::HashMap<CacheKey, u64> {
    let mut out = std::collections::HashMap::new();
    let dir = root.join(SETS);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "list") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let list_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let json_len = path
            .with_extension("json")
            .metadata()
            .map(|m| m.len())
            .unwrap_or(0);
        let size = list_len + json_len;
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("where=") else {
                continue;
            };
            let Some((dir_name, stem)) = rest.split_once('\t') else {
                continue;
            };
            let Some(platform) = platform_from_dir(dir_name) else {
                continue;
            };
            out.insert(cache_key(platform, stem), size);
        }
    }
    out
}

/// A short size for the picker, such as `80 KB` or `1.2 MB`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{} KB", (bytes + 512) / 1024)
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        if mb >= 10.0 {
            format!("{mb:.0} MB")
        } else {
            format!("{mb:.1} MB")
        }
    }
}

/// Total bytes for `count` unknown items given an optional average, plus known sizes.
/// The bool is true when some of the total was estimated.
pub fn approx_bytes(known: &[u64], unknown: usize, avg: Option<u64>) -> Option<(u64, bool)> {
    let sum: u64 = known.iter().sum();
    if unknown == 0 {
        return Some((sum, false));
    }
    let avg = avg.or_else(|| {
        if known.is_empty() {
            None
        } else {
            Some(sum / known.len() as u64)
        }
    })?;
    Some((sum + avg * unknown as u64, true))
}

/// Size of the selected games in a catalog, when it can be estimated.
pub fn selection_size(
    systems: &[CacheSystem],
    selected: &HashSet<CacheKey>,
    avg: Option<u64>,
) -> Option<(u64, bool)> {
    let mut known = Vec::new();
    let mut unknown = 0usize;
    for sys in systems {
        for game in &sys.games {
            if !selected.contains(&game.key) {
                continue;
            }
            match game.size {
                Some(n) => known.push(n),
                None => unknown += 1,
            }
        }
    }
    if known.is_empty() && unknown == 0 {
        return Some((0, false));
    }
    approx_bytes(&known, unknown, avg)
}

fn load_pick_lines(root: &Path) -> Vec<PickLine> {
    let text = std::fs::read_to_string(root.join(CACHE_PICK)).unwrap_or_default();
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((dir, stem)) = line.split_once('\t') else {
            continue;
        };
        let Some(platform) = platform_from_dir(dir.trim()) else {
            continue;
        };
        let stem = stem.trim();
        if stem == "*" {
            out.push(PickLine::All(platform));
        } else if !stem.is_empty() {
            out.push(PickLine::Game(cache_key(platform, stem)));
        }
    }
    out
}

/// The last picker selection, applied to the systems now on the card.
pub fn load_pick(root: &Path, systems: &[CacheSystem]) -> HashSet<CacheKey> {
    selected_from_pick(systems, &load_pick_lines(root))
}

/// Expand a saved picker file against the systems now on the card.
pub fn selected_from_pick(systems: &[CacheSystem], lines: &[PickLine]) -> HashSet<CacheKey> {
    let mut out = HashSet::new();
    for line in lines {
        match line {
            PickLine::All(platform) => {
                if let Some(sys) = systems.iter().find(|s| s.platform == *platform) {
                    out.extend(sys.games.iter().map(|g| g.key.clone()));
                }
            }
            PickLine::Game(key) => {
                if systems
                    .iter()
                    .any(|s| s.games.iter().any(|g| g.key == *key))
                {
                    out.insert(key.clone());
                }
            }
        }
    }
    out
}

/// Remember the picker selection. A fully selected system is stored as `DIR\t*`.
pub fn save_pick(root: &Path, systems: &[CacheSystem], selected: &HashSet<CacheKey>) {
    let mut text = String::from("# Last Cache achievement sets selection. A star in the second column means every game on that system.\n");
    for sys in systems {
        let n = sys.games.len();
        let count = sys.games.iter().filter(|g| selected.contains(&g.key)).count();
        if count == 0 {
            continue;
        }
        if n > 0 && count == n {
            text.push_str(&format!("{}\t*\n", sys.platform.dir_name()));
        } else {
            for game in &sys.games {
                if selected.contains(&game.key) {
                    text.push_str(&format!(
                        "{}\t{}\n",
                        game.key.platform.dir_name(),
                        game.key.stem
                    ));
                }
            }
        }
    }
    let path = root.join(CACHE_PICK);
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = slot_store::atomic_write(&path, text.as_bytes());
}

/// Carts matching `keys`, in that order, that are still in `carts`.
pub fn carts_for_keys(carts: &[Cart], keys: &[CacheKey]) -> Vec<Cart> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for key in keys {
        if !seen.insert(key.clone()) {
            continue;
        }
        if let Some(cart) = carts
            .iter()
            .find(|c| c.platform == key.platform && c.stem == key.stem)
        {
            out.push(cart.clone());
        }
    }
    out
}


fn cache_gate() -> &'static Mutex<Gate> {
    static GATE: OnceLock<Mutex<Gate>> = OnceLock::new();
    GATE.get_or_init(|| {
        Mutex::new(Gate {
            next: Instant::now(),
            running: false,
        })
    })
}

fn cache_line() -> &'static Mutex<Option<String>> {
    static LINE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    LINE.get_or_init(|| Mutex::new(None))
}

fn cache_result() -> &'static Mutex<Option<(usize, usize, usize)>> {
    static RESULT: OnceLock<Mutex<Option<(usize, usize, usize)>>> = OnceLock::new();
    RESULT.get_or_init(|| Mutex::new(None))
}

/// Counts from the cache pass that just finished, if any, then clears them.
pub fn take_cache_result() -> Option<(usize, usize, usize)> {
    cache_result().lock().unwrap_or_else(|e| e.into_inner()).take()
}
/// Test-only: pretend a cache pass is running (or has just finished).
#[cfg(test)]
pub fn test_set_cache_state(
    line: Option<&str>,
    running: bool,
    result: Option<(usize, usize, usize)>,
) {
    cache_gate().lock().unwrap_or_else(|e| e.into_inner()).running = running;
    *cache_line().lock().unwrap_or_else(|e| e.into_inner()) = line.map(str::to_string);
    *cache_result().lock().unwrap_or_else(|e| e.into_inner()) = result;
}


/// Latest "Caching 3/12: Sonic" line, only while a cache pass is actually running.
/// A cancelled or finished pass does not leave a stale line on the screen.
pub fn cache_status() -> Option<String> {
    if !cache_busy() {
        return None;
    }
    cache_line()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

pub fn cache_busy() -> bool {
    cache_gate()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .running
}

fn try_begin_cache() -> bool {
    let mut gate = cache_gate().lock().unwrap_or_else(|e| e.into_inner());
    if gate.running {
        return false;
    }
    gate.running = true;
    true
}

fn end_cache() {
    if let Ok(mut gate) = cache_gate().lock() {
        gate.running = false;
    }
    *cache_line().lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn pending_uncached(root: &Path, retry_misses: bool) -> bool {
    let wanted = cache_wanted(&Library::read(root), &cache_queue(root));
    if wanted.is_empty() {
        return false;
    }
    let cached = cached_places(root);
    let misses = if retry_misses {
        HashSet::new()
    } else {
        cache_misses(root)
    };
    wanted
        .iter()
        .any(|k| !cached.contains(k) && !misses.contains(k))
}

/// When Wi-Fi is up, cache any starred or recently played set that is not on the card yet.
pub fn poke_cache(root: &Path, user: &str, token: &str) {
    if cache_busy() || !pending_uncached(root, false) {
        return;
    }
    let _ = CacheRun::begin(
        root.to_path_buf(),
        user.to_string(),
        token.to_string(),
        false,
        None,
    );
}

/// Asks the server for one hashed game's set and session, and stores both.
pub fn cache_hash(
    root: &Path,
    user: &str,
    token: &str,
    hash: &str,
    place: &str,
    mut send: impl FnMut(&str) -> Result<(u16, Vec<u8>), String>,
) -> Result<bool, String> {
    let sets =
        slot_cheevos::sets_body(user, token, hash).ok_or("could not build the set request")?;
    let (status, body) = send(&sets)?;
    let text = String::from_utf8_lossy(&body);
    if status != 200 || !json_success(&text) {
        return Ok(false);
    }
    let key = hash.to_ascii_lowercase();
    remember_set(root, &key, &body, place)?;
    let game_id = json_number(&text, "GameId").unwrap_or(0) as u32;
    if game_id != 0 {
        if let Some(session) = slot_cheevos::session_body(user, token, game_id, hash) {
            if let Ok((status, body)) = send(&session) {
                if status == 200 && json_success(&String::from_utf8_lossy(&body)) {
                    let _ = save_bytes(&session_path(root, game_id), &body);
                }
            }
        }
    }
    let mut urls = Vec::new();
    for ach in achievements_in(&text) {
        urls.extend(badge_urls(&ach.badge));
    }
    if !urls.is_empty() {
        crate::badge::ensure(root, &urls);
    }
    Ok(true)
}

pub enum CacheEvent {
    Step {
        done: usize,
        total: usize,
        name: String,
    },
    Done {
        cached: usize,
        missed: usize,
        failed: usize,
    },
}

pub struct CacheRun {
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    rx: mpsc::Receiver<CacheEvent>,
}

impl CacheRun {
    /// Background / default pass: favorites, the last `RECENT_SET_CACHE` played
    /// games, and anything still queued. Already cached sets are skipped, so a
    /// cancelled pass can be resumed.
    pub fn start(root: PathBuf, user: String, token: String) -> Self {
        Self::begin(root, user, token, true, None)
    }

    /// Cache only these games (the picker's Cache selected button).
    /// Already cached sets are skipped. Earlier misses are tried again.
    pub fn start_keys(root: PathBuf, user: String, token: String, keys: Vec<CacheKey>) -> Self {
        Self::begin(root, user, token, true, Some(keys))
    }

    fn begin(
        root: PathBuf,
        user: String,
        token: String,
        retry_misses: bool,
        explicit: Option<Vec<CacheKey>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        if !try_begin_cache() {
            let _ = tx.send(CacheEvent::Done {
                cached: 0,
                missed: 0,
                failed: 0,
            });
            return Self { cancel, rx };
        }
        let flag = cancel.clone();
        let spawned = std::thread::Builder::new()
            .name("slot-ra-cache".into())
            .spawn(move || {
                struct CacheGuard;
                impl Drop for CacheGuard {
                    fn drop(&mut self) {
                        end_cache();
                    }
                }
                let _guard = CacheGuard;
                let carts = slot_store::scan(&root).unwrap_or_default();
                let library = Library::read(&root);
                let queued = cache_queue(&root);
                let mut targets = if let Some(keys) = explicit {
                    carts_for_keys(&carts, &keys)
                } else {
                    cache_targets(&carts, &library, &queued)
                };
                let cached_already = cached_places(&root);
                let misses = if retry_misses {
                    HashSet::new()
                } else {
                    cache_misses(&root)
                };
                targets.retain(|cart| {
                    let key = cache_key(cart.platform, &cart.stem);
                    !cached_already.contains(&key) && !misses.contains(&key)
                });
                let total = targets.len();
                let mut cached = 0;
                let mut missed = 0;
                let mut failed = 0;
                for (done, cart) in targets.into_iter().enumerate() {
                    if flag.load(std::sync::atomic::Ordering::Relaxed) {
                        break;
                    }
                    let name = cart.stem.clone();
                    let line = format!("Caching {}/{total}: {name}", done + 1);
                    *cache_line().lock().unwrap_or_else(|e| e.into_inner()) = Some(line);
                    let _ = tx.send(CacheEvent::Step { done, total, name });
                    let key = cache_key(cart.platform, &cart.stem);
                    let Some(console) = console_of(cart.platform) else {
                        missed += 1;
                        remember_miss(&root, &key);
                        forget_queued(&root, &key);
                        continue;
                    };
                    let Some(hash) = slot_cheevos::hash_rom(console, &cart.rom) else {
                        missed += 1;
                        remember_miss(&root, &key);
                        forget_queued(&root, &key);
                        continue;
                    };
                    if set_cached_hash(&root, &hash) {
                        cached += 1;
                        forget_queued(&root, &key);
                        continue;
                    }
                    let place = place_of(cart.platform, &cart.stem);
                    match cache_hash(&root, &user, &token, &hash, &place, post_form) {
                        Ok(true) => {
                            cached += 1;
                            forget_queued(&root, &key);
                        }
                        Ok(false) => {
                            missed += 1;
                            remember_miss(&root, &key);
                            forget_queued(&root, &key);
                        }
                        Err(_) => failed += 1,
                    }
                    for _ in 0..4 {
                        if flag.load(std::sync::atomic::Ordering::Relaxed) {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
                let _ = tx.send(CacheEvent::Done {
                    cached,
                    missed,
                    failed,
                });
                *cache_result().lock().unwrap_or_else(|e| e.into_inner()) =
                    Some((cached, missed, failed));
                if cached > 0 {
                    let note = if cached == 1 {
                        "Cached 1 achievement set".into()
                    } else {
                        format!("Cached {cached} achievement sets")
                    };
                    *notes().lock().unwrap_or_else(|e| e.into_inner()) = Some(note);
                }
            });
        if spawned.is_err() {
            end_cache();
        }
        Self { cancel, rx }
    }

    pub fn try_recv(&self) -> Result<CacheEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub fn cancel(&self) {
        self.cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

fn console_of(platform: Platform) -> Option<Console> {
    Console::from_id(platform.def().ra)
}

fn post_form(body: &str) -> Result<(u16, Vec<u8>), String> {
    let reply = crate::net::send(&crate::net::Request {
        tls: true,
        server: "retroachievements.org:443",
        method: "POST",
        path: "/dorequest.php",
        form: Some(body),
        json: None,
        agent: None,
        bearer: None,
        limit: 4 << 20,
    })?;
    Ok((reply.status, reply.body))
}

pub struct SyncOutcome {
    pub sent: u32,
    pub refused: Vec<String>,
    pub retry_in: Option<u64>,
}

/// Sends up to `limit` waiting unlocks. A network failure stops the batch and says how long to wait.
pub fn sync_some(
    root: &Path,
    user: &str,
    token: &str,
    now: i64,
    limit: u32,
    mut send: impl FnMut(&str) -> Result<(u16, Vec<u8>), String>,
) -> SyncOutcome {
    let mut sent = 0;
    let mut refused = Vec::new();
    let mut retry_in = None;
    for _ in 0..limit {
        let next = load_queue(root).into_iter().find(|u| !u.refused);
        let Some(row) = next else { break };
        let seconds = now.saturating_sub(row.when).max(0) as u32;
        let Some(body) = slot_cheevos::award_body(user, token, row.id, false, &row.hash, seconds)
        else {
            settle_unlock(root, row.id, Judge::Refused, "could not build the unlock");
            refused.push(format!("{} could not be built", row.id));
            continue;
        };
        if body.contains("h=1") {
            settle_unlock(root, row.id, Judge::Refused, "hardcore is not submitted");
            refused.push(format!("{} was hardcore", row.id));
            continue;
        }
        match send(&body) {
            Ok((status, bytes)) => {
                let text = String::from_utf8_lossy(&bytes);
                match judge(status, &text) {
                    Judge::Synced => {
                        settle_unlock(root, row.id, Judge::Synced, "");
                        sent += 1;
                    }
                    Judge::Retry => {
                        settle_unlock(root, row.id, Judge::Retry, "");
                        let attempts = load_queue(root)
                            .into_iter()
                            .find(|u| u.id == row.id)
                            .map(|u| u.attempts)
                            .unwrap_or(1);
                        retry_in = Some(backoff_secs(attempts));
                        break;
                    }
                    Judge::Refused => {
                        let why = json_error(&text)
                            .unwrap_or_else(|| format!("the server said {status}"));
                        settle_unlock(root, row.id, Judge::Refused, &why);
                        refused.push(why);
                    }
                }
            }
            Err(_) => {
                settle_unlock(root, row.id, Judge::Retry, "");
                let attempts = load_queue(root)
                    .into_iter()
                    .find(|u| u.id == row.id)
                    .map(|u| u.attempts)
                    .unwrap_or(1);
                retry_in = Some(backoff_secs(attempts));
                break;
            }
        }
    }
    SyncOutcome {
        sent,
        refused,
        retry_in,
    }
}

struct Gate {
    next: Instant,
    running: bool,
}

fn gate() -> &'static Mutex<Gate> {
    static GATE: OnceLock<Mutex<Gate>> = OnceLock::new();
    GATE.get_or_init(|| {
        Mutex::new(Gate {
            next: Instant::now(),
            running: false,
        })
    })
}

fn notes() -> &'static Mutex<Option<String>> {
    static NOTE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    NOTE.get_or_init(|| Mutex::new(None))
}

/// A line for a toast, once, after a queued unlock was sent or refused.
pub fn take_note() -> Option<String> {
    notes().lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// When Wi-Fi is up, send one waiting unlock if its backoff has elapsed.
pub fn poke(root: &Path, user: &str, token: &str) {
    if waiting(root) == 0 {
        return;
    }
    {
        let mut gate = gate().lock().unwrap_or_else(|e| e.into_inner());
        if gate.running || Instant::now() < gate.next {
            return;
        }
        gate.running = true;
    }
    let root = root.to_path_buf();
    let user = user.to_string();
    let token = token.to_string();
    let spawned = std::thread::Builder::new()
        .name("slot-ra-sync".into())
        .spawn(move || {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let outcome = sync_some(&root, &user, &token, now, 1, post_form);
            let wait = outcome.retry_in.unwrap_or(2);
            if let Ok(mut gate) = gate().lock() {
                gate.running = false;
                gate.next = Instant::now() + Duration::from_secs(wait);
            }
            let note = if outcome.sent > 0 {
                Some(if outcome.sent == 1 {
                    "Achievement sent".into()
                } else {
                    format!("{} achievements sent", outcome.sent)
                })
            } else {
                outcome
                    .refused
                    .into_iter()
                    .next()
                    .map(|why| format!("Achievement not accepted: {why}"))
            };
            if let Some(note) = note {
                *notes().lock().unwrap_or_else(|e| e.into_inner()) = Some(note);
            }
        });
    if spawned.is_err() {
        if let Ok(mut gate) = gate().lock() {
            gate.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_fixture() -> &'static str {
        r#"{"Success":true,"GameId":99,"Title":"Tetris","Sets":[{"Achievements":[
            {"ID":5,"Title":"Clear","Description":"Clear a line","Points":10,"MemAddr":"0xH123=1_\"x\"","BadgeName":"00010"},
            {"ID":6,"Title":"Quote","Description":"Say \"hi\"","Points":5,"BadgeName":"00011"}
        ]}]}"#
    }

    #[test]
    fn a_torn_queue_comes_back_and_a_hardcore_unlock_is_not_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let post = "r=awardachievement&u=ada&t=token&a=5&h=0&m=0123456789abcdef0123456789abcdef";
        let err = exchange(root, Some(post), Err("offline".into()), 1_700_000_000);
        assert!(err.is_err());
        let rows = load_queue(root);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, 5);
        assert_eq!(rows[0].when, 1_700_000_000);
        exchange(root, Some(post), Err("offline".into()), 1_700_000_100).ok();
        assert_eq!(load_queue(root)[0].when, 1_700_000_000);

        std::fs::write(queue_path(root), b"slot-unlocks 1\nnot a checksum\n").unwrap();
        std::fs::write(queue_path(root).with_file_name("queue.txt.9.tmp"), b"torn").unwrap();
        let back = load_queue(root);
        assert_eq!(back[0].id, 5);

        let hard = "r=awardachievement&a=9&h=1&m=0123456789abcdef0123456789abcdef";
        assert!(exchange(root, Some(hard), Err("offline".into()), 10).is_err());
        assert!(load_queue(root).iter().all(|u| u.id != 9));
    }

    #[test]
    fn a_cached_set_loads_offline_and_a_queued_unlock_is_already_earned() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ask = "r=achievementsets&u=ada&t=token&m=0123456789abcdef0123456789abcdef";
        assert!(exchange(root, Some(ask), Err("offline".into()), 1).is_err());
        exchange(root, Some(ask), Ok((200, set_fixture().into())), 1).unwrap();
        let cached = exchange(root, Some(ask), Err("offline".into()), 2).unwrap();
        assert_eq!(cached.0, 200);
        assert!(String::from_utf8_lossy(&cached.1).contains("Clear"));
        let cards = games(root);
        assert_eq!(cards[0].title, "Tetris");
        assert_eq!(cards[0].achievements.len(), 2);
        assert!(!cards[0].achievements[0].unlocked);

        remember_unlock(root, 5, "0123456789abcdef0123456789abcdef", 50);
        let session = "r=startsession&u=ada&t=token&g=99&h=0&m=0123456789abcdef0123456789abcdef";
        let live = br#"{"Success":true,"Unlocks":[],"HardcoreUnlocks":[]}"#;
        let merged = exchange(root, Some(session), Ok((200, live.to_vec())), 60).unwrap();
        let text = String::from_utf8_lossy(&merged.1);
        assert!(text.contains("\"ID\":5"));
        let again = exchange(root, Some(session), Err("offline".into()), 70).unwrap();
        assert!(String::from_utf8_lossy(&again.1).contains("\"ID\":5"));
        assert!(games(root)[0]
            .achievements
            .iter()
            .any(|a| a.id == 5 && a.unlocked && a.waiting));
    }

    #[test]
    fn a_sent_unlock_leaves_the_queue_and_a_refusal_stays_unsent() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        remember_unlock(root, 5, "0123456789abcdef0123456789abcdef", 1_000);
        remember_unlock(root, 6, "0123456789abcdef0123456789abcdef", 1_000);
        let outcome = sync_some(root, "ada", "token", 1_030, 1, |_| {
            Ok((200, br#"{"Success":true,"AchievementID":5}"#.to_vec()))
        });
        assert_eq!(outcome.sent, 1);
        assert!(load_queue(root).iter().all(|u| u.id != 5));
        let outcome = sync_some(root, "ada", "token", 1_040, 1, |_| {
            Ok((
                200,
                br#"{"Success":false,"Error":"softcore unlocks are closed for this achievement"}"#
                    .to_vec(),
            ))
        });
        assert_eq!(outcome.sent, 0);
        assert!(outcome.refused.iter().any(|w| w.contains("closed")));
        let row = load_queue(root).into_iter().find(|u| u.id == 6).unwrap();
        assert!(row.refused);
        let again = sync_some(root, "ada", "token", 1_050, 5, |_| {
            panic!("refused unlocks are not sent again")
        });
        assert_eq!(again.sent, 0);

        remember_unlock(root, 7, "0123456789abcdef0123456789abcdef", 1_000);
        let outcome = sync_some(root, "ada", "token", 1_100, 1, |_| Err("offline".into()));
        assert_eq!(outcome.retry_in, Some(0));
        let outcome = sync_some(root, "ada", "token", 1_100, 1, |_| Err("offline".into()));
        assert_eq!(outcome.retry_in, Some(1));
        assert_eq!(backoff_secs(8), 64);
        assert_eq!(backoff_secs(9), 120);
    }

    #[test]
    fn the_award_body_is_softcore_and_signed_by_rcheevos() {
        let body = slot_cheevos::award_body(
            "ada",
            "token",
            5,
            false,
            "0123456789abcdef0123456789abcdef",
            0,
        )
        .unwrap();
        assert!(body.contains("r=awardachievement"));
        assert!(body.contains("h=0"));
        assert!(body.contains("a=5"));
        assert!(body.contains("v="));
        assert!(!body.contains("o="));
        let later = slot_cheevos::award_body(
            "ada",
            "token",
            5,
            false,
            "0123456789abcdef0123456789abcdef",
            90,
        )
        .unwrap();
        assert!(later.contains("o=90"));
        assert!(!later.contains("h=1"));
    }

    #[test]
    fn a_saved_token_logs_in_when_the_network_is_down() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ask = "r=login2&u=Ada&t=secret-token";
        let logged = exchange(root, Some(ask), Err("offline".into()), 1).unwrap();
        assert!(String::from_utf8_lossy(&logged.1).contains("\"Token\":\"secret-token\""));
        let again = exchange(root, Some(ask), Err("offline".into()), 2).unwrap();
        assert!(String::from_utf8_lossy(&again.1).contains("Ada"));
        let rejected = br#"{"Success":false,"Error":"Invalid user/token combination"}"#;
        let answer = exchange(root, Some(ask), Ok((200, rejected.to_vec())), 3).unwrap();
        assert!(String::from_utf8_lossy(&answer.1).contains("Invalid"));
    }

    fn test_cart(platform: Platform, stem: &str) -> Cart {
        Cart {
            platform,
            stem: stem.into(),
            rom: std::path::PathBuf::from(format!("Games/{}/{stem}.rom", platform.dir_name())),
            label: None,
            famicom_label: None,
            title: stem.into(),
            code: String::new(),
            shell: None,
        }
    }

    #[test]
    fn cache_targets_are_favorites_and_the_most_recently_played() {
        let carts: Vec<Cart> = (0..40)
            .map(|i| test_cart(Platform::Gba, &format!("Game{i:02}")))
            .collect();
        let mut library = Library::default();
        library.get_mut(Platform::Gba, "Game00").favorite = true;
        library.get_mut(Platform::Gba, "Game01").favorite = true;
        library.get_mut(Platform::Gba, "Game01").last_played = 5;
        for i in 10..40 {
            library
                .get_mut(Platform::Gba, &format!("Game{i:02}"))
                .last_played = i as i64;
        }
        let got = cache_targets(&carts, &library, &[]);
        let stems: Vec<_> = got.iter().map(|c| c.stem.as_str()).collect();
        assert_eq!(stems[0], "Game00");
        assert_eq!(stems[1], "Game01");
        // 20 most recent: Game39 .. Game20. Game01 is already included as a favorite.
        assert_eq!(stems.len(), 22);
        assert_eq!(stems[2], "Game39");
        assert_eq!(stems[21], "Game20");
        assert!(!stems.contains(&"Game19"));
        assert!(!stems.contains(&"Game02"));
    }

    #[test]
    fn a_queued_game_is_cached_and_an_unstar_leaves_a_saved_set() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let carts = vec![
            test_cart(Platform::Gba, "Star"),
            test_cart(Platform::Md, "Played"),
            test_cart(Platform::Snes, "Other"),
        ];
        let mut library = Library::default();
        library.get_mut(Platform::Gba, "Star").favorite = true;
        library.get_mut(Platform::Md, "Played").last_played = 50;
        queue_set(root, Platform::Snes, "Other");
        queue_set(root, Platform::Snes, "Other");
        let queued = cache_queue(root);
        assert_eq!(queued.len(), 1);
        let got = cache_targets(&carts, &library, &queued);
        assert_eq!(
            got.iter().map(|c| c.stem.as_str()).collect::<Vec<_>>(),
            ["Star", "Played", "Other"]
        );

        // A set already on the card is not deleted when the game is unqueued.
        let sets = root.join("Config/Achievements/sets");
        std::fs::create_dir_all(&sets).unwrap();
        std::fs::write(sets.join("abc.json"), b"{}").unwrap();
        std::fs::write(
            sets.join("abc.list"),
            "game=Other\nid=1\nwhere=SNES\tOther\n",
        )
        .unwrap();
        unqueue_set(root, Platform::Snes, "Other");
        assert!(cache_queue(root).is_empty());
        assert!(sets.join("abc.json").is_file());
        assert!(sets.join("abc.list").is_file());
        let library = Library::default();
        let got = cache_targets(&carts, &library, &cache_queue(root));
        assert!(got.is_empty());
    }

    #[test]
    fn cache_wanted_caps_recent_plays_at_twenty() {
        let mut library = Library::default();
        for i in 0..30 {
            library
                .get_mut(Platform::Nes, &format!("N{i:02}"))
                .last_played = i as i64 + 1;
        }
        library.get_mut(Platform::Nes, "Fav").favorite = true;
        let wanted = cache_wanted(&library, &[cache_key(Platform::Gb, "Queued")]);
        assert!(wanted.iter().any(|k| k.stem == "Fav"));
        assert!(wanted.iter().any(|k| k.stem == "Queued"));
        let recent: Vec<_> = wanted
            .iter()
            .filter(|k| k.stem.starts_with('N'))
            .map(|k| k.stem.as_str())
            .collect();
        assert_eq!(recent.len(), 20);
        assert_eq!(recent[0], "N29");
        assert_eq!(recent[19], "N10");
    }

    #[test]
    fn cache_catalog_groups_systems_and_skips_those_without_retroachievements() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let carts = vec![
            test_cart(Platform::Snes, "Yoshi"),
            test_cart(Platform::Snes, "Mario"),
            test_cart(Platform::Gb, "Tetris"),
            test_cart(Platform::Gw, "Ball"),
            test_cart(Platform::A800, "Star Raiders"),
        ];
        let sets = root.join("Config/Achievements/sets");
        std::fs::create_dir_all(&sets).unwrap();
        std::fs::write(sets.join("abc.json"), b"{\"id\":1}").unwrap();
        std::fs::write(
            sets.join("abc.list"),
            "game=Mario\nid=1\nwhere=SNES\tMario\n",
        )
        .unwrap();
        let cat = cache_catalog_from(&carts, root);
        let dirs: Vec<_> = cat.systems.iter().map(|s| s.platform.dir_name()).collect();
        assert_eq!(dirs, ["GB", "SNES"]);
        let snes = cat.systems.iter().find(|s| s.platform == Platform::Snes).unwrap();
        assert_eq!(
            snes.games.iter().map(|g| g.key.stem.as_str()).collect::<Vec<_>>(),
            ["Mario", "Yoshi"]
        );
        assert!(snes.games[0].cached);
        assert!(snes.games[0].size.is_some());
        assert!(!snes.games[1].cached);
        assert!(cat.avg_size.is_some());
    }

    #[test]
    fn a_saved_picker_selection_round_trips_and_a_star_covers_new_games() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let carts = vec![
            test_cart(Platform::Snes, "Mario"),
            test_cart(Platform::Snes, "Yoshi"),
            test_cart(Platform::Gb, "Tetris"),
            test_cart(Platform::Gb, "Kirby"),
            test_cart(Platform::Nes, "Zelda"),
        ];
        let cat = cache_catalog_from(&carts, root);
        let mut selected = HashSet::new();
        for game in &cat.systems.iter().find(|s| s.platform == Platform::Snes).unwrap().games {
            selected.insert(game.key.clone());
        }
        selected.insert(cache_key(Platform::Gb, "Tetris"));
        save_pick(root, &cat.systems, &selected);
        let text = std::fs::read_to_string(root.join("Config/Achievements/cache-pick.txt")).unwrap();
        assert!(text.contains("SNES\t*"), "{text}");
        assert!(text.contains("GB\tTetris"), "{text}");
        assert!(!text.lines().any(|l| l.starts_with("NES\t")), "{text}");

        let carts2 = vec![
            test_cart(Platform::Snes, "Mario"),
            test_cart(Platform::Snes, "Yoshi"),
            test_cart(Platform::Snes, "Star Fox"),
            test_cart(Platform::Gb, "Tetris"),
        ];
        let cat2 = cache_catalog_from(&carts2, root);
        let loaded = load_pick(root, &cat2.systems);
        assert!(loaded.contains(&cache_key(Platform::Snes, "Star Fox")));
        assert!(loaded.contains(&cache_key(Platform::Snes, "Mario")));
        assert!(loaded.contains(&cache_key(Platform::Gb, "Tetris")));
        assert_eq!(loaded.len(), 4);
    }

    #[test]
    fn format_and_approx_bytes_show_known_and_estimated_sizes() {
        assert_eq!(format_bytes(800), "800 B");
        assert_eq!(format_bytes(2048), "2 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(approx_bytes(&[100, 300], 0, None), Some((400, false)));
        assert_eq!(approx_bytes(&[100, 300], 2, None), Some((400 + 200 * 2, true)));
        assert_eq!(approx_bytes(&[], 3, Some(50)), Some((150, true)));
        assert_eq!(approx_bytes(&[], 3, None), None);
        let carts = vec![test_cart(Platform::Gb, "A"), test_cart(Platform::Gb, "B")];
        let dir = tempfile::tempdir().unwrap();
        let cat = cache_catalog_from(&carts, dir.path());
        let mut selected = HashSet::new();
        selected.insert(cache_key(Platform::Gb, "A"));
        assert_eq!(selection_size(&cat.systems, &selected, None), None);
        assert_eq!(
            selection_size(&cat.systems, &selected, Some(80)),
            Some((80, true))
        );
    }

    #[test]
    fn carts_for_keys_keeps_order_and_drops_missing_games() {
        let carts = vec![
            test_cart(Platform::Snes, "Mario"),
            test_cart(Platform::Gb, "Tetris"),
        ];
        let keys = vec![
            cache_key(Platform::Gb, "Tetris"),
            cache_key(Platform::Nes, "Gone"),
            cache_key(Platform::Snes, "Mario"),
            cache_key(Platform::Gb, "Tetris"),
        ];
        let got = carts_for_keys(&carts, &keys);
        assert_eq!(
            got.iter().map(|c| c.stem.as_str()).collect::<Vec<_>>(),
            ["Tetris", "Mario"]
        );
    }

}
