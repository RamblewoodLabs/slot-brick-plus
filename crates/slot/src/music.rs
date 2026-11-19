//! HOME → Music. Offline audio from YouTube channels and playlists.
//!
//! Files land in `Slot/Music/<channel>/<title>.m4a`, with the thumbnail beside them.
//! Downloads use the same chunked, resumable path as videos. Playback continues on the
//! system menu and stops when a game starts. B goes up one level; the keyboard opens
//! only from Add a channel or playlist.
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use slot_gfx::OUT_W;
use slot_input::{Action, Btn};
use slot_ui::{TexId, UndoFace};

use crate::sheet::Sheet;
use crate::youtube::{self, Video};

const MENU_PX: f32 = 30.0;
const MENU_H: u32 = 40;
const MENU_PITCH: u32 = 48;
const MENU_LEGEND: u32 = 430;
const AUDIO_HZ: u32 = 22_050;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum View {
    Home,
    Search,
    List,
    Player,
}

#[derive(Clone, Debug)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub channel: String,
    pub path: PathBuf,
    pub cover: Option<PathBuf>,
}

pub struct MusicMenu {
    root: PathBuf,
    online: bool,
    view: View,
    status: String,
    row: usize,
    query: String,
    keys: bool,
    key: usize,
    tracks: Vec<Track>,
    list: Vec<Video>,
    list_name: String,
    /// Where B returns from the player.
    from: View,
    order: Vec<usize>,
    at: usize,
    shuffle: bool,
    repeat: bool,
    paused: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    position_ms: Arc<Mutex<u64>>,
    duration_ms: u64,
    pcm: Arc<Mutex<Vec<i16>>>,
    cover_face: Option<(u32, u32, Vec<u8>)>,
    threads: Vec<JoinHandle<()>>,
    child: Option<std::process::Child>,
    pub dirty: bool,
    pub face: Option<TexId>,
}

impl MusicMenu {
    pub fn open(root: PathBuf, online: bool) -> Self {
        let _ = fs::create_dir_all(root.join("Music"));
        let mut menu = Self {
            root,
            online,
            view: View::Home,
            status: "Add a channel or playlist, or play something already saved".into(),
            row: 0,
            query: String::new(),
            keys: false,
            key: 0,
            tracks: Vec::new(),
            list: Vec::new(),
            list_name: String::new(),
            from: View::Home,
            order: Vec::new(),
            at: 0,
            shuffle: false,
            repeat: false,
            paused: Arc::new(AtomicBool::new(false)),
            stop: Arc::new(AtomicBool::new(false)),
            position_ms: Arc::new(Mutex::new(0)),
            duration_ms: 0,
            pcm: Arc::new(Mutex::new(Vec::new())),
            cover_face: None,
            threads: Vec::new(),
            child: None,
            dirty: true,
            face: None,
        };
        menu.tracks = library(&menu.root);
        menu
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    pub fn playing(&self) -> bool {
        self.child.is_some()
    }

    /// A game is starting: music stops. The download queue is left alone.
    pub fn stop_for_game(&mut self) {
        self.stop_player();
    }

    pub fn take_pcm(&self) -> Vec<i16> {
        if self.paused.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let mut pcm = self.pcm.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *pcm)
    }

    pub fn input(&mut self, action: Action) -> bool {
        self.dirty = true;
        if self.keys {
            return self.search_input(action);
        }
        match action {
            Action::GbaDown(Btn::B) => return self.back(),
            Action::GbaDown(Btn::Up) if self.view == View::Player => self.step(-1),
            Action::GbaDown(Btn::Down) if self.view == View::Player => self.step(1),
            Action::GbaDown(Btn::Up) => {
                self.row =
                    self.row.saturating_add(self.rows().saturating_sub(1)) % self.rows().max(1)
            }
            Action::GbaDown(Btn::Down) => self.row = (self.row + 1) % self.rows().max(1),
            Action::GbaDown(Btn::A) => self.activate(),
            Action::GbaDown(Btn::Left) if self.view == View::Player => self.seek(-10_000),
            Action::GbaDown(Btn::Right) if self.view == View::Player => self.seek(10_000),
            Action::GbaDown(Btn::Y) if self.view == View::Player => self.shuffle = !self.shuffle,
            Action::GbaDown(Btn::X) if self.view == View::Player => self.repeat = !self.repeat,
            _ => {}
        }
        false
    }

    pub fn render(&self) -> UndoFace {
        let mut sheet = Sheet::new();
        let w = OUT_W.saturating_sub(48);
        sheet.text("MUSIC", 16, 8, w, MENU_H, MENU_PX, false);
        if !self.status.is_empty() {
            sheet.text(&self.status, 16, 48, w, MENU_H, 22.0, true);
        }
        if self.view == View::Player {
            self.render_player(&mut sheet);
        } else if !self.keys {
            let rows = self.labels();
            let visible = ((MENU_LEGEND - 96) / MENU_PITCH) as usize;
            let first = self.row.saturating_sub(visible.saturating_sub(1));
            for (line, row) in (first..rows.len().min(first + visible)).enumerate() {
                let y = 96 + line as u32 * MENU_PITCH;
                if row == self.row {
                    sheet.rect(0, y, OUT_W, MENU_H, slot_ui::edge());
                }
                sheet.text(
                    &rows[row],
                    16,
                    y,
                    OUT_W.saturating_sub(32),
                    MENU_H,
                    MENU_PX,
                    false,
                );
            }
        }
        sheet.text(
            &self.legend(),
            16,
            MENU_LEGEND,
            OUT_W.saturating_sub(32),
            MENU_H,
            22.0,
            false,
        );
        sheet.finish()
    }

    fn render_player(&self, sheet: &mut Sheet) {
        let title = self.current().map(|t| t.title.clone()).unwrap_or_default();
        let channel = self
            .current()
            .map(|t| t.channel.clone())
            .unwrap_or_default();
        let mut top = 100u32;
        if let Some((w, h, rgba)) = &self.cover_face {
            sheet.image(16, 96, *w, *h, rgba);
            top = 96 + h + 8;
        }
        sheet.text(
            &title,
            16,
            top,
            OUT_W.saturating_sub(32),
            MENU_H,
            MENU_PX,
            false,
        );
        sheet.text(
            &channel,
            16,
            top + 40,
            OUT_W.saturating_sub(32),
            28,
            20.0,
            true,
        );
        let pos = *self.position_ms.lock().unwrap_or_else(|e| e.into_inner());
        let dur = self.duration_ms.max(1);
        let frac = (pos as f32 / dur as f32).clamp(0.0, 1.0);
        let bar = top + 80;
        sheet.rect(16, bar, OUT_W - 32, 8, [0.3, 0.3, 0.34, 1.0]);
        sheet.rect(
            16,
            bar,
            ((OUT_W - 32) as f32 * frac) as u32,
            8,
            slot_ui::edge(),
        );
        let state = if self.paused.load(Ordering::Relaxed) {
            "PAUSED"
        } else {
            "PLAYING"
        };
        let extra = format!(
            "{}    {}    {}",
            state,
            if self.shuffle { "SHUFFLE" } else { "IN ORDER" },
            if self.repeat { "REPEAT" } else { "ONCE" }
        );
        sheet.text(
            &extra,
            16,
            bar + 16,
            OUT_W.saturating_sub(32),
            28,
            18.0,
            true,
        );
    }

    fn legend(&self) -> String {
        match self.view {
            View::Player => "A PLAY/PAUSE    UP/DOWN TRACK    LEFT/RIGHT SEEK    Y SHUFFLE    X REPEAT    B BACK".into(),
            View::Search => "A TYPE    START SEARCH    B BACK".into(),
            _ => "A SELECT    B BACK".into(),
        }
    }

    fn labels(&self) -> Vec<String> {
        match self.view {
            View::Home => {
                let mut rows = vec!["Add a channel or playlist".into()];
                rows.extend(
                    self.tracks
                        .iter()
                        .map(|t| format!("{}  ·  {}", t.title, t.channel)),
                );
                if self.tracks.is_empty() {
                    rows.push("Nothing saved yet".into());
                }
                rows
            }
            View::List => {
                if self.list.is_empty() {
                    vec!["Nothing in this list".into()]
                } else {
                    self.list.iter().map(|v| v.title.clone()).collect()
                }
            }
            _ => Vec::new(),
        }
    }

    fn rows(&self) -> usize {
        self.labels().len().max(1)
    }

    fn back(&mut self) -> bool {
        match self.view {
            View::Home => true,
            View::Search => {
                self.keys = false;
                self.view = View::Home;
                self.status = "Add a channel or playlist, or play something already saved".into();
                false
            }
            View::List => {
                self.view = View::Home;
                self.row = 0;
                self.status = "Add a channel or playlist, or play something already saved".into();
                false
            }
            View::Player => {
                self.view = self.from;
                false
            }
        }
    }

    fn activate(&mut self) {
        match self.view {
            View::Home if self.row == 0 => {
                self.view = View::Search;
                self.keys = true;
                self.query.clear();
                self.status = "Name, @handle, or a playlist link, then START".into();
            }
            View::Home => {
                let index = self.row - 1;
                if index < self.tracks.len() {
                    self.from = View::Home;
                    self.start_order(index);
                }
            }
            View::List => {
                let Some(video) = self.list.get(self.row).cloned() else {
                    return;
                };
                if let Some(track) = self.tracks.iter().find(|t| t.id == video.id) {
                    let index = self
                        .tracks
                        .iter()
                        .position(|t| t.id == track.id)
                        .unwrap_or(0);
                    self.from = View::List;
                    self.start_order(index);
                    return;
                }
                if !self.online {
                    self.status = "Connect to Wi-Fi first".into();
                    return;
                }
                enqueue(&self.root, &video);
                self.status = format!("Queued {}", video.title);
                self.view = View::List;
            }
            View::Player => {
                let paused = !self.paused.load(Ordering::Relaxed);
                self.paused.store(paused, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    fn search_input(&mut self, action: Action) -> bool {
        match action {
            Action::GbaDown(Btn::B) => {
                self.keys = false;
                self.view = View::Home;
                false
            }
            Action::GbaDown(Btn::A) => {
                const KEYS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.-_@";
                if let Some(ch) = KEYS.chars().nth(self.key % KEYS.chars().count()) {
                    self.query.push(ch);
                }
                false
            }
            Action::GbaDown(Btn::X) => {
                self.query.pop();
                false
            }
            Action::GbaDown(Btn::Start) => {
                self.lookup();
                false
            }
            Action::GbaDown(Btn::Right) => {
                self.key = (self.key + 1) % 40;
                false
            }
            Action::GbaDown(Btn::Left) => {
                self.key = (self.key + 39) % 40;
                false
            }
            _ => false,
        }
    }

    fn lookup(&mut self) {
        let query = self.query.trim().to_string();
        if query.is_empty() {
            self.status = "Type a name or a playlist link".into();
            return;
        }
        if !self.online {
            self.status = "Connect to Wi-Fi first".into();
            return;
        }
        self.keys = false;
        if let Some(id) = youtube::playlist_id(&query) {
            match youtube::playlist_videos(&id, "Playlist") {
                Ok(videos) => {
                    self.list = videos.into_iter().take(30).collect();
                    self.list_name = "Playlist".into();
                    self.view = View::List;
                    self.row = 0;
                    self.status = format!("{} from {}", self.list.len(), self.list_name);
                }
                Err(e) => self.status = e,
            }
            return;
        }
        match youtube::find_channels(&query) {
            Ok(channels) if !channels.is_empty() => {
                let channel = &channels[0];
                match youtube::channel_videos(&channel.id, &channel.name) {
                    Ok(videos) => {
                        self.list = youtube::latest_five(videos);
                        self.list_name = channel.name.clone();
                        self.view = View::List;
                        self.row = 0;
                        self.status = format!("{} from {}", self.list.len(), self.list_name);
                    }
                    Err(e) => self.status = e,
                }
            }
            Ok(_) => self.status = "No channel matched".into(),
            Err(e) => self.status = e,
        }
    }

    pub fn play_library(&mut self, index: usize) {
        self.tracks = library(&self.root);
        if index < self.tracks.len() {
            self.from = View::Home;
            *self.position_ms.lock().unwrap_or_else(|e| e.into_inner()) = 0;
            self.start_order(index);
        }
    }

    fn start_order(&mut self, index: usize) {
        *self.position_ms.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        self.order = (0..self.tracks.len()).collect();
        if self.shuffle && self.order.len() > 1 {
            let n = self.order.len();
            self.order.rotate_left(index % n);
            self.at = 0;
        } else {
            self.at = self.order.iter().position(|i| *i == index).unwrap_or(0);
        }
        self.play_current();
    }

    fn play_current(&mut self) {
        self.stop_player();
        let Some(index) = self.order.get(self.at).copied() else {
            return;
        };
        let Some(track) = self.tracks.get(index) else {
            return;
        };
        self.cover_face = track.cover.as_deref().and_then(load_cover);
        self.view = View::Player;
        self.status = format!("{}  ·  {}", track.title, track.channel);
        self.duration_ms = self.duration_ms.max(180_000);
        let start = *self.position_ms.lock().unwrap_or_else(|e| e.into_inner());
        self.paused.store(false, Ordering::Relaxed);
        self.stop.store(false, Ordering::Relaxed);
        let Some(bin) = ffmpeg_path(&self.root) else {
            self.status = "Playback needs ffmpeg in Slot/System".into();
            return;
        };
        let file = track.path.clone();
        match spawn_audio(&bin, &file, start) {
            Ok(mut child) => {
                let out = child.stdout.take();
                self.child = Some(child);
                if let Some(out) = out {
                    let pcm = self.pcm.clone();
                    let stop = self.stop.clone();
                    let paused = self.paused.clone();
                    let pos = self.position_ms.clone();
                    if let Ok(handle) = thread::Builder::new()
                        .name("slot-music".into())
                        .spawn(move || audio_loop(out, pcm, stop, paused, pos))
                    {
                        self.threads.push(handle);
                    }
                }
            }
            Err(e) => self.status = e,
        }
    }

    fn step(&mut self, dir: i32) {
        if self.order.is_empty() {
            return;
        }
        if dir < 0 {
            self.at = if self.at == 0 {
                self.order.len() - 1
            } else {
                self.at - 1
            };
        } else {
            self.at = (self.at + 1) % self.order.len();
        }
        *self.position_ms.lock().unwrap_or_else(|e| e.into_inner()) = 0;
        self.play_current();
    }

    fn seek(&mut self, delta: i64) {
        let mut pos = self.position_ms.lock().unwrap_or_else(|e| e.into_inner());
        let next = (*pos as i64 + delta).max(0) as u64;
        *pos = next;
        drop(pos);
        self.play_current();
    }

    fn current(&self) -> Option<&Track> {
        self.order.get(self.at).and_then(|i| self.tracks.get(*i))
    }

    fn stop_player(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
        self.pcm.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub fn poll(&mut self) {
        self.tracks = library(&self.root);
        if self
            .child
            .as_mut()
            .is_some_and(|c| c.try_wait().ok().flatten().is_some())
        {
            self.child = None;
            if self.repeat {
                self.play_current();
            } else if self.at + 1 < self.order.len() {
                self.at += 1;
                self.play_current();
            }
        }
        crate::music_queue::ensure(&self.root);
    }
}

impl Drop for MusicMenu {
    fn drop(&mut self) {
        self.stop_player();
    }
}

pub fn library(root: &Path) -> Vec<Track> {
    let mut out = Vec::new();
    let Ok(channels) = fs::read_dir(root.join("Music")) else {
        return out;
    };
    for channel in channels.flatten() {
        if !channel.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let channel_name = channel.file_name().to_string_lossy().to_string();
        let Ok(files) = fs::read_dir(channel.path()) else {
            continue;
        };
        for file in files.flatten() {
            let name = file.file_name().to_string_lossy().to_string();
            if !name.ends_with(".m4a") {
                continue;
            }
            let title = name.trim_end_matches(".m4a").to_string();
            let cover = file.path().with_extension("png");
            out.push(Track {
                id: title.clone(),
                title,
                channel: channel_name.clone(),
                path: file.path(),
                cover: cover.is_file().then_some(cover),
            });
        }
    }
    out.sort_by(|a, b| a.title.cmp(&b.title));
    out
}

pub fn fat_name(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_control() || "\\/:*?\"<>|".contains(c) {
            out.push('_');
        } else {
            out.push(c);
        }
        if out.chars().count() >= 60 {
            break;
        }
    }
    let out = out.trim().trim_matches('.').to_string();
    if out.is_empty() {
        "track".into()
    } else {
        out
    }
}

fn enqueue(root: &Path, video: &Video) {
    let _ = crate::music_queue::push(root, &video.id, &video.title, &video.channel);
}

fn load_cover(path: &Path) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = fs::read(path).ok()?;
    let dec = png::Decoder::new(bytes.as_slice());
    let mut reader = dec.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let src_w = info.width;
    let src_h = info.height;
    if src_w == 0 || src_h == 0 {
        return None;
    }
    let dst_w = 200u32;
    let dst_h = (200 * src_h / src_w).clamp(80, 140);
    let mut out = vec![0u8; (dst_w * dst_h * 4) as usize];
    let stride = match info.color_type {
        png::ColorType::Rgba => 4,
        png::ColorType::Rgb => 3,
        _ => return None,
    };
    for y in 0..dst_h {
        for x in 0..dst_w {
            let sx = x * src_w / dst_w;
            let sy = y * src_h / dst_h;
            let i = ((sy * src_w + sx) * stride) as usize;
            let o = ((y * dst_w + x) * 4) as usize;
            if i + stride as usize > info.buffer_size() || o + 4 > out.len() {
                continue;
            }
            out[o] = buf[i];
            out[o + 1] = buf[i + 1];
            out[o + 2] = buf[i + 2];
            out[o + 3] = if stride == 4 { buf[i + 3] } else { 255 };
        }
    }
    Some((dst_w, dst_h, out))
}

fn ffmpeg_path(root: &Path) -> Option<PathBuf> {
    let own = root.join("System/ffmpeg");
    if own.is_file() {
        return Some(own);
    }
    Command::new("ffmpeg")
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .ok()
        .filter(|s| s.success())
        .map(|_| PathBuf::from("ffmpeg"))
}

fn spawn_audio(bin: &Path, file: &Path, start_ms: u64) -> Result<std::process::Child, String> {
    let at = format!("{:.3}", start_ms as f64 / 1000.0);
    Command::new(bin)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-ss",
            &at,
            "-i",
        ])
        .arg(file)
        .args([
            "-vn",
            "-ac",
            "2",
            "-ar",
            &AUDIO_HZ.to_string(),
            "-f",
            "s16le",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Cannot start ffmpeg: {e}"))
}

fn audio_loop(
    mut out: impl Read,
    pcm: Arc<Mutex<Vec<i16>>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    pos: Arc<Mutex<u64>>,
) {
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        if paused.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(30));
            continue;
        }
        let n = match out.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        let mut samples = Vec::with_capacity(n / 2);
        for pair in buf[..n].chunks_exact(2) {
            samples.push(i16::from_le_bytes([pair[0], pair[1]]));
        }
        if let Ok(mut at) = pos.lock() {
            *at += (samples.len() as u64 / 2) * 1000 / AUDIO_HZ as u64;
        }
        if let Ok(mut dest) = pcm.lock() {
            dest.extend(samples);
        }
    }
}

pub fn tracks_for_shelf(root: &Path) -> Vec<crate::videos::Tape> {
    library(root)
        .into_iter()
        .map(|track| crate::videos::Tape {
            id: track.id,
            title: track.title,
            position_ms: 0,
            thumb: track.cover,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fat_name_keeps_letters_and_drops_illegal_marks() {
        assert_eq!(fat_name("A:B*C?"), "A_B_C_");
        assert_eq!(fat_name("  ...  "), "track");
        assert!(fat_name(&"気".repeat(80)).chars().count() <= 60);
    }

    #[test]
    fn b_from_a_list_returns_home_and_the_keyboard_is_only_from_add() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = MusicMenu::open(root.path().to_path_buf(), false);
        menu.input(Action::GbaDown(Btn::A));
        assert!(menu.keys);
        assert_eq!(menu.view, View::Search);
        assert!(!menu.input(Action::GbaDown(Btn::B)));
        assert_eq!(menu.view, View::Home);
        assert!(!menu.keys);
        menu.view = View::List;
        menu.list = vec![Video {
            id: "abcdefghijk".into(),
            title: "Song".into(),
            channel: "Band".into(),
            duration_ms: 0,
            published: String::new(),
        }];
        assert!(!menu.input(Action::GbaDown(Btn::B)));
        assert_eq!(menu.view, View::Home);
        assert!(menu.input(Action::GbaDown(Btn::B)));
    }

    #[test]
    fn queueing_a_track_stays_on_the_list() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = MusicMenu::open(root.path().to_path_buf(), true);
        menu.view = View::List;
        menu.list = vec![Video {
            id: "abcdefghijk".into(),
            title: "Song".into(),
            channel: "Band".into(),
            duration_ms: 1,
            published: String::new(),
        }];
        menu.input(Action::GbaDown(Btn::A));
        assert_eq!(menu.view, View::List);
        assert!(menu.status.starts_with("Queued "));
    }
}
