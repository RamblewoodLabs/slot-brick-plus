//! HOME → Videos. Search a YouTube channel on the Brick, download what you pick
//! into Slot/Videos, and play it back offline.
//!
//! Picture: H.264 at 360p or below, in an MP4, decoded by ffmpeg to a 480×360
//! window at 12 frames a second. That is small enough for the H700 to decode in
//! software, and it fills Slot's 640×480 screen once the Brick scales it up to
//! 1024×768. Playback needs `Slot/System/ffmpeg` (or `ffmpeg` on the path).
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use slot_gfx::OUT_W;
use slot_input::{Action, Btn};
use slot_ui::{TexId, UndoFace};

use crate::sheet::Sheet;
use crate::youtube::{self, Channel, Video};

pub const FRAME_W: u32 = 480;
/// 16:9, so a 360p picture fills the window. Short enough to leave room for the title and the keys.
pub const FRAME_H: u32 = 270;
const FPS: u32 = 12;
const AUDIO_HZ: u32 = 22_050;
/// 0 is no cap. The rest are the Video space limit row, up to 16 GB.
const LIMITS_MB: [u32; 6] = [0, 1024, 2048, 4096, 8192, 16384];
/// HOME menu type, on the 640×480 screen the Brick scales to 1024×768.
const MENU_PX: f32 = 30.0;
const MENU_H: u32 = 40;
const MENU_PITCH: u32 = 48;
const MENU_LEGEND: u32 = 432;

const KEYS: [&str; 3] = [
    "1234567890qwertyuiopasdfghjkl-zxcvbnm_. ",
    "!@#$%^&*()QWERTYUIOPASDFGHJKL+ZXCVBNM?, ",
    "1234567890-_=+[]{}\\|;:'\"<>,./?`~!@#$%^& ",
];

#[derive(Clone, Debug)]
pub(crate) struct Saved {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) channel: String,
    pub(crate) bytes: u64,
    pub(crate) duration_ms: u64,
    pub(crate) position_ms: u64,
}

#[derive(Clone)]
struct Sub {
    id: String,
    name: String,
}

#[derive(Clone)]
struct Job {
    id: String,
    title: String,
    #[allow(dead_code)]
    channel: String,
    #[allow(dead_code)]
    duration_ms: u64,
}

enum Found {
    Channels(Vec<Channel>),
    Videos {
        name: String,
        videos: Vec<Video>,
        thumbs: Vec<Option<Vec<u8>>>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum View {
    Home,
    Search,
    Channels,
    Videos,
    Library,
    Queue,
    Player,
}

struct KeysState {
    query: String,
    key: usize,
    page: usize,
}

struct SharedFrame {
    rgba: Vec<u8>,
    seq: u64,
}

struct Player {
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
    frames: Arc<AtomicU64>,
    origin_ms: u64,
    video: Option<std::process::Child>,
    audio: Option<std::process::Child>,
    threads: Vec<JoinHandle<()>>,
    frame: Arc<Mutex<SharedFrame>>,
    pcm: Arc<Mutex<Vec<i16>>>,
}

pub struct VideoMenu {
    root: PathBuf,
    online: bool,
    view: View,
    back: View,
    status: String,
    row: usize,
    library: Vec<Saved>,
    /// Channels the player asked to follow. `channels` is a search result, not this list.
    subs: Vec<Sub>,
    retry: Option<Job>,
    channels: Vec<Channel>,
    videos: Vec<Video>,
    /// Small pictures for the channel's rows, one per video, already RGBA.
    row_thumbs: Vec<Option<Vec<u8>>>,
    channel_name: String,
    keys: Option<KeysState>,
    waiting: Vec<Job>,
    active: Option<Job>,
    lookup: Option<mpsc::Receiver<Result<Found, String>>>,
    progress: Arc<AtomicU64>,
    progress_total: Arc<AtomicU64>,
    player: Option<Player>,
    playing: Option<Saved>,
    volume: u8,
    limit_mb: u32,
    armed: Option<usize>,
    shown_sec: u64,
    shown_pct: u64,
    pub dirty: bool,
    pub face: Option<TexId>,
    pub frame_face: Option<TexId>,
}

impl VideoMenu {
    pub fn open(root: PathBuf, online: bool) -> Self {
        let _ = std::fs::create_dir_all(root.join("Videos"));
        let _ = std::fs::create_dir_all(root.join("Logs"));
        scrub_parts(&root);
        let mut menu = VideoMenu {
            root,
            online,
            view: View::Home,
            back: View::Home,
            status: "Search a channel, or play a video already on the card".into(),
            row: 0,
            library: Vec::new(),
            subs: Vec::new(),
            retry: None,
            channels: Vec::new(),
            videos: Vec::new(),
            row_thumbs: Vec::new(),
            channel_name: String::new(),
            keys: None,
            waiting: Vec::new(),
            active: None,
            lookup: None,
            progress: Arc::new(AtomicU64::new(0)),
            progress_total: Arc::new(AtomicU64::new(0)),
            player: None,
            playing: None,
            volume: 8,
            limit_mb: 0,
            armed: None,
            shown_sec: u64::MAX,
            shown_pct: u64::MAX,
            dirty: true,
            face: None,
            frame_face: None,
        };
        menu.library = read_index(&menu.root);
        menu.subs = read_subs(&menu.root);
        menu.limit_mb = read_limit(&menu.root);
        menu
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }

    /// Toasts the shelf should show. A finished download, or one that failed.
    pub fn poll(&mut self) -> Vec<(String, String)> {
        let mut notes = Vec::new();
        if let Some(rx) = &self.lookup {
            match rx.try_recv() {
                Ok(Ok(Found::Channels(channels))) => {
                    let n = channels.len();
                    self.channels = channels;
                    self.view = View::Channels;
                    self.row = 0;
                    self.keys = None;
                    self.lookup = None;
                    self.status = format!("{n} channels");
                    self.dirty = true;
                }
                Ok(Ok(Found::Videos {
                    name,
                    videos,
                    thumbs,
                })) => {
                    let n = videos.len();
                    self.videos = videos;
                    self.row_thumbs = thumbs;
                    self.channel_name = name;
                    self.view = View::Videos;
                    self.row = 0;
                    self.lookup = None;
                    self.status = format!("{n} videos");
                    self.dirty = true;
                }
                Ok(Err(message)) => {
                    self.lookup = None;
                    self.status = message;
                    self.dirty = true;
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.lookup = None;
                    self.status = "Search stopped".into();
                    self.dirty = true;
                }
            }
        }
        notes.extend(self.sync_queue());
        if self.active.is_some() {
            let pct = self.progress_total.load(Ordering::Relaxed).max(1);
            let shown = self.progress.load(Ordering::Relaxed).saturating_mul(100) / pct;
            if shown != self.shown_pct {
                self.shown_pct = shown;
                self.dirty = true;
            }
        }
        if let Some(player) = &self.player {
            let sec = player.position_ms() / 1000;
            if sec != self.shown_sec {
                self.shown_sec = sec;
                self.dirty = true;
            }
            if sec % 5 == 0 {
                if let Some(saved) = &self.playing {
                    let (id, pos) = (saved.id.clone(), player.position_ms());
                    if let Some(row) = self.library.iter_mut().find(|s| s.id == id) {
                        if row.position_ms / 5000 != pos / 5000 {
                            row.position_ms = pos;
                            let _ = write_index(&self.root, &self.library);
                        }
                    }
                }
            }
        }
        notes
    }

    pub fn stop_for_sleep(&mut self) {
        // Sleep suspends the machine. The downloader is a separate process and keeps its
        // partial file, so it carries on after the Brick wakes or restarts.
        self.stop_player(true);
        self.view = View::Home;
    }

    /// New picture, when ffmpeg has drawn another one.
    pub fn frame_if_new(&self, seen: u64) -> Option<(u64, Vec<u8>)> {
        let player = self.player.as_ref()?;
        let frame = player.frame.lock().unwrap_or_else(|e| e.into_inner());
        (frame.seq != seen && !frame.rgba.is_empty()).then(|| (frame.seq, frame.rgba.clone()))
    }

    /// Stereo samples at 22,050 Hz, for the shelf to mix in.
    pub fn take_pcm(&self) -> Vec<i16> {
        let Some(player) = &self.player else {
            return Vec::new();
        };
        if player.paused.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let mut pcm = player.pcm.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *pcm)
    }

    pub fn volume_gain(&self) -> f32 {
        if self.player.is_none() {
            return 0.0;
        }
        self.volume as f32 / 10.0
    }

    pub fn playing(&self) -> bool {
        self.player
            .as_ref()
            .is_some_and(|p| !p.paused.load(Ordering::Relaxed) && p.video.is_some())
    }

    /// True when this screen should close.
    pub fn input(&mut self, action: Action) -> bool {
        if !matches!(
            action,
            Action::GbaDown(_) | Action::ShelfLeft | Action::ShelfRight
        ) {
            return false;
        }
        self.dirty = true;
        if self.lookup.is_some() {
            if matches!(action, Action::GbaDown(Btn::B)) {
                self.lookup = None;
                self.status = "Search cancelled".into();
            }
            return false;
        }
        if self.view == View::Player {
            return self.player_input(action);
        }
        if self.view == View::Search {
            return self.search_input(action);
        }
        match action {
            Action::GbaDown(Btn::B) => return self.back(),
            Action::GbaDown(Btn::Up) => {
                self.row =
                    self.row.saturating_add(self.rows().saturating_sub(1)) % self.rows().max(1)
            }
            Action::GbaDown(Btn::Down) => self.row = (self.row + 1) % self.rows().max(1),
            Action::GbaDown(Btn::Left) | Action::ShelfLeft
                if self.view == View::Home && self.row == self.limit_row() =>
            {
                self.step_limit(false)
            }
            Action::GbaDown(Btn::Right) | Action::ShelfRight
                if self.view == View::Home && self.row == self.limit_row() =>
            {
                self.step_limit(true)
            }
            Action::GbaDown(Btn::Left)
            | Action::ShelfLeft
            | Action::GbaDown(Btn::Right)
            | Action::ShelfRight
                if self.view == View::Home && self.row == self.pause_row() =>
            {
                let on = !crate::video_queue::pause_in_games(&self.root);
                let _ = crate::video_queue::set_pause_in_games(&self.root, on);
                self.status = if on {
                    "Downloads pause while a game is open".into()
                } else {
                    "Downloads keep going during a game".into()
                };
                self.dirty = true;
            }
            Action::GbaDown(Btn::A) if self.view == View::Queue => {
                let paused = crate::video_queue::toggle_hold(&self.root);
                self.status = if paused {
                    "Download paused".into()
                } else {
                    "Download continuing".into()
                };
            }
            Action::GbaDown(Btn::A) => self.activate(),
            Action::GbaDown(Btn::Y) if self.view == View::Library => self.delete_saved(),
            Action::GbaDown(Btn::Y) if self.channel_row().is_some() => self.remove_channel(),
            Action::GbaDown(Btn::X) if self.view == View::Queue => self.cancel_download(),
            _ => {}
        }
        if self.row >= self.rows() {
            self.row = 0;
        }
        false
    }

    pub fn render(&self) -> UndoFace {
        let mut sheet = Sheet::new();
        let w = OUT_W.saturating_sub(48);
        if self.view != View::Player {
            sheet.text("VIDEOS", 16, 8, w, MENU_H, MENU_PX, false);
        }
        if !self.status.is_empty() && self.view != View::Player {
            sheet.text(&self.status, 16, 48, w, MENU_H, 24.0, true);
        }
        if self.view != View::Player
            && self.on_battery()
            && !crate::video_queue::jobs(&self.root).is_empty()
        {
            sheet.text("Plug in to keep downloading", 16, 80, w, 18, 16.0, true);
        }
        match self.view {
            View::Search => self.render_keys(&mut sheet),
            View::Player => self.render_player(&mut sheet),
            _ => self.render_rows(&mut sheet),
        }
        sheet.text(
            &self.legend(),
            16,
            MENU_LEGEND,
            OUT_W.saturating_sub(32),
            MENU_H,
            MENU_PX,
            false,
        );
        sheet.finish()
    }

    fn render_rows(&self, sheet: &mut Sheet) {
        let rows = self.row_labels();
        let visible = ((MENU_LEGEND - 96) / MENU_PITCH) as usize;
        let first = self.row.saturating_sub(visible.saturating_sub(1));
        for (line, row) in (first..rows.len().min(first + visible)).enumerate() {
            let y = 96 + line as u32 * MENU_PITCH;
            if row == self.row {
                sheet.rect(0, y, OUT_W, MENU_H, slot_ui::edge());
            }
            let mut x = 16;
            let mut text_w = OUT_W.saturating_sub(32);
            if self.view == View::Videos {
                if let Some(px) = self.row_thumbs.get(row).and_then(|t| t.as_ref()) {
                    sheet.image(12, y + 4, 64, 32, px);
                    x = 84;
                    text_w = OUT_W.saturating_sub(100);
                }
            }
            sheet.text(&rows[row], x, y, text_w, MENU_H, MENU_PX, false);
        }
    }

    fn render_keys(&self, sheet: &mut Sheet) {
        let Some(keys) = &self.keys else {
            return;
        };
        sheet.text_case(
            &keys.query,
            16,
            96,
            OUT_W.saturating_sub(32),
            MENU_H,
            MENU_PX,
            false,
            false,
        );
        let margin = 16u32;
        let gap = 6u32;
        let key_w = (OUT_W - margin * 2 - gap * 9) / 10;
        let page = KEYS[keys.page];
        for i in 0..40 {
            let x = margin + (i as u32 % 10) * (key_w + gap);
            let y = 148 + (i as u32 / 10) * 52;
            if y + MENU_H > MENU_LEGEND {
                break;
            }
            if i == keys.key {
                sheet.rect(x, y, key_w, MENU_H, slot_ui::edge());
            }
            let label = match page.chars().nth(i) {
                Some(' ') => "SPC".to_string(),
                Some(c) => c.to_string(),
                None => continue,
            };
            sheet.text_case(&label, x, y, key_w, MENU_H, 26.0, false, false);
        }
    }

    fn render_player(&self, sheet: &mut Sheet) {
        let (x, y) = frame_origin();
        sheet.rect(x, y, FRAME_W, FRAME_H, [0.05, 0.05, 0.06, 1.0]);
        let title = self
            .playing
            .as_ref()
            .map(|s| s.title.as_str())
            .unwrap_or("VIDEO");
        sheet.text(
            title,
            16,
            8,
            OUT_W.saturating_sub(32),
            MENU_H,
            MENU_PX,
            false,
        );
        if self.player.is_none() {
            let note = if self.status.is_empty() {
                "NO PICTURE"
            } else {
                self.status.as_str()
            };
            sheet.text(
                note,
                x + 8,
                y + FRAME_H / 2 - 20,
                FRAME_W.saturating_sub(16),
                MENU_H,
                MENU_PX,
                true,
            );
        }
        let pos = self
            .player
            .as_ref()
            .map(Player::position_ms)
            .or_else(|| self.playing.as_ref().map(|s| s.position_ms))
            .unwrap_or(0);
        let dur = self.playing.as_ref().map(|s| s.duration_ms).unwrap_or(0);
        let clock = match dur {
            0 => format!("{}    VOL {}", clock(pos), self.volume),
            _ => format!("{} / {}    VOL {}", clock(pos), clock(dur), self.volume),
        };
        let paused = self
            .player
            .as_ref()
            .is_none_or(|p| p.paused.load(Ordering::Relaxed));
        let state = if paused { "PAUSED" } else { "PLAYING" };
        sheet.text(
            &format!("{state}    {clock}"),
            16,
            y + FRAME_H + 8,
            OUT_W.saturating_sub(32),
            MENU_H,
            MENU_PX,
            false,
        );
    }

    fn legend(&self) -> String {
        match self.view {
            View::Search => "A TYPE    X DELETE    Y PAGE    START SEARCH    B BACK".into(),
            View::Player => "A PLAY/PAUSE    LEFT/RIGHT 10S    UP/DOWN VOLUME    B BACK".into(),
            View::Library => "A PLAY    Y DELETE    B BACK".into(),
            View::Queue => "X CANCEL    B BACK".into(),
            View::Home if self.row == self.limit_row() => "LEFT/RIGHT LIMIT    B BACK".into(),
            View::Home if self.row == self.pause_row() => "LEFT/RIGHT    B BACK".into(),
            View::Home if self.channel_row().is_some() => "A LATEST 5    Y REMOVE    B BACK".into(),
            _ => "A SELECT    B BACK".into(),
        }
    }

    fn row_labels(&self) -> Vec<String> {
        match self.view {
            View::Home => {
                let mut rows = vec!["Add a channel".into()];
                for sub in &self.subs {
                    let ask = if self.armed == Some(rows.len()) {
                        "REMOVE?  "
                    } else {
                        ""
                    };
                    rows.push(format!("{ask}{}", sub.name));
                }
                rows.push(format!("Saved videos ({})", self.library.len()));
                rows.push(format!(
                    "Downloads ({})",
                    self.waiting.len()
                        + usize::from(self.active.is_some())
                        + usize::from(self.retry.is_some())
                ));
                rows.push(self.space_label());
                rows.push(self.limit_label());
                rows.push(format!(
                    "Pause downloads in games          {}",
                    if crate::video_queue::pause_in_games(&self.root) {
                        "ON"
                    } else {
                        "OFF"
                    }
                ));
                rows
            }
            View::Channels => {
                if self.channels.is_empty() {
                    vec!["No channels".into()]
                } else {
                    self.channels
                        .iter()
                        .map(|c| {
                            if c.subscribers.is_empty() {
                                c.name.clone()
                            } else {
                                format!("{}  ·  {}", c.name, c.subscribers)
                            }
                        })
                        .collect()
                }
            }
            View::Videos => {
                if self.videos.is_empty() {
                    vec!["No videos".into()]
                } else {
                    self.videos
                        .iter()
                        .map(|v| {
                            let mark = if self.library.iter().any(|s| s.id == v.id) {
                                "SAVED  "
                            } else {
                                ""
                            };
                            match v.duration_ms {
                                0 => format!("{mark}{}  {}", v.title, v.published),
                                ms => format!("{mark}{}  {}  {}", v.title, clock(ms), v.published),
                            }
                        })
                        .collect()
                }
            }
            View::Library => {
                if self.library.is_empty() {
                    vec!["Nothing saved yet".into()]
                } else {
                    self.library
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let ask = if self.armed == Some(i) {
                                "DELETE?  "
                            } else {
                                ""
                            };
                            format!("{ask}{}  {}", s.title, mb(s.bytes))
                        })
                        .collect()
                }
            }
            View::Queue => {
                let mut rows = Vec::new();
                if let Some(job) = &self.active {
                    let got = self.progress.load(Ordering::Relaxed);
                    let total = self.progress_total.load(Ordering::Relaxed);
                    let pct = match total {
                        0 => format!("{} MB", got / 1_000_000),
                        n => format!("{}%", (got.saturating_mul(100) / n).min(100)),
                    };
                    rows.push(format!("DOWNLOADING  {pct}  {}", job.title));
                }
                if self.waiting.is_empty() && rows.is_empty() {
                    rows.push("Nothing queued".into());
                }
                for job in &self.waiting {
                    rows.push(format!("QUEUED  {}", job.title));
                }
                rows
            }
            View::Search | View::Player => Vec::new(),
        }
    }

    fn rows(&self) -> usize {
        self.row_labels().len().max(1)
    }

    /// One level up. A saved channel's videos return to the main list, and that list's B
    /// closes onto the shelf. The keyboard is only opened from "Add a channel".
    fn back(&mut self) -> bool {
        self.armed = None;
        match self.view {
            View::Home => true,
            View::Search => {
                self.keys = None;
                self.view = View::Home;
                self.row = 0;
                self.status = "Search a channel, or play a video already on the card".into();
                false
            }
            // Search results sit under the keyboard the player opened on purpose.
            View::Channels => {
                self.view = View::Search;
                if self.keys.is_none() {
                    self.keys = Some(KeysState {
                        query: String::new(),
                        key: 0,
                        page: 0,
                    });
                }
                false
            }
            View::Videos | View::Library | View::Queue => {
                self.keys = None;
                self.view = View::Home;
                self.row = 0;
                self.status = "Search a channel, or play a video already on the card".into();
                false
            }
            View::Player => {
                self.view = self.back;
                self.keys = None;
                false
            }
        }
    }

    fn activate(&mut self) {
        self.armed = None;
        match self.view {
            View::Home => self.activate_home(),
            View::Channels => {
                let Some(channel) = self.channels.get(self.row).cloned() else {
                    return;
                };
                self.subscribe(channel.id, channel.name);
            }
            View::Videos => {
                let Some(video) = self.videos.get(self.row).cloned() else {
                    return;
                };
                if self.library.iter().any(|s| s.id == video.id) {
                    self.back = View::Videos;
                    self.play(&video.id);
                    return;
                }
                self.enqueue(video);
            }
            View::Library => {
                let Some(saved) = self.library.get(self.row).cloned() else {
                    return;
                };
                self.back = View::Library;
                self.play(&saved.id);
            }
            _ => {}
        }
    }

    fn limit_row(&self) -> usize {
        self.subs.len() + 4
    }

    fn pause_row(&self) -> usize {
        self.subs.len() + 5
    }

    fn channel_row(&self) -> Option<usize> {
        if self.view == View::Home && (1..=self.subs.len()).contains(&self.row) {
            Some(self.row - 1)
        } else {
            None
        }
    }

    fn activate_home(&mut self) {
        if self.row == 0 {
            self.view = View::Search;
            self.keys = Some(KeysState {
                query: String::new(),
                key: 0,
                page: 0,
            });
            self.status = "Name, @handle, or channel link, then START".into();
            return;
        }
        if let Some(at) = self.channel_row() {
            let sub = self.subs[at].clone();
            self.open_sub(sub);
            return;
        }
        if self.row == self.subs.len() + 1 {
            self.view = View::Library;
            self.row = 0;
            self.library = read_index(&self.root);
            self.status = "Saved on the card. Plays with Wi-Fi off.".into();
        } else if self.row == self.subs.len() + 2 {
            self.view = View::Queue;
            self.row = 0;
            if let Some(job) = self.retry.take() {
                self.waiting.push(job);
                self.status = "Trying that download again".into();
                self.kick();
            } else {
                self.status = "The download queue".into();
            }
        }
    }

    fn open_sub(&mut self, sub: Sub) {
        if !self.online {
            self.status = "Connect to Wi-Fi first".into();
            return;
        }
        self.status = format!("Latest from {}…", sub.name);
        self.channel_name = sub.name.clone();
        let (id, name) = (sub.id, sub.name);
        self.spawn_lookup(move || {
            youtube::channel_videos(&id, &name).map(|videos| {
                let videos = youtube::latest_five(videos);
                let thumbs = videos.iter().map(|v| thumb_rgba(&v.id, 64, 32)).collect();
                Found::Videos {
                    name,
                    videos,
                    thumbs,
                }
            })
        });
    }

    fn subscribe(&mut self, id: String, name: String) {
        if !youtube::valid_channel_id(&id) {
            self.status = "That channel id cannot be saved".into();
            return;
        }
        if self.subs.iter().any(|s| s.id == id) {
            self.status = format!("Already following {name}");
        } else {
            self.subs.push(Sub {
                id,
                name: name.clone(),
            });
            let _ = write_subs(&self.root, &self.subs);
            self.status = format!("Following {name}");
        }
        self.view = View::Home;
        self.row = 0;
        self.keys = None;
    }

    fn remove_channel(&mut self) {
        let Some(at) = self.channel_row() else { return };
        if self.armed != Some(self.row) {
            self.armed = Some(self.row);
            self.status = "Press Y again to stop following".into();
            return;
        }
        self.armed = None;
        let gone = self.subs.remove(at);
        let _ = write_subs(&self.root, &self.subs);
        self.row = self.row.min(self.rows().saturating_sub(1));
        self.status = format!("Stopped following {}", gone.name);
    }

    fn enqueue(&mut self, video: Video) {
        if crate::video_queue::jobs(&self.root)
            .iter()
            .any(|job| job.id == video.id)
            || self
                .waiting
                .iter()
                .chain(self.active.iter())
                .any(|job| job.id == video.id)
        {
            self.status = "Already in the queue".into();
            return;
        }
        if !self.online {
            self.status = "Connect to Wi-Fi first".into();
            return;
        }
        self.make_room();
        let title = video.title.clone();
        if let Err(e) = crate::video_queue::enqueue(
            &self.root,
            &video.id,
            &video.title,
            &video.channel,
            video.duration_ms,
        ) {
            self.status = e;
            return;
        }
        // Stay on this channel's list. B from here returns to the main Videos page.
        self.view = View::Videos;
        self.status = if self.on_battery() {
            format!("Queued {title}. Plug in to keep downloading")
        } else {
            format!("Queued {title}")
        };
        let _ = self.sync_queue();
    }

    fn on_battery(&self) -> bool {
        let live = crate::menu_status::current();
        live.percent.is_some() && !live.charging
    }

    fn kick(&mut self) {
        let _ = self.sync_queue();
    }

    /// The queue and its progress live in files, and a separate process does the download.
    /// Launching a game does not stop that process.
    fn sync_queue(&mut self) -> Vec<(String, String)> {
        let mut notes = Vec::new();
        let jobs = crate::video_queue::jobs(&self.root);
        let progress = crate::video_queue::progress(&self.root);
        let before: Vec<String> = self
            .waiting
            .iter()
            .chain(self.active.iter())
            .map(|job| job.id.clone())
            .collect();
        for id in &before {
            if jobs.iter().any(|job| &job.id == id) {
                continue;
            }
            if video_path(&self.root, id).is_file() {
                if let Some(saved) = read_index(&self.root).into_iter().find(|row| &row.id == id) {
                    let title = saved.title.clone();
                    self.library.retain(|row| &row.id != id);
                    self.library.push(saved);
                    notes.push(("Saved".into(), title.clone()));
                    self.status = format!("Saved {title}");
                    self.dirty = true;
                }
            }
        }
        if let Some(row) = progress.as_ref() {
            self.progress.store(row.got, Ordering::Relaxed);
            self.progress_total.store(row.total, Ordering::Relaxed);
        }
        self.active = jobs.first().map(|job| Job {
            id: job.id.clone(),
            title: job.title.clone(),
            channel: job.channel.clone(),
            duration_ms: job.duration_ms,
        });
        self.waiting = jobs
            .into_iter()
            .skip(1)
            .map(|job| Job {
                id: job.id,
                title: job.title,
                channel: job.channel,
                duration_ms: job.duration_ms,
            })
            .collect();
        if let Some(err) = crate::video_queue::last_error(&self.root) {
            if self.status != err {
                self.status = err.clone();
                self.dirty = true;
                notes.push(("Download".into(), err));
            }
        }
        crate::video_queue::ensure(&self.root);
        if self.active.is_some() {
            let pct = self.progress_total.load(Ordering::Relaxed).max(1);
            let shown = self.progress.load(Ordering::Relaxed).saturating_mul(100) / pct;
            if shown != self.shown_pct {
                self.shown_pct = shown;
                self.dirty = true;
            }
        }
        notes
    }

    fn spawn_lookup(&mut self, work: impl FnOnce() -> Result<Found, String> + Send + 'static) {
        if self.lookup.is_some() {
            self.status = "Still searching".into();
            return;
        }
        let (tx, rx) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("slot-yt".into())
            .spawn(move || {
                let _ = tx.send(work());
            });
        if spawned.is_err() {
            self.status = "Could not start the search".into();
            return;
        }
        self.lookup = Some(rx);
    }

    fn search_input(&mut self, action: Action) -> bool {
        let Some(keys) = &mut self.keys else {
            self.view = View::Home;
            return false;
        };
        match action {
            Action::GbaDown(Btn::B) => {
                self.keys = None;
                self.view = View::Home;
                self.status = "Search a channel, or play a video already on the card".into();
            }
            Action::GbaDown(Btn::X) => {
                keys.query.pop();
            }
            Action::GbaDown(Btn::Y) => keys.page = (keys.page + 1) % 3,
            Action::GbaDown(Btn::Up) => keys.key = (keys.key + 30) % 40,
            Action::GbaDown(Btn::Down) => keys.key = (keys.key + 10) % 40,
            Action::GbaDown(Btn::Left) | Action::ShelfLeft => {
                keys.key = keys.key / 10 * 10 + (keys.key % 10 + 9) % 10
            }
            Action::GbaDown(Btn::Right) | Action::ShelfRight => {
                keys.key = keys.key / 10 * 10 + (keys.key % 10 + 1) % 10
            }
            Action::GbaDown(Btn::A) => {
                if keys.query.chars().count() < 64 {
                    if let Some(ch) = KEYS[keys.page].chars().nth(keys.key) {
                        keys.query.push(ch);
                    }
                }
            }
            Action::GbaDown(Btn::Start) => {
                let query = keys.query.trim().to_string();
                if query.is_empty() {
                    self.status = "Type a channel name".into();
                    return false;
                }
                if !self.online {
                    self.status = "Connect to Wi-Fi first".into();
                    return false;
                }
                match youtube::channel_ref(&query) {
                    Err(e) => self.status = e,
                    Ok(youtube::ChannelRef::Id(id)) => self.subscribe(id, query),
                    Ok(youtube::ChannelRef::Handle(_)) | Ok(youtube::ChannelRef::Name(_)) => {
                        self.status = format!("Looking up {query}…");
                        self.spawn_lookup(move || {
                            youtube::find_channels(&query).map(Found::Channels)
                        });
                    }
                }
            }
            _ => {}
        }
        false
    }

    fn player_input(&mut self, action: Action) -> bool {
        match action {
            Action::GbaDown(Btn::B) => {
                self.stop_player(true);
                self.view = self.back;
                self.status = "Saved on the card".into();
                false
            }
            Action::GbaDown(Btn::A) => {
                if self.player.as_ref().is_some_and(|p| p.ended()) {
                    if let Some(id) = self.playing.as_ref().map(|s| s.id.clone()) {
                        self.play(&id);
                    }
                } else if let Some(player) = &self.player {
                    let paused = !player.paused.load(Ordering::Relaxed);
                    player.paused.store(paused, Ordering::Relaxed);
                } else if let Some(id) = self.playing.as_ref().map(|s| s.id.clone()) {
                    self.play(&id);
                }
                false
            }
            Action::GbaDown(Btn::Left) => {
                self.seek(-10_000);
                false
            }
            Action::GbaDown(Btn::Right) => {
                self.seek(10_000);
                false
            }
            Action::GbaDown(Btn::L1) => {
                self.seek(-60_000);
                false
            }
            Action::GbaDown(Btn::R1) => {
                self.seek(60_000);
                false
            }
            Action::GbaDown(Btn::Up) => {
                self.volume = (self.volume + 1).min(10);
                false
            }
            Action::GbaDown(Btn::Down) => {
                self.volume = self.volume.saturating_sub(1);
                false
            }
            _ => false,
        }
    }

    fn seek(&mut self, delta_ms: i64) {
        let Some(saved) = self.playing.clone() else {
            return;
        };
        let now = self.player.as_ref().map(Player::position_ms).unwrap_or(0) as i64;
        let mut at = (now + delta_ms).max(0) as u64;
        if saved.duration_ms > 0 {
            at = at.min(saved.duration_ms.saturating_sub(500));
        }
        self.play_at(&saved.id, at);
    }

    /// Play a saved video from where it stopped. The shelf's A button uses this.
    pub fn play_id(&mut self, id: &str) -> bool {
        if !self.library.iter().any(|s| s.id == id) {
            self.library = read_index(&self.root);
        }
        if !self.library.iter().any(|s| s.id == id) {
            return false;
        }
        self.view = View::Player;
        self.back = View::Home;
        self.play(id);
        self.dirty = true;
        true
    }

    /// Read the cache again after the shelf deletes a video.
    pub fn reload_saved(&mut self) {
        self.library = read_index(&self.root);
        if self
            .playing
            .as_ref()
            .is_some_and(|s| !self.library.iter().any(|row| row.id == s.id))
        {
            self.stop_player(false);
            self.playing = None;
            self.view = View::Home;
        }
        self.dirty = true;
    }

    fn play(&mut self, id: &str) {
        let at = self
            .library
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.position_ms)
            .unwrap_or(0);
        self.play_at(id, at);
    }

    fn play_at(&mut self, id: &str, at_ms: u64) {
        self.stop_player(false);
        let Some(saved) = self.library.iter().find(|s| s.id == id).cloned() else {
            self.status = "That video is not in the cache".into();
            self.view = View::Player;
            return;
        };
        let file = video_path(&self.root, id);
        if !file.is_file() {
            self.status = "That video file is missing".into();
            self.view = View::Player;
            self.playing = Some(saved);
            return;
        }
        let Some(bin) = ffmpeg_path(&self.root) else {
            self.status = "Playback needs ffmpeg in Slot/System, or ffmpeg on the path".into();
            self.view = View::Player;
            self.playing = Some(saved);
            return;
        };
        match Player::start(&bin, &file, at_ms) {
            Ok(player) => {
                self.player = Some(player);
                self.playing = Some(saved);
                self.view = View::Player;
                self.shown_sec = u64::MAX;
                self.status = String::new();
            }
            Err(e) => {
                self.status = e;
                self.playing = Some(saved);
                self.view = View::Player;
            }
        }
    }

    fn stop_player(&mut self, save: bool) {
        let pos = self.player.as_ref().map(Player::position_ms);
        if let Some(mut player) = self.player.take() {
            player.stop();
        }
        if save {
            if let (Some(pos), Some(saved)) = (pos, &self.playing) {
                let id = saved.id.clone();
                if let Some(row) = self.library.iter_mut().find(|s| s.id == id) {
                    row.position_ms = pos;
                    let _ = write_index(&self.root, &self.library);
                }
            }
        }
    }

    fn delete_saved(&mut self) {
        if self.library.is_empty() {
            return;
        }
        let row = self.row.min(self.library.len() - 1);
        if self.armed != Some(row) {
            self.armed = Some(row);
            self.status = "Press Y again to delete it".into();
            return;
        }
        self.armed = None;
        let saved = self.library.remove(row);
        let _ = std::fs::remove_file(video_path(&self.root, &saved.id));
        let _ = std::fs::remove_file(self.root.join("Videos").join(format!("{}.m4a", saved.id)));
        let _ = write_index(&self.root, &self.library);
        self.row = row.min(self.library.len().saturating_sub(1));
        self.status = format!("Deleted {}", saved.title);
    }

    fn cancel_download(&mut self) {
        let jobs = crate::video_queue::jobs(&self.root);
        let id = jobs.get(self.row).map(|job| job.id.clone());
        if let Some(id) = id {
            crate::video_queue::remove(&self.root, &id);
            self.status = "Removed from the queue".into();
            let _ = self.sync_queue();
        }
    }

    fn step_limit(&mut self, up: bool) {
        let at = LIMITS_MB
            .iter()
            .position(|n| *n == self.limit_mb)
            .unwrap_or(0);
        let at = if up {
            (at + 1).min(LIMITS_MB.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        self.limit_mb = LIMITS_MB[at];
        let _ = write_limit(&self.root, self.limit_mb);
        if self.limit_mb > 0 && self.used() > self.limit_bytes() {
            self.make_room();
        }
        self.status = match self.limit_mb {
            0 => "Video space limit: No limit".into(),
            mb => format!("Video space limit: {}", gb_label(mb)),
        };
    }

    fn space_label(&self) -> String {
        let free = slot_store::free_space(&self.root.join("Videos"))
            .or_else(|| slot_store::free_space(&self.root));
        match free {
            Some(free) => format!(
                "Videos {} used · {} free",
                human_bytes(self.used()),
                human_bytes(free)
            ),
            None => format!("Videos {} used", human_bytes(self.used())),
        }
    }

    fn limit_label(&self) -> String {
        match self.limit_mb {
            0 => "Video space limit  No limit".into(),
            mb => format!("Video space limit  {}", gb_label(mb)),
        }
    }

    fn make_room(&mut self) {
        while self.used() > self.limit_bytes() && !self.library.is_empty() {
            let saved = self.library.remove(0);
            let _ = std::fs::remove_file(video_path(&self.root, &saved.id));
            self.status = format!("Removed {} to make room", saved.title);
        }
        let _ = write_index(&self.root, &self.library);
    }

    fn used(&self) -> u64 {
        self.library.iter().map(|s| s.bytes).sum()
    }

    fn limit_bytes(&self) -> u64 {
        match self.limit_mb {
            0 => u64::MAX,
            mb => u64::from(mb) * 1_000_000,
        }
    }
}

impl Drop for VideoMenu {
    fn drop(&mut self) {
        // The download lives in its own process. Closing this screen must not stop it.
        self.stop_player(true);
    }
}

impl Player {
    fn start(bin: &Path, file: &Path, at_ms: u64) -> Result<Player, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let ended = Arc::new(AtomicBool::new(false));
        let frames = Arc::new(AtomicU64::new(0));
        let frame = Arc::new(Mutex::new(SharedFrame {
            rgba: Vec::new(),
            seq: 0,
        }));
        let pcm = Arc::new(Mutex::new(Vec::new()));
        let mut video = spawn_video(bin, file, at_ms)?;
        let mut audio = spawn_audio(bin, file, at_ms).ok();
        let video_out = video.stdout.take().ok_or("ffmpeg gave no picture")?;
        let audio_out = audio.as_mut().and_then(|c| c.stdout.take());
        let mut threads = Vec::new();
        {
            let frame = frame.clone();
            let frames = frames.clone();
            let stop = stop.clone();
            let paused = paused.clone();
            let ended = ended.clone();
            threads.push(
                thread::Builder::new()
                    .name("slot-video-pic".into())
                    .spawn(move || {
                        picture_loop(video_out, frame, frames, stop, paused, ended);
                    })
                    .map_err(|e| e.to_string())?,
            );
        }
        if let Some(audio_out) = audio_out {
            let pcm = pcm.clone();
            let stop = stop.clone();
            let paused = paused.clone();
            if let Ok(handle) =
                thread::Builder::new()
                    .name("slot-video-snd".into())
                    .spawn(move || {
                        sound_loop(audio_out, pcm, stop, paused);
                    })
            {
                threads.push(handle);
            }
        }
        Ok(Player {
            stop,
            paused,
            ended,
            frames,
            origin_ms: at_ms,
            video: Some(video),
            audio,
            threads,
            frame,
            pcm,
        })
    }

    fn position_ms(&self) -> u64 {
        self.origin_ms + self.frames.load(Ordering::Relaxed).saturating_mul(1000) / u64::from(FPS)
    }

    fn ended(&self) -> bool {
        self.ended.load(Ordering::Relaxed)
    }

    fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        if let Some(mut child) = self.video.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(mut child) = self.audio.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.stop();
    }
}

fn picture_loop(
    mut out: impl std::io::Read,
    frame: Arc<Mutex<SharedFrame>>,
    frames: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    ended: Arc<AtomicBool>,
) {
    let n = (FRAME_W * FRAME_H * 4) as usize;
    let mut buf = vec![0u8; n];
    let mut start = Instant::now();
    let mut drawn = 0u64;
    while !stop.load(Ordering::Relaxed) {
        while paused.load(Ordering::Relaxed) && !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(30));
            start =
                Instant::now() - Duration::from_millis(drawn.saturating_mul(1000) / u64::from(FPS));
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if !read_full(&mut out, &mut buf) {
            ended.store(true, Ordering::Relaxed);
            break;
        }
        drawn += 1;
        frames.store(drawn, Ordering::Relaxed);
        if let Ok(mut slot) = frame.lock() {
            slot.rgba.clear();
            slot.rgba.extend_from_slice(&buf);
            slot.seq = slot.seq.wrapping_add(1);
        }
        let due = start + Duration::from_millis(drawn.saturating_mul(1000) / u64::from(FPS));
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            thread::sleep(wait.min(Duration::from_millis(200)));
        }
    }
}

fn sound_loop(
    mut out: impl std::io::Read,
    pcm: Arc<Mutex<Vec<i16>>>,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) {
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        while paused.load(Ordering::Relaxed) && !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(30));
        }
        let full = pcm
            .lock()
            .map(|p| p.len() > AUDIO_HZ as usize * 4)
            .unwrap_or(false);
        if full {
            thread::sleep(Duration::from_millis(20));
            continue;
        }
        let n = match out.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let n = n - n % 2;
        let mut samples = Vec::with_capacity(n / 2);
        for chunk in buf[..n].chunks_exact(2) {
            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }
        if let Ok(mut slot) = pcm.lock() {
            slot.extend(samples);
        }
    }
}

fn read_full(reader: &mut impl std::io::Read, buf: &mut [u8]) -> bool {
    let mut got = 0;
    while got < buf.len() {
        match reader.read(&mut buf[got..]) {
            Ok(0) => return false,
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return false,
        }
    }
    true
}

fn spawn_video(bin: &Path, file: &Path, at_ms: u64) -> Result<std::process::Child, String> {
    Command::new(bin)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-ss"])
        .arg(secs(at_ms))
        .arg("-i")
        .arg(file)
        .args(["-an", "-vf"])
        .arg(format!(
            "scale={FRAME_W}:{FRAME_H}:force_original_aspect_ratio=decrease,pad={FRAME_W}:{FRAME_H}:(ow-iw)/2:(oh-ih)/2:color=black"
        ))
        .args(["-pix_fmt", "rgba", "-r"])
        .arg(FPS.to_string())
        .args(["-f", "rawvideo", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Cannot start ffmpeg: {e}"))
}

fn spawn_audio(bin: &Path, file: &Path, at_ms: u64) -> Result<std::process::Child, String> {
    Command::new(bin)
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-ss"])
        .arg(secs(at_ms))
        .arg("-i")
        .arg(file)
        .args(["-vn", "-ac", "2", "-ar"])
        .arg(AUDIO_HZ.to_string())
        .args(["-f", "s16le", "pipe:1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())
}

fn secs(ms: u64) -> String {
    format!("{:.3}", ms as f64 / 1000.0)
}

pub fn frame_origin() -> (u32, u32) {
    ((OUT_W - FRAME_W) / 2, 52)
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

/// One line in `Logs/videos.log`, so a failure on the card can be read later.
const LOG_MAX: u64 = 512 * 1024;

pub fn log_line(root: &Path, line: &str) {
    if !log_worth_writing(line) {
        return;
    }
    let path = root.join("Logs/videos.log");
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) >= LOG_MAX {
        let _ = std::fs::rename(&path, root.join("Logs/videos.previous.log"));
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };
    let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    if !line.ends_with('\n') {
        let _ = std::io::Write::write_all(&mut file, b"\n");
    }
}

fn log_worth_writing(line: &str) -> bool {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    struct Seen {
        line: String,
        at: Instant,
    }
    static SEEN: Mutex<Option<Seen>> = Mutex::new(None);
    let Ok(mut slot) = SEEN.lock() else {
        return true;
    };
    let now = Instant::now();
    if let Some(seen) = slot.as_mut() {
        if seen.line == line && now.duration_since(seen.at) < Duration::from_secs(30) {
            return false;
        }
    }
    *slot = Some(Seen {
        line: line.to_string(),
        at: now,
    });
    true
}

pub(crate) fn scrub_parts(root: &Path) {
    let keep: Vec<String> = crate::video_queue::jobs(root)
        .into_iter()
        .map(|job| job.id)
        .collect();
    let Ok(dir) = std::fs::read_dir(root.join("Videos")) else {
        return;
    };
    for entry in dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let queued = keep.iter().any(|id| name.starts_with(id));
        if !queued && (name.ends_with(".part") || name.ends_with(".mux.mp4")) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

pub(crate) fn video_path(root: &Path, id: &str) -> PathBuf {
    root.join("Videos").join(format!("{id}.mp4"))
}

fn thumb_path(root: &Path, id: &str) -> PathBuf {
    root.join("Videos").join(format!("{id}.png"))
}

/// A video on the Videos shelf.
#[derive(Clone, Debug)]
pub struct Tape {
    pub id: String,
    pub title: String,
    pub position_ms: u64,
    pub thumb: Option<PathBuf>,
}

/// Saved videos that still have a file, oldest first as the cache lists them.
pub fn tapes(root: &Path) -> Vec<Tape> {
    let mut out: Vec<Tape> = read_index(root)
        .into_iter()
        .map(|saved| {
            let thumb = thumb_path(root, &saved.id);
            Tape {
                id: saved.id,
                title: saved.title,
                position_ms: saved.position_ms,
                thumb: thumb.is_file().then_some(thumb),
            }
        })
        .collect();
    let progress = crate::video_queue::progress(root);
    for job in crate::video_queue::jobs(root) {
        if out.iter().any(|tape| tape.id == job.id) {
            continue;
        }
        let title = match progress.as_ref().filter(|row| row.id == job.id) {
            Some(row) if row.total > 0 => {
                format!(
                    "{}  {}%",
                    job.title,
                    (row.got.saturating_mul(100) / row.total).min(99)
                )
            }
            Some(row) if row.got > 0 => format!("{}  downloading", job.title),
            _ => format!("{}  queued", job.title),
        };
        let thumb = thumb_path(root, &job.id);
        out.push(Tape {
            id: job.id,
            title,
            position_ms: 0,
            thumb: thumb.is_file().then_some(thumb),
        });
    }
    out
}

/// Remove a saved video, its picture, and its line in the cache.
pub fn forget(root: &Path, id: &str) -> bool {
    if !youtube::valid_id(id) {
        return false;
    }
    let mut library = read_index(root);
    let before = library.len();
    library.retain(|saved| saved.id != id);
    let _ = write_index(root, &library);
    let _ = std::fs::remove_file(video_path(root, id));
    let _ = std::fs::remove_file(thumb_path(root, id));
    let _ = std::fs::remove_file(video_path(root, id).with_extension("mp4.part"));
    before != library.len()
}

/// The picture YouTube serves for a video, saved as a PNG beside the file.
pub fn fetch_thumbnail(root: &Path, id: &str) -> bool {
    if !youtube::valid_id(id) {
        return false;
    }
    let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
    let Ok(jpeg) = crate::net::get(&url, 1 << 20) else {
        return false;
    };
    let Some(png) = jpeg_to_png(&jpeg) else {
        return false;
    };
    let _ = std::fs::create_dir_all(root.join("Videos"));
    std::fs::write(thumb_path(root, id), png).is_ok()
}

/// A JPEG as a PNG. Slot's shelf reads PNG, and the thumbnail arrives as JPEG.
pub fn jpeg_to_png(jpeg: &[u8]) -> Option<Vec<u8>> {
    let (rgba, w, h) = jpeg_pixels(jpeg)?;
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header().ok()?.write_image_data(&rgba).ok()?;
    Some(bytes)
}

fn thumb_rgba(id: &str, w: u32, h: u32) -> Option<Vec<u8>> {
    if !youtube::valid_id(id) {
        return None;
    }
    let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
    let jpeg = crate::net::get(&url, 1 << 20).ok()?;
    let (rgba, sw, sh) = jpeg_pixels(&jpeg)?;
    Some(scale_nearest(&rgba, sw, sh, w, h))
}

fn jpeg_pixels(jpeg: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg));
    let pixels = decoder.decode().ok()?;
    let info = decoder.info()?;
    let w = u32::from(info.width);
    let h = u32::from(info.height);
    let rgba = match info.pixel_format {
        jpeg_decoder::PixelFormat::RGB24 => {
            let mut out = Vec::with_capacity(pixels.len() / 3 * 4);
            for chunk in pixels.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        jpeg_decoder::PixelFormat::L8 => {
            let mut out = Vec::with_capacity(pixels.len() * 4);
            for gray in pixels {
                out.extend_from_slice(&[gray, gray, gray, 255]);
            }
            out
        }
        _ => return None,
    };
    (rgba.len() == (w * h * 4) as usize).then_some((rgba, w, h))
}

fn scale_nearest(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    if sw == 0 || sh == 0 || dw == 0 || dh == 0 {
        return out;
    }
    for y in 0..dh {
        for x in 0..dw {
            let sx = (u64::from(x) * u64::from(sw) / u64::from(dw)).min(u64::from(sw) - 1) as u32;
            let sy = (u64::from(y) * u64::from(sh) / u64::from(dh)).min(u64::from(sh) - 1) as u32;
            let s = ((sy * sw + sx) * 4) as usize;
            let d = ((y * dw + x) * 4) as usize;
            if s + 4 <= src.len() {
                out[d..d + 4].copy_from_slice(&src[s..s + 4]);
            }
        }
    }
    out
}

fn read_subs(root: &Path) -> Vec<Sub> {
    let Ok(text) = std::fs::read_to_string(root.join("Videos/channels.txt")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (id, name) = line.split_once('\t')?;
            youtube::valid_channel_id(id).then(|| Sub {
                id: id.to_string(),
                name: name.to_string(),
            })
        })
        .collect()
}

fn write_subs(root: &Path, subs: &[Sub]) -> std::io::Result<()> {
    let mut text = String::new();
    for sub in subs {
        text.push_str(&sub.id);
        text.push('\t');
        text.push_str(&flat(&sub.name));
        text.push('\n');
    }
    let _ = std::fs::create_dir_all(root.join("Videos"));
    slot_store::atomic_write(&root.join("Videos/channels.txt"), text.as_bytes())
}

fn index_path(root: &Path) -> PathBuf {
    root.join("Videos/index.txt")
}

pub(crate) fn read_index(root: &Path) -> Vec<Saved> {
    let Ok(text) = std::fs::read_to_string(index_path(root)) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(parse_saved)
        .filter(|s| youtube::valid_id(&s.id) && video_path(root, &s.id).is_file())
        .collect()
}

fn parse_saved(line: &str) -> Option<Saved> {
    let mut fields = line.split('\t');
    let id = fields.next()?.to_string();
    let title = fields.next()?.to_string();
    let channel = fields.next()?.to_string();
    let bytes = fields.next()?.parse().ok()?;
    let duration_ms = fields.next()?.parse().ok()?;
    let position_ms = fields.next()?.parse().ok()?;
    Some(Saved {
        id,
        title,
        channel,
        bytes,
        duration_ms,
        position_ms,
    })
}

pub(crate) fn write_index(root: &Path, library: &[Saved]) -> std::io::Result<()> {
    let mut text = String::new();
    for saved in library {
        if !youtube::valid_id(&saved.id) {
            continue;
        }
        text.push_str(&saved.id);
        text.push('\t');
        text.push_str(&flat(&saved.title));
        text.push('\t');
        text.push_str(&flat(&saved.channel));
        text.push('\t');
        text.push_str(&saved.bytes.to_string());
        text.push('\t');
        text.push_str(&saved.duration_ms.to_string());
        text.push('\t');
        text.push_str(&saved.position_ms.to_string());
        text.push('\n');
    }
    let _ = std::fs::create_dir_all(root.join("Videos"));
    slot_store::atomic_write(&index_path(root), text.as_bytes())
}

fn flat(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() || c == '\t' { ' ' } else { c })
        .collect()
}

fn read_limit(root: &Path) -> u32 {
    let Ok(text) = std::fs::read_to_string(root.join("Config/videos.txt")) else {
        return 0;
    };
    // 1536 was the old default, written before anyone chose a limit.
    text.lines()
        .find_map(|line| line.strip_prefix("limit_mb=")?.trim().parse().ok())
        .filter(|n| *n == 0 || LIMITS_MB.contains(n))
        .filter(|n| *n != 1536)
        .unwrap_or(0)
}

fn write_limit(root: &Path, limit_mb: u32) -> std::io::Result<()> {
    let _ = std::fs::create_dir_all(root.join("Config"));
    slot_store::atomic_write(
        &root.join("Config/videos.txt"),
        format!("limit_mb={limit_mb}\n").as_bytes(),
    )
}

fn clock(ms: u64) -> String {
    let secs = ms / 1000;
    let (h, m, s) = (secs / 3600, (secs / 60) % 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn mb(bytes: u64) -> String {
    human_bytes(bytes)
}

fn human_bytes(bytes: u64) -> String {
    const K: u64 = 1024;
    if bytes >= K * K * K {
        let gb = bytes as f64 / (K * K * K) as f64;
        if gb >= 10.0 {
            format!("{gb:.0} GB")
        } else {
            format!("{gb:.1} GB")
        }
    } else if bytes >= K * K {
        format!("{} MB", bytes / (K * K))
    } else {
        format!("{} KB", bytes / K.max(1))
    }
}

fn gb_label(mb: u32) -> String {
    if mb % 1024 == 0 {
        format!("{} GB", mb / 1024)
    } else {
        format!("{mb} MB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_input::{Action, Btn};

    #[test]
    fn the_keyboard_pages_have_forty_keys() {
        for page in KEYS {
            assert_eq!(page.chars().count(), 40, "{page}");
        }
    }

    #[test]
    fn saved_videos_round_trip_and_a_missing_file_is_dropped() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("Videos")).unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.mp4"), b"mp4").unwrap();
        let saved = vec![Saved {
            id: "abcdefghijk".into(),
            title: "A game\tname".into(),
            channel: "Nintendo".into(),
            bytes: 12,
            duration_ms: 90_000,
            position_ms: 1500,
        }];
        write_index(root.path(), &saved).unwrap();
        let again = read_index(root.path());
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].title, "A game name");
        assert_eq!(again[0].position_ms, 1500);
        std::fs::remove_file(root.path().join("Videos/abcdefghijk.mp4")).unwrap();
        assert!(read_index(root.path()).is_empty());
    }

    #[test]
    fn a_channel_row_with_a_short_thumbnail_still_draws() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = VideoMenu::open(root.path().to_path_buf(), true);
        menu.view = View::Videos;
        menu.videos = vec![youtube::Video {
            id: "abcdefghijk".into(),
            title: "A long title with é and 動画: the one that used to close the screen".into(),
            channel: "Retro Game Corps".into(),
            duration_ms: 60_000,
            published: "today".into(),
        }];
        menu.row_thumbs = vec![Some(vec![200u8; 56 * 32 * 4])];
        let face = menu.render();
        assert_eq!(face.w, slot_gfx::OUT_W);
        assert!(face.rgba.iter().any(|b| *b == 200));
    }

    #[test]
    fn a_partial_download_is_removed_and_a_finished_one_stays_on_the_shelf() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("Videos")).unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.mp4.part"), b"partial").unwrap();
        scrub_parts(root.path());
        assert!(!root.path().join("Videos/abcdefghijk.mp4.part").exists());
        std::fs::write(root.path().join("Videos/abcdefghijk.mp4"), b"....ftyp").unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.png"), b"png").unwrap();
        write_index(
            root.path(),
            &[Saved {
                id: "abcdefghijk".into(),
                title: "Kept".into(),
                channel: "Linus Tech Tips".into(),
                bytes: 8,
                duration_ms: 1,
                position_ms: 0,
            }],
        )
        .unwrap();
        let tapes = tapes(root.path());
        assert_eq!(tapes.len(), 1);
        assert!(tapes[0].thumb.is_some());
        log_line(root.path(), "saved abcdefghijk");
        let log = std::fs::read_to_string(root.path().join("Logs/videos.log")).unwrap();
        assert!(log.contains("saved abcdefghijk"));
    }

    #[test]
    fn the_shelf_lists_a_saved_video_and_forget_removes_it() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("Videos")).unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.mp4"), b"mp4").unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.png"), b"png").unwrap();
        write_index(
            root.path(),
            &[Saved {
                id: "abcdefghijk".into(),
                title: "Concert".into(),
                channel: "Band".into(),
                bytes: 3,
                duration_ms: 1000,
                position_ms: 400,
            }],
        )
        .unwrap();
        let listed = tapes(root.path());
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].title, "Concert");
        assert_eq!(listed[0].position_ms, 400);
        assert!(listed[0].thumb.is_some());
        assert!(forget(root.path(), "abcdefghijk"));
        assert!(tapes(root.path()).is_empty());
        assert!(!root.path().join("Videos/abcdefghijk.mp4").exists());
        assert!(!root.path().join("Videos/abcdefghijk.png").exists());
    }

    #[test]
    fn a_jpeg_thumbnail_becomes_a_png() {
        let dir = tempfile::tempdir().unwrap();
        let jpg = dir.path().join("t.jpg");
        let made = std::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=8x8",
                "-frames:v",
                "1",
            ])
            .arg(&jpg)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .ok()
            .is_some_and(|status| status.success());
        if !made {
            return;
        }
        let jpeg = std::fs::read(&jpg).unwrap();
        let png = jpeg_to_png(&jpeg).expect("jpeg decodes");
        assert!(png.starts_with(b"\x89PNG"));
        let (rgba, w, h) = jpeg_pixels(&jpeg).unwrap();
        assert_eq!((w, h), (8, 8));
        assert!(rgba[0] > 200 && rgba[1] < 40);
    }

    #[test]
    fn the_cache_limit_steps_and_is_remembered() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = VideoMenu::open(root.path().to_path_buf(), false);
        assert_eq!(menu.limit_mb, 0);
        assert!(menu
            .row_labels()
            .iter()
            .any(|row| row.contains("used") && row.contains("free")));
        for _ in 0..4 {
            menu.input(Action::GbaDown(Btn::Down));
        }
        menu.input(Action::ShelfRight);
        assert_eq!(menu.limit_mb, 1024);
        menu.input(Action::ShelfRight);
        assert_eq!(menu.limit_mb, 2048);
        let again = VideoMenu::open(root.path().to_path_buf(), false);
        assert_eq!(again.limit_mb, 2048);
        std::fs::write(root.path().join("Config/videos.txt"), "limit_mb=1536\n").unwrap();
        assert_eq!(read_limit(root.path()), 0);
    }

    #[test]
    fn y_deletes_a_saved_video_on_the_second_press() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("Videos")).unwrap();
        std::fs::write(root.path().join("Videos/abcdefghijk.mp4"), b"mp4").unwrap();
        write_index(
            root.path(),
            &[Saved {
                id: "abcdefghijk".into(),
                title: "A game".into(),
                channel: "Nintendo".into(),
                bytes: 3,
                duration_ms: 1000,
                position_ms: 0,
            }],
        )
        .unwrap();
        let mut menu = VideoMenu::open(root.path().to_path_buf(), false);
        menu.input(Action::GbaDown(Btn::Down));
        menu.input(Action::GbaDown(Btn::A));
        assert_eq!(menu.library.len(), 1);
        menu.input(Action::GbaDown(Btn::Y));
        assert_eq!(menu.library.len(), 1);
        menu.input(Action::GbaDown(Btn::Y));
        assert!(menu.library.is_empty());
        assert!(!root.path().join("Videos/abcdefghijk.mp4").exists());
    }

    #[test]
    fn b_from_a_channels_videos_steps_back_to_the_list_and_never_opens_the_keyboard() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = VideoMenu::open(root.path().to_path_buf(), true);
        menu.subs = vec![Sub {
            id: "UCabcdefghij".into(),
            name: "Retro Game Corps".into(),
        }];
        menu.view = View::Videos;
        menu.videos = vec![youtube::Video {
            id: "abcdefghijk".into(),
            title: "Latest episode".into(),
            channel: "Retro Game Corps".into(),
            duration_ms: 60_000,
            published: "today".into(),
        }];
        menu.input(Action::GbaDown(Btn::A));
        assert_eq!(menu.view, View::Videos);
        assert!(menu.keys.is_none());
        assert!(
            menu.status.starts_with("Queued "),
            "status was {}",
            menu.status
        );
        assert!(!menu.input(Action::GbaDown(Btn::B)));
        assert_eq!(menu.view, View::Home);
        assert!(menu.keys.is_none());
        assert!(menu.input(Action::GbaDown(Btn::B)));

        let mut menu = VideoMenu::open(root.path().to_path_buf(), true);
        menu.input(Action::GbaDown(Btn::A));
        assert_eq!(menu.view, View::Search);
        assert!(menu.keys.is_some());
        menu.input(Action::GbaDown(Btn::B));
        assert_eq!(menu.view, View::Home);
        assert!(menu.keys.is_none());
    }

    #[test]
    fn the_screens_draw_a_full_frame() {
        let root = tempfile::tempdir().unwrap();
        let mut menu = VideoMenu::open(root.path().to_path_buf(), true);
        let home = menu.render();
        assert_eq!(
            home.rgba.len(),
            (slot_gfx::OUT_W * slot_gfx::OUT_H * 4) as usize
        );
        menu.input(Action::GbaDown(Btn::A));
        let keys = menu.render();
        assert_eq!(keys.w, slot_gfx::OUT_W);
        menu.view = View::Player;
        menu.status = "Playback needs ffmpeg in Slot/System, or ffmpeg on the path".into();
        menu.playing = Some(Saved {
            id: "abcdefghijk".into(),
            title: "A game".into(),
            channel: "Nintendo".into(),
            bytes: 3,
            duration_ms: 90_000,
            position_ms: 12_000,
        });
        let mut player = menu.render();
        paint_frame(&mut player);
        assert!(player.rgba.iter().any(|b| *b != 0));
        if let Ok(dir) = std::env::var("SLOT_FEATURE_PREVIEW") {
            let dir = std::path::PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            write_png(&dir.join("videos-home.png"), &home);
            write_png(&dir.join("videos-keyboard.png"), &keys);
            write_png(&dir.join("videos-player.png"), &player);
        }
    }

    fn paint_frame(face: &mut UndoFace) {
        let (x0, y0) = frame_origin();
        for y in 0..FRAME_H {
            for x in 0..FRAME_W {
                let at = (((y0 + y) * face.w + x0 + x) * 4) as usize;
                if at + 3 >= face.rgba.len() {
                    continue;
                }
                face.rgba[at] = (40 + x / 4) as u8;
                face.rgba[at + 1] = (70 + y / 6) as u8;
                face.rgba[at + 2] = 120;
                face.rgba[at + 3] = 255;
            }
        }
    }

    fn write_png(path: &std::path::Path, face: &UndoFace) {
        let file = std::fs::File::create(path).unwrap();
        let mut encoder = png::Encoder::new(file, face.w, face.h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&face.rgba)
            .unwrap();
    }
}
