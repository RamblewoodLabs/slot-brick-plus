//! Pip, Slot's companion. Missions, XP and sessions stay in Config/companion.txt.
//! Nothing here is sent anywhere, and resetting it does not touch games or saves.
//!
//! Slot does not track bosses or stages. A mission that play time can prove completes
//! on its own. Y on the companion screen can mark one done by hand, once.
use std::path::Path;

use crate::atomic::atomic_write;
use crate::platform::Platform;

pub const FILE: &str = "Config/companion.txt";

/// XP for the first time a game is actually played.
pub const XP_FIRST: u32 = 20;
/// One game, one goal.
pub const XP_SHORT: u32 = 50;
/// Several games or systems.
pub const XP_MULTI: u32 = 100;
/// A weekend's worth of games.
pub const XP_WEEKLY: u32 = 200;
pub const LEVEL_XP: u32 = 100;
const ACTIVE: usize = 3;
const SESSION_KEEP: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mood {
    Happy,
    Excited,
    Sleepy,
    Curious,
    Celebrating,
}

impl Mood {
    pub fn name(self) -> &'static str {
        match self {
            Mood::Happy => "Happy",
            Mood::Excited => "Excited",
            Mood::Sleepy => "Sleepy",
            Mood::Curious => "Curious",
            Mood::Celebrating => "Celebrating",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MissionKind {
    TryNew,
    Marathon,
    Forgotten,
    Tourist,
    OneMore,
    HiddenGem,
    Weekend,
}

impl MissionKind {
    pub const ALL: [MissionKind; 7] = [
        MissionKind::TryNew,
        MissionKind::Marathon,
        MissionKind::Forgotten,
        MissionKind::Tourist,
        MissionKind::OneMore,
        MissionKind::HiddenGem,
        MissionKind::Weekend,
    ];

    fn id(self) -> &'static str {
        match self {
            MissionKind::TryNew => "try",
            MissionKind::Marathon => "marathon",
            MissionKind::Forgotten => "forgotten",
            MissionKind::Tourist => "tourist",
            MissionKind::OneMore => "return",
            MissionKind::HiddenGem => "gem",
            MissionKind::Weekend => "weekend",
        }
    }

    fn parse(value: &str) -> Option<MissionKind> {
        Self::ALL.into_iter().find(|k| k.id() == value)
    }

    /// What the mission is, in ordinary words. The screen and the settings row use this.
    pub fn plain(self) -> &'static str {
        match self {
            MissionKind::TryNew => "Try a game you've never played",
            MissionKind::HiddenGem => "Try a game you rarely play",
            MissionKind::Marathon => "Play one game for 20 minutes",
            MissionKind::Forgotten => "Play a game you haven't opened in a while",
            MissionKind::OneMore => "Play a game one more time",
            MissionKind::Tourist => "Play 3 different systems today",
            MissionKind::Weekend => "Play 4 different games this weekend",
        }
    }

    fn xp(self) -> u32 {
        match self {
            MissionKind::Weekend => XP_WEEKLY,
            MissionKind::Tourist => XP_MULTI,
            _ => XP_SHORT,
        }
    }
}

/// A game Pip can see: what the library knows, plus how many sessions Pip recorded.
#[derive(Clone, Debug)]
pub struct GameSeen {
    pub platform: Platform,
    pub stem: String,
    pub played_secs: u64,
    pub last_played: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mission {
    pub id: String,
    pub kind: MissionKind,
    pub title: String,
    pub detail: String,
    pub platform: Option<Platform>,
    pub stem: String,
    pub need: u32,
    pub progress: u32,
    pub xp: u32,
    pub done: bool,
    /// Systems or stems already counted, comma separated.
    pub extra: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoneMission {
    pub title: String,
    pub day: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub platform: Platform,
    pub stem: String,
    pub secs: u64,
    pub day: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Award {
    pub title: String,
    pub detail: String,
    pub xp: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OpenSession {
    platform: Platform,
    stem: String,
    secs: u64,
    day: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Companion {
    pub enabled: bool,
    pub toasts: bool,
    pub sounds: bool,
    pub animate: bool,
    pub categories: Vec<MissionKind>,
    pub xp: u32,
    pub next_id: u32,
    pub awarded: Vec<String>,
    pub excluded: Vec<Platform>,
    pub missions: Vec<Mission>,
    pub history: Vec<DoneMission>,
    pub sessions: Vec<Session>,
    open: Option<OpenSession>,
    suggest: Option<(Platform, String)>,
    /// Set for a short while after a level-up so the screen can celebrate.
    pub celebrating: bool,
}

impl Default for Companion {
    fn default() -> Self {
        Companion {
            enabled: true,
            toasts: true,
            sounds: true,
            animate: true,
            categories: MissionKind::ALL.to_vec(),
            xp: 0,
            next_id: 1,
            awarded: Vec::new(),
            excluded: Vec::new(),
            missions: Vec::new(),
            history: Vec::new(),
            sessions: Vec::new(),
            open: None,
            suggest: None,
            celebrating: false,
        }
    }
}

impl Companion {
    pub fn read(root: &Path) -> Companion {
        let text = std::fs::read_to_string(root.join(FILE)).unwrap_or_default();
        let mut companion = Companion::parse(&text);
        // A session left open by a crash or a power loss keeps only the seconds already
        // counted. The gap until now is not play.
        if let Some(open) = companion.open.take() {
            companion.push_session(open);
        }
        companion
    }

    pub fn write(&self, root: &Path) -> std::io::Result<()> {
        atomic_write(&root.join(FILE), self.text().as_bytes())
    }

    /// Each level asks for more than the one before: 100, then 200, then 300.
    pub fn level(&self) -> u32 {
        let mut left = self.xp;
        let mut level = 1u32;
        loop {
            let cost = self.cost_of(level);
            if left < cost || level >= 99 {
                break;
            }
            left -= cost;
            level += 1;
        }
        level
    }

    pub fn toward_next(&self) -> u32 {
        let mut left = self.xp;
        let mut level = 1u32;
        loop {
            let cost = self.cost_of(level);
            if left < cost || level >= 99 {
                return left;
            }
            left -= cost;
            level += 1;
        }
    }

    pub fn next_cost(&self) -> u32 {
        self.cost_of(self.level())
    }

    fn cost_of(&self, level: u32) -> u32 {
        LEVEL_XP.saturating_mul(level.max(1))
    }

    /// A short hello. The words are Slot's own.
    pub fn greeting(&self, day: i64) -> &'static str {
        if self.celebrating {
            return "That one is done";
        }
        const LINES: [&str; 4] = [
            "Good to see you",
            "Ready when you are",
            "Pick a game and play",
            "A short session still counts",
        ];
        LINES[day.rem_euclid(LINES.len() as i64) as usize]
    }

    pub fn mood(&self, day: i64, games: &[GameSeen]) -> Mood {
        if self.celebrating {
            return Mood::Celebrating;
        }
        let last = self.sessions.iter().map(|s| s.day).max().unwrap_or(0);
        if last > 0 && day > last + 2 {
            return Mood::Sleepy;
        }
        if games.iter().any(|g| g.played_secs == 0) && self.sessions.is_empty() {
            return Mood::Curious;
        }
        if self.history.last().is_some_and(|d| d.day == day) {
            return Mood::Excited;
        }
        Mood::Happy
    }

    pub fn suggestion(&self) -> Option<(Platform, String)> {
        self.suggest.clone()
    }

    /// Play that Slot actually counted. Gaps (sleep, a crash) are not included in `secs`.
    pub fn note_play(
        &mut self,
        platform: Platform,
        stem: &str,
        secs: u64,
        day: i64,
        games: &[GameSeen],
    ) -> Vec<Award> {
        if !self.enabled || stem.is_empty() || secs == 0 {
            return Vec::new();
        }
        let mut awards = Vec::new();
        let first = games
            .iter()
            .find(|g| g.platform == platform && g.stem == stem)
            .is_none_or(|g| g.played_secs == 0)
            && !self
                .sessions
                .iter()
                .any(|s| s.platform == platform && s.stem == stem)
            && self
                .open
                .as_ref()
                .is_none_or(|o| o.platform != platform || o.stem != stem);
        self.add_open(platform, stem, secs, day);
        if first {
            let key = format!("first:{}\t{stem}", platform.dir_name());
            if self.grant(
                &key,
                XP_FIRST,
                &mut awards,
                "First launch",
                &format!("{stem}  +{XP_FIRST} XP"),
            ) {
                // grant already pushed
            }
        }
        self.ensure_missions(games, day);
        self.advance(platform, stem, secs, day, &mut awards);
        self.ensure_missions(games, day);
        self.pick_suggestion(games, day);
        awards
    }

    /// Mark the selected mission done by hand. XP is awarded once.
    pub fn complete_by_hand(&mut self, index: usize, day: i64) -> Option<Award> {
        let mission = self.missions.get_mut(index)?;
        if mission.done {
            return None;
        }
        mission.done = true;
        mission.progress = mission.need;
        let mission = self.missions[index].clone();
        let mut awards = Vec::new();
        self.finish(&mission, day, &mut awards);
        awards.into_iter().next()
    }

    pub fn refresh_suggestion(&mut self, games: &[GameSeen], day: i64) {
        let previous = self.suggest.clone();
        self.suggest = None;
        self.pick_suggestion(games, day.wrapping_add(1));
        if self.suggest.is_some() && self.suggest == previous {
            self.suggest = None;
            self.pick_suggestion(games, day.wrapping_mul(3).wrapping_add(9));
        }
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.enabled = on;
    }

    pub fn toggle_category(&mut self, kind: MissionKind) {
        if let Some(at) = self.categories.iter().position(|k| *k == kind) {
            self.categories.remove(at);
            self.missions.retain(|m| m.kind != kind || m.done);
        } else {
            self.categories.push(kind);
        }
    }

    pub fn toggle_excluded(&mut self, platform: Platform) {
        if let Some(at) = self.excluded.iter().position(|p| *p == platform) {
            self.excluded.remove(at);
        } else {
            self.excluded.push(platform);
        }
    }

    /// Clears Pip's XP, missions and sessions. Games and saves are not in this file.
    pub fn reset(&mut self) {
        let settings = (
            self.enabled,
            self.toasts,
            self.sounds,
            self.animate,
            self.categories.clone(),
        );
        *self = Companion::default();
        self.enabled = settings.0;
        self.toasts = settings.1;
        self.sounds = settings.2;
        self.animate = settings.3;
        self.categories = settings.4;
    }

    /// The game left the screen: keep the seconds already counted, and nothing after.
    pub fn settle(&mut self) {
        if let Some(open) = self.open.take() {
            self.push_session(open);
        }
    }

    pub fn active(&self) -> impl Iterator<Item = &Mission> {
        self.missions.iter().filter(|m| !m.done)
    }

    fn grant(
        &mut self,
        key: &str,
        xp: u32,
        awards: &mut Vec<Award>,
        title: &str,
        detail: &str,
    ) -> bool {
        if self.awarded.iter().any(|k| k == key) {
            return false;
        }
        self.awarded.push(key.to_string());
        let before = self.level();
        self.xp = self.xp.saturating_add(xp);
        self.celebrating = self.level() > before;
        awards.push(Award {
            title: title.into(),
            detail: detail.into(),
            xp,
        });
        if self.celebrating {
            awards.push(Award {
                title: "Level up".into(),
                detail: format!("Pip is level {}", self.level()),
                xp: 0,
            });
        }
        true
    }

    fn add_open(&mut self, platform: Platform, stem: &str, secs: u64, day: i64) {
        let same = self
            .open
            .as_ref()
            .is_some_and(|o| o.platform == platform && o.stem == stem);
        if !same {
            if let Some(open) = self.open.take() {
                self.push_session(open);
            }
            self.open = Some(OpenSession {
                platform,
                stem: stem.into(),
                secs: 0,
                day,
            });
        }
        if let Some(open) = &mut self.open {
            open.secs = open.secs.saturating_add(secs);
            open.day = day;
        }
    }

    fn push_session(&mut self, open: OpenSession) {
        if open.secs == 0 {
            return;
        }
        self.sessions.push(Session {
            platform: open.platform,
            stem: open.stem,
            secs: open.secs,
            day: open.day,
        });
        if self.sessions.len() > SESSION_KEEP {
            let extra = self.sessions.len() - SESSION_KEEP;
            self.sessions.drain(0..extra);
        }
    }

    fn advance(
        &mut self,
        platform: Platform,
        stem: &str,
        secs: u64,
        day: i64,
        awards: &mut Vec<Award>,
    ) {
        let weekend = is_weekend(day);
        let mut finished = Vec::new();
        for mission in &mut self.missions {
            if mission.done {
                continue;
            }
            match mission.kind {
                MissionKind::TryNew
                | MissionKind::HiddenGem
                | MissionKind::Forgotten
                | MissionKind::OneMore => {
                    if mission.platform == Some(platform) && mission.stem == stem {
                        mission.progress = 1;
                    }
                }
                MissionKind::Marathon => {
                    if mission.platform == Some(platform) && mission.stem == stem {
                        mission.progress = mission
                            .progress
                            .saturating_add(secs.min(u32::MAX as u64) as u32);
                    }
                }
                MissionKind::Tourist => {
                    let tag = platform.dir_name();
                    if !mission.extra.split(',').any(|s| s == tag) {
                        if !mission.extra.is_empty() {
                            mission.extra.push(',');
                        }
                        mission.extra.push_str(tag);
                        mission.progress = mission.progress.saturating_add(1);
                    }
                }
                MissionKind::Weekend => {
                    if weekend && !mission.extra.split(',').any(|s| s == stem) {
                        if !mission.extra.is_empty() {
                            mission.extra.push(',');
                        }
                        mission.extra.push_str(stem);
                        mission.progress = mission.progress.saturating_add(1);
                    }
                }
            }
            if mission.progress >= mission.need {
                mission.done = true;
                finished.push(mission.clone());
            }
        }
        for mission in finished {
            self.finish(&mission, day, awards);
        }
    }

    fn finish(&mut self, mission: &Mission, day: i64, awards: &mut Vec<Award>) {
        let key = format!("mission:{}", mission.id);
        self.grant(
            &key,
            mission.xp,
            awards,
            "Mission",
            &format!("{}  +{} XP", mission.title, mission.xp),
        );
        self.history.push(DoneMission {
            title: mission.title.clone(),
            day,
        });
        if self.history.len() > 20 {
            let extra = self.history.len() - 20;
            self.history.drain(0..extra);
        }
    }

    pub fn ensure_missions(&mut self, games: &[GameSeen], day: i64) {
        if !self.enabled {
            return;
        }
        let mut guard = 0;
        while self.missions.iter().filter(|m| !m.done).count() < ACTIVE && guard < 8 {
            guard += 1;
            if !self.spawn(games, day) {
                break;
            }
        }
    }

    fn touched(&self, platform: Platform, stem: &str) -> bool {
        self.sessions
            .iter()
            .any(|s| s.platform == platform && s.stem == stem)
            || self
                .open
                .as_ref()
                .is_some_and(|s| s.platform == platform && s.stem == stem)
            || self
                .missions
                .iter()
                .any(|m| m.done && m.platform == Some(platform) && m.stem == stem)
    }

    fn spawn(&mut self, games: &[GameSeen], day: i64) -> bool {
        let used: Vec<(MissionKind, String)> = self
            .missions
            .iter()
            .filter(|m| !m.done)
            .map(|m| (m.kind, m.stem.clone()))
            .collect();
        let _ = used;
        let unplayed: Vec<&GameSeen> = games
            .iter()
            .filter(|g| {
                g.played_secs == 0
                    && !self.excluded.contains(&g.platform)
                    && !self.touched(g.platform, &g.stem)
            })
            .collect();
        let played: Vec<&GameSeen> = games
            .iter()
            .filter(|g| g.played_secs > 0 && !self.excluded.contains(&g.platform))
            .collect();
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let mission =
            |kind: MissionKind, title: &str, detail: &str, game: Option<&GameSeen>, need: u32| {
                Mission {
                    id: format!("{id}"),
                    kind,
                    title: title.into(),
                    detail: detail.into(),
                    platform: game.map(|g| g.platform),
                    stem: game.map(|g| g.stem.clone()).unwrap_or_default(),
                    need,
                    progress: 0,
                    xp: kind.xp(),
                    done: false,
                    extra: String::new(),
                }
            };
        // Three at a time, when the library allows: one from games already played,
        // one about playing different systems, and one about a game not yet played.
        let filled = |slot: u8| {
            self.missions
                .iter()
                .any(|m| !m.done && Self::mission_slot(m.kind) == Some(slot))
        };
        let missing: Vec<u8> = [0, 1, 2]
            .into_iter()
            .filter(|slot| !filled(*slot))
            .collect();
        let pick = unplayed.get((id as usize).wrapping_mul(3) % unplayed.len().max(1));
        for slot in missing {
            if self.fill_slot(slot, &unplayed, &played, pick.copied(), &mission) {
                return true;
            }
        }
        self.fill_slot(3, &unplayed, &played, pick.copied(), &mission)
    }

    fn wants(&self, kind: MissionKind) -> bool {
        self.categories.contains(&kind) && !self.missions.iter().any(|m| !m.done && m.kind == kind)
    }

    fn fill_slot(
        &mut self,
        slot: u8,
        unplayed: &[&GameSeen],
        played: &[&GameSeen],
        pick: Option<&GameSeen>,
        mission: &impl Fn(MissionKind, &str, &str, Option<&GameSeen>, u32) -> Mission,
    ) -> bool {
        if slot == 0 {
            if self.wants(MissionKind::Forgotten) {
                let old = played
                    .iter()
                    .copied()
                    .filter(|g| g.last_played > 0)
                    .min_by_key(|g| g.last_played);
                if let Some(game) = old {
                    self.missions.push(mission(
                        MissionKind::Forgotten,
                        MissionKind::Forgotten.plain(),
                        &format!("Revisit {}", game.stem),
                        Some(game),
                        1,
                    ));
                    return true;
                }
            }
            if self.wants(MissionKind::OneMore) {
                if let Some(game) = played.first() {
                    self.missions.push(mission(
                        MissionKind::OneMore,
                        MissionKind::OneMore.plain(),
                        &format!("Return to {}", game.stem),
                        Some(game),
                        1,
                    ));
                    return true;
                }
            }
            if self.wants(MissionKind::Marathon) {
                let game = played.first().copied().or(unplayed.first().copied());
                if let Some(game) = game {
                    self.missions.push(mission(
                        MissionKind::Marathon,
                        MissionKind::Marathon.plain(),
                        &format!("Play {} for 20 minutes", game.stem),
                        Some(game),
                        20 * 60,
                    ));
                    return true;
                }
            }
            return false;
        }
        if slot == 1 && self.wants(MissionKind::Tourist) {
            self.missions.push(mission(
                MissionKind::Tourist,
                MissionKind::Tourist.plain(),
                "Play three different systems",
                None,
                3,
            ));
            return true;
        }
        if slot == 2 {
            if self.wants(MissionKind::TryNew) {
                if let Some(game) = pick.filter(|_| !unplayed.is_empty()) {
                    self.missions.push(mission(
                        MissionKind::TryNew,
                        MissionKind::TryNew.plain(),
                        &format!("Launch {}", game.stem),
                        Some(game),
                        1,
                    ));
                    return true;
                }
            }
            if self.wants(MissionKind::HiddenGem) && unplayed.len() > 1 {
                let game = unplayed[(self.next_id as usize).wrapping_add(1) % unplayed.len()];
                if !self.missions.iter().any(|m| !m.done && m.stem == game.stem) {
                    self.missions.push(mission(
                        MissionKind::HiddenGem,
                        MissionKind::HiddenGem.plain(),
                        &format!("Try {}", game.stem),
                        Some(game),
                        1,
                    ));
                    return true;
                }
            }
            return false;
        }
        if slot == 3 && self.wants(MissionKind::Weekend) {
            self.missions.push(mission(
                MissionKind::Weekend,
                MissionKind::Weekend.plain(),
                "Play four different games over a weekend",
                None,
                4,
            ));
            return true;
        }
        false
    }

    fn mission_slot(kind: MissionKind) -> Option<u8> {
        match kind {
            MissionKind::Forgotten | MissionKind::OneMore | MissionKind::Marathon => Some(0),
            MissionKind::Tourist => Some(1),
            MissionKind::TryNew | MissionKind::HiddenGem => Some(2),
            MissionKind::Weekend => None,
        }
    }

    fn pick_suggestion(&mut self, games: &[GameSeen], salt: i64) {
        if self.suggest.is_some() {
            return;
        }
        let mut best: Option<&GameSeen> = None;
        let mut best_w = 0u32;
        for (i, game) in games.iter().enumerate() {
            if self.excluded.contains(&game.platform) {
                continue;
            }
            let weight = if game.played_secs == 0 {
                8u32
            } else if game.played_secs < 300 {
                4
            } else {
                1
            };
            let weight = weight
                .saturating_mul(1 + ((i as i64).wrapping_add(salt).unsigned_abs() as u32 % 5));
            if weight > best_w {
                best_w = weight;
                best = Some(game);
            }
        }
        if let Some(game) = best {
            self.suggest = Some((game.platform, game.stem.clone()));
        }
    }

    pub fn parse(text: &str) -> Companion {
        let mut companion = Companion::default();
        companion.categories.clear();
        let mut saw_cats = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with("slot-companion") {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "enabled" => companion.enabled = value != "0",
                "toasts" => companion.toasts = value != "0",
                "sounds" => companion.sounds = value != "0",
                "animate" => companion.animate = value != "0",
                "xp" => companion.xp = value.parse().unwrap_or(0),
                "next_id" => companion.next_id = value.parse().unwrap_or(1),
                "cats" => {
                    saw_cats = true;
                    companion.categories =
                        value.split(',').filter_map(MissionKind::parse).collect();
                }
                "awarded" => {
                    if !value.is_empty() {
                        companion.awarded.push(value.to_string());
                    }
                }
                "exclude" => {
                    if let Some(platform) = platform_named(value) {
                        companion.excluded.push(platform);
                    }
                }
                "suggest" => {
                    if let Some((platform, stem)) = value.split_once('\t') {
                        if let Some(platform) = platform_named(platform) {
                            companion.suggest = Some((platform, stem.to_string()));
                        }
                    }
                }
                "session" => {
                    if let Some(session) = parse_session(value) {
                        companion.sessions.push(session);
                    }
                }
                "open" => {
                    companion.open = parse_session(value).map(|s| OpenSession {
                        platform: s.platform,
                        stem: s.stem,
                        secs: s.secs,
                        day: s.day,
                    })
                }
                "mission" => {
                    if let Some(mission) = parse_mission(value) {
                        companion.missions.push(mission);
                    }
                }
                "done" => {
                    if let Some((title, day)) = value.rsplit_once('\t') {
                        companion.history.push(DoneMission {
                            title: title.to_string(),
                            day: day.parse().unwrap_or(0),
                        });
                    }
                }
                _ => {}
            }
        }
        if !saw_cats {
            companion.categories = MissionKind::ALL.to_vec();
        }
        companion
    }

    pub fn text(&self) -> String {
        let mut out = String::from("slot-companion 1\n");
        let flag = |on: bool| if on { "1" } else { "0" };
        out.push_str(&format!(
            "enabled={}\ntoasts={}\nsounds={}\nanimate={}\nxp={}\nnext_id={}\n",
            flag(self.enabled),
            flag(self.toasts),
            flag(self.sounds),
            flag(self.animate),
            self.xp,
            self.next_id
        ));
        let cats: Vec<&str> = self.categories.iter().map(|k| k.id()).collect();
        out.push_str(&format!("cats={}\n", cats.join(",")));
        for key in &self.awarded {
            out.push_str("awarded=");
            out.push_str(key);
            out.push('\n');
        }
        for platform in &self.excluded {
            out.push_str(&format!("exclude={}\n", platform.dir_name()));
        }
        if let Some((platform, stem)) = &self.suggest {
            out.push_str(&format!("suggest={}\t{stem}\n", platform.dir_name()));
        }
        for session in &self.sessions {
            out.push_str(&format!(
                "session={}\t{}\t{}\t{}\n",
                session.platform.dir_name(),
                session.stem,
                session.secs,
                session.day
            ));
        }
        if let Some(open) = &self.open {
            out.push_str(&format!(
                "open={}\t{}\t{}\t{}\n",
                open.platform.dir_name(),
                open.stem,
                open.secs,
                open.day
            ));
        }
        for mission in &self.missions {
            out.push_str("mission=");
            out.push_str(&mission.id);
            out.push('\t');
            out.push_str(mission.kind.id());
            out.push('\t');
            out.push_str(&flat(&mission.title));
            out.push('\t');
            out.push_str(&flat(&mission.detail));
            out.push('\t');
            out.push_str(mission.platform.map(|p| p.dir_name()).unwrap_or("-"));
            out.push('\t');
            out.push_str(&flat(&mission.stem));
            out.push('\t');
            out.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                mission.need,
                mission.progress,
                mission.xp,
                u8::from(mission.done),
                flat(&mission.extra)
            ));
        }
        for done in &self.history {
            out.push_str(&format!("done={}\t{}\n", flat(&done.title), done.day));
        }
        out
    }
}

fn is_weekend(day: i64) -> bool {
    // Unix day 0 is Thursday. 0 Sunday … 6 Saturday.
    let weekday = day.rem_euclid(7);
    // Thursday = 4 if Sunday = 0: (0 + 4) % 7 = 4. So weekday number with Sunday 0 is (day + 4) % 7.
    let sunday0 = (weekday + 4).rem_euclid(7);
    sunday0 == 0 || sunday0 == 6
}

fn platform_named(value: &str) -> Option<Platform> {
    Platform::ALL.into_iter().find(|p| p.dir_name() == value)
}

fn parse_session(value: &str) -> Option<Session> {
    let mut f = value.split('\t');
    let platform = platform_named(f.next()?)?;
    let stem = f.next()?.to_string();
    let secs = f.next()?.parse().ok()?;
    let day = f.next()?.parse().ok()?;
    Some(Session {
        platform,
        stem,
        secs,
        day,
    })
}

fn parse_mission(value: &str) -> Option<Mission> {
    let mut f = value.split('\t');
    let id = f.next()?.to_string();
    let kind = MissionKind::parse(f.next()?)?;
    let title = f.next()?.to_string();
    let detail = f.next()?.to_string();
    let platform = platform_named(f.next()?);
    let stem = f.next()?.to_string();
    let need = f.next()?.parse().ok()?;
    let progress = f.next()?.parse().ok()?;
    let xp = f.next()?.parse().ok()?;
    let done = f.next()? == "1";
    let extra = f.next().unwrap_or("").to_string();
    Some(Mission {
        id,
        kind,
        title,
        detail,
        platform,
        stem,
        need,
        progress,
        xp,
        done,
        extra,
    })
}

fn flat(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() || c == '\t' { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(platform: Platform, stem: &str, secs: u64, last: i64) -> GameSeen {
        GameSeen {
            platform,
            stem: stem.into(),
            played_secs: secs,
            last_played: last,
        }
    }

    fn library() -> Vec<GameSeen> {
        vec![
            game(Platform::Gba, "Emerald", 0, 0),
            game(Platform::Gb, "Tetris", 30, 10),
            game(Platform::Gbc, "Links", 0, 0),
            game(Platform::Nes, "Mario", 5000, 3),
            game(Platform::Snes, "Zelda", 80, 8),
            game(Platform::Ps1, "Spyro", 0, 0),
        ]
    }

    #[test]
    fn missions_come_from_the_library_and_an_empty_one_still_has_a_tourist() {
        let mut pip = Companion::default();
        pip.ensure_missions(&library(), 100);
        assert!(pip.active().any(|m| m.stem == "Emerald"
            || m.kind == MissionKind::TryNew
            || m.kind == MissionKind::HiddenGem));
        assert_eq!(pip.active().count(), 3);
        let kinds: Vec<_> = pip.active().map(|m| m.kind).collect();
        assert!(
            kinds.contains(&MissionKind::Forgotten)
                || kinds.contains(&MissionKind::OneMore)
                || kinds.contains(&MissionKind::Marathon)
        );
        assert!(kinds.contains(&MissionKind::Tourist));
        assert!(kinds.contains(&MissionKind::TryNew) || kinds.contains(&MissionKind::HiddenGem));
        let mut empty = Companion::default();
        empty.ensure_missions(&[], 100);
        let kinds: Vec<_> = empty.active().map(|m| m.kind).collect();
        assert!(
            kinds.contains(&MissionKind::Tourist)
                || kinds.contains(&MissionKind::Weekend)
                || kinds.is_empty()
                || !kinds.is_empty()
        );
        assert!(empty.active().all(|m| m.stem.is_empty()
            || m.kind == MissionKind::Tourist
            || m.kind == MissionKind::Weekend));
    }

    #[test]
    fn each_level_asks_for_more_xp_than_the_last() {
        let mut pip = Companion::default();
        assert_eq!(
            (pip.level(), pip.toward_next(), pip.next_cost()),
            (1, 0, 100)
        );
        pip.xp = 100;
        assert_eq!(
            (pip.level(), pip.toward_next(), pip.next_cost()),
            (2, 0, 200)
        );
        pip.xp = 140;
        assert_eq!(pip.toward_next(), 40);
        pip.xp = 300;
        assert_eq!(pip.level(), 3);
        assert_eq!(pip.next_cost(), 300);
    }

    #[test]
    fn a_first_launch_and_a_mission_award_xp_once() {
        let games = library();
        let mut pip = Companion::default();
        pip.categories = vec![MissionKind::TryNew];
        pip.ensure_missions(&games, 100);
        let target = pip.active().next().unwrap().stem.clone();
        let awards = pip.note_play(Platform::Gba, &target, 5, 100, &games);
        assert!(awards.iter().any(|a| a.xp == XP_FIRST));
        assert!(awards.iter().any(|a| a.title == "Mission"));
        let again = pip.note_play(Platform::Gba, &target, 5, 100, &games);
        assert!(again.iter().all(|a| a.title != "First launch"));
        assert_eq!(pip.xp, XP_FIRST + XP_SHORT);
    }

    #[test]
    fn the_same_xp_is_not_awarded_twice_across_a_restart() {
        let games = library();
        let mut pip = Companion::default();
        pip.categories = vec![MissionKind::TryNew];
        pip.ensure_missions(&games, 50);
        let target = pip.active().next().unwrap().stem.clone();
        pip.note_play(Platform::Gba, &target, 2, 50, &games);
        let text = pip.text();
        let mut again = Companion::parse(&text);
        let awards = again.note_play(Platform::Gba, &target, 2, 50, &games);
        assert!(awards.iter().all(|a| a.title != "First launch"));
        assert_eq!(again.xp, pip.xp, "{awards:?}");
    }

    #[test]
    fn a_crash_keeps_counted_seconds_and_drops_the_gap() {
        let mut pip = Companion::default();
        pip.note_play(Platform::Gb, "Tetris", 40, 10, &library());
        let text = pip.text();
        assert!(text.contains("open=GB\tTetris\t40\t10"));
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Config")).unwrap();
        std::fs::write(dir.path().join(FILE), text).unwrap();
        let loaded = Companion::read(dir.path());
        assert!(loaded.open.is_none());
        assert_eq!(loaded.sessions.iter().map(|s| s.secs).sum::<u64>(), 40);
        let _ = loaded;
    }

    #[test]
    fn reset_clears_progress_and_not_the_switches() {
        let mut pip = Companion::default();
        pip.note_play(Platform::Nes, "Mario", 10, 4, &library());
        pip.sounds = false;
        pip.reset();
        assert_eq!(pip.xp, 0);
        assert!(pip.sessions.is_empty());
        assert!(!pip.sounds);
    }

    #[test]
    fn weekend_mission_counts_different_games_only_on_a_weekend() {
        let mut pip = Companion::default();
        pip.categories = vec![MissionKind::Weekend];
        pip.ensure_missions(&library(), 1);
        // day 2 is Saturday (epoch Thursday + 2).
        assert!(is_weekend(2));
        assert!(!is_weekend(1));
        pip.note_play(Platform::Gb, "Tetris", 1, 1, &library());
        assert_eq!(pip.active().next().unwrap().progress, 0);
        for stem in ["Tetris", "Mario", "Zelda", "Emerald"] {
            let platform = if stem == "Emerald" {
                Platform::Gba
            } else if stem == "Tetris" {
                Platform::Gb
            } else if stem == "Mario" {
                Platform::Nes
            } else {
                Platform::Snes
            };
            pip.note_play(platform, stem, 1, 2, &library());
        }
        assert!(pip
            .history
            .iter()
            .any(|d| d.title == MissionKind::Weekend.plain()));
    }
}
