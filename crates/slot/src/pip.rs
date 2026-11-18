//! HOME → Companion. Pip, an original pixel creature, and the missions it keeps.
//! Not PakUI's companion: different name, different drawing, Slot's own plate.
use slot_gfx::{OUT_H, OUT_W};
use slot_input::{Action, Btn};
use slot_store::{Companion, GameSeen, MissionKind, Mood, Platform};
use slot_ui::{TexId, UndoFace};

use crate::sheet::Sheet;

const SCALE: u32 = 3;
const CELL: u32 = 32;

pub struct PipLaunch {
    pub platform: Platform,
    pub stem: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Main,
    History,
    Settings,
}

pub struct PipMenu {
    page: Page,
    row: usize,
    reset_armed: bool,
    pub dirty: bool,
    pub face: Option<TexId>,
    launch: Option<PipLaunch>,
}

impl PipMenu {
    pub fn open() -> Self {
        PipMenu {
            page: Page::Main,
            row: 0,
            reset_armed: false,
            dirty: true,
            face: None,
            launch: None,
        }
    }

    pub fn take_launch(&mut self) -> Option<PipLaunch> {
        self.launch.take()
    }

    pub fn game(&self, pip: &Companion) -> Option<String> {
        pip.suggestion().map(|(_, stem)| stem).or_else(|| {
            pip.active()
                .find(|m| !m.stem.is_empty())
                .map(|m| m.stem.clone())
        })
    }

    /// True when the screen should close. `pip` is updated in place.
    pub fn input(
        &mut self,
        action: Action,
        pip: &mut Companion,
        games: &[GameSeen],
        day: i64,
    ) -> bool {
        if !matches!(
            action,
            Action::GbaDown(_) | Action::ShelfLeft | Action::ShelfRight
        ) {
            return false;
        }
        self.dirty = true;
        if self.page != Page::Main && matches!(action, Action::GbaDown(Btn::B)) {
            self.page = Page::Main;
            self.row = 0;
            self.reset_armed = false;
            return false;
        }
        let rows = self.rows(pip);
        match action {
            Action::GbaDown(Btn::B) => return true,
            Action::GbaDown(Btn::Up) => self.row = (self.row + rows - 1) % rows,
            Action::GbaDown(Btn::Down) => self.row = (self.row + 1) % rows,
            Action::GbaDown(Btn::X) if self.page == Page::Main => {
                pip.refresh_suggestion(games, day)
            }
            Action::GbaDown(Btn::Y) if self.page == Page::Main => {
                let active: Vec<usize> = pip
                    .missions
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| !m.done)
                    .map(|(i, _)| i)
                    .collect();
                if let Some(at) = active.get(self.row) {
                    let _ = pip.complete_by_hand(*at, day);
                }
            }
            Action::GbaDown(Btn::L1) | Action::ShelfLeft if self.page == Page::Main => {
                if let Some((platform, _)) = pip.suggestion() {
                    pip.toggle_excluded(platform);
                    pip.refresh_suggestion(games, day);
                }
            }
            Action::GbaDown(Btn::A) => self.confirm(pip, games, day),
            _ => {}
        }
        false
    }

    fn confirm(&mut self, pip: &mut Companion, games: &[GameSeen], day: i64) {
        match self.page {
            Page::Main => {
                let n = pip.active().count();
                if self.row < n {
                    let mission = pip.active().nth(self.row).cloned();
                    if let Some(mission) = mission {
                        if let Some(platform) = mission.platform {
                            self.launch = Some(PipLaunch {
                                platform,
                                stem: mission.stem,
                            });
                        }
                    }
                } else if self.row == n {
                    if let Some((platform, stem)) = pip.suggestion() {
                        self.launch = Some(PipLaunch { platform, stem });
                    }
                } else if self.row == n + 1 {
                    self.page = Page::History;
                    self.row = 0;
                } else {
                    self.page = Page::Settings;
                    self.row = 0;
                }
            }
            Page::Settings => self.settings(pip, games, day),
            Page::History => {}
        }
    }

    fn settings(&mut self, pip: &mut Companion, games: &[GameSeen], day: i64) {
        let cats = MissionKind::ALL.len();
        match self.row {
            0 => pip.set_enabled(!pip.enabled),
            1 => pip.toasts = !pip.toasts,
            2 => pip.sounds = !pip.sounds,
            3 => pip.animate = !pip.animate,
            n if n < 4 + cats => {
                pip.toggle_category(MissionKind::ALL[n - 4]);
                pip.ensure_missions(games, day);
            }
            _ => {
                if self.reset_armed {
                    pip.reset();
                    self.reset_armed = false;
                } else {
                    self.reset_armed = true;
                }
            }
        }
    }

    fn rows(&self, pip: &Companion) -> usize {
        match self.page {
            Page::Main => pip.active().count() + 3,
            Page::History => 1,
            Page::Settings => 4 + MissionKind::ALL.len() + 1,
        }
        .max(1)
    }

    pub fn render(&self, pip: &Companion, day: i64, games: &[GameSeen], frame: u64) -> UndoFace {
        let mut sheet = Sheet::new();
        let w = OUT_W.saturating_sub(48);
        match self.page {
            Page::Main => self.render_main(&mut sheet, pip, day, games, frame, w),
            Page::History => self.render_history(&mut sheet, pip, w),
            Page::Settings => self.render_settings(&mut sheet, pip, w),
        }
        sheet.finish()
    }

    fn render_main(
        &self,
        sheet: &mut Sheet,
        pip: &Companion,
        day: i64,
        games: &[GameSeen],
        frame: u64,
        w: u32,
    ) {
        let mood = pip.mood(day, games);
        let blink = pip.animate && frame % 8 > 5;
        let sprite = sprite(mood, blink);
        let size = CELL * SCALE;
        sheet.image((OUT_W - size) / 2, 8, size, size, &sprite);
        sheet.text("PIP", 24, 8, 80, 24, 16.0, false);
        sheet.text(mood.name(), 24, 112, w, 22, 15.0, true);
        sheet.text(
            &format!("LEVEL {}    {} / 100 XP", pip.level(), pip.toward_next()),
            24,
            134,
            w,
            22,
            15.0,
            false,
        );
        let mut row = 0usize;
        let mut y = 164u32;
        for mission in pip.active() {
            if row == self.row {
                sheet.rect(0, y, OUT_W, 36, slot_ui::edge());
            }
            sheet.text(
                &format!(
                    "{}  {}/{}",
                    mission.title,
                    mission.progress.min(mission.need),
                    mission.need
                ),
                16,
                y + 2,
                w,
                32,
                15.0,
                false,
            );
            row += 1;
            y += 38;
            if y > 360 {
                break;
            }
        }
        let suggest = pip
            .suggestion()
            .map(|(_, stem)| format!("Next: {stem}"))
            .unwrap_or_else(|| "Next: nothing new".into());
        if row == self.row {
            sheet.rect(0, y, OUT_W, 32, slot_ui::edge());
        }
        sheet.text(&suggest, 16, y, w, 28, 15.0, false);
        y += 34;
        row += 1;
        for (label, at) in [("History", row), ("Settings", row + 1)] {
            if self.row == at {
                sheet.rect(0, y, OUT_W, 28, slot_ui::edge());
            }
            sheet.text(label, 16, y, w, 26, 15.0, false);
            y += 30;
        }
        sheet.text(
            "A PLAY    X ANOTHER    Y I DID IT    L SKIP SYSTEM    B BACK",
            8,
            OUT_H.saturating_sub(28),
            OUT_W.saturating_sub(16),
            24,
            12.0,
            false,
        );
    }

    fn render_history(&self, sheet: &mut Sheet, pip: &Companion, w: u32) {
        sheet.text("RECENTLY", 24, 20, w, 32, 24.0, false);
        if pip.history.is_empty() && pip.sessions.is_empty() {
            sheet.text(
                "No outings yet. Pip is happy to wait.",
                24,
                80,
                w,
                28,
                15.0,
                true,
            );
        }
        let mut y = 70u32;
        for session in pip.sessions.iter().rev().take(6) {
            sheet.text(
                &format!(
                    "{}  {}  {}s",
                    session.platform.dir_name(),
                    session.stem,
                    session.secs
                ),
                24,
                y,
                w,
                24,
                14.0,
                false,
            );
            y += 26;
        }
        y += 8;
        for done in pip.history.iter().rev().take(4) {
            sheet.text(&format!("Done  {}", done.title), 24, y, w, 24, 14.0, true);
            y += 26;
        }
        sheet.text("B BACK", 16, OUT_H.saturating_sub(28), w, 24, 15.0, false);
    }

    fn render_settings(&self, sheet: &mut Sheet, pip: &Companion, w: u32) {
        sheet.text("PIP'S SETTINGS", 24, 16, w, 28, 20.0, false);
        let mut rows = vec![
            format!("Pip  {}", on(pip.enabled)),
            format!("Toasts  {}", on(pip.toasts)),
            format!("Sounds  {}", on(pip.sounds)),
            format!("Animations  {}", on(pip.animate)),
        ];
        for kind in MissionKind::ALL {
            let name = match kind {
                MissionKind::TryNew => "Try something new",
                MissionKind::Marathon => "Retro marathon",
                MissionKind::Forgotten => "Forgotten favorite",
                MissionKind::Tourist => "Console tourist",
                MissionKind::OneMore => "One more round",
                MissionKind::HiddenGem => "Hidden gem",
                MissionKind::Weekend => "Weekend adventure",
            };
            rows.push(format!("{name}  {}", on(pip.categories.contains(&kind))));
        }
        rows.push(if self.reset_armed {
            "Reset progress?  A AGAIN".into()
        } else {
            "Reset progress".into()
        });
        for (i, label) in rows.iter().enumerate() {
            let y = 52 + i as u32 * 28;
            if i == self.row {
                sheet.rect(0, y, OUT_W, 26, slot_ui::edge());
            }
            sheet.text(label, 16, y, w, 24, 13.0, false);
        }
        sheet.text(
            "A CHANGE    B BACK",
            16,
            OUT_H.saturating_sub(24),
            w,
            22,
            14.0,
            false,
        );
    }
}

fn on(flag: bool) -> &'static str {
    if flag {
        "On"
    } else {
        "Off"
    }
}

/// Pip, drawn here. A round little creature with a slot-shaped mark. Original.
fn sprite(mood: Mood, blink: bool) -> Vec<u8> {
    let n = CELL * SCALE;
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    let sand = [0xef, 0xc8, 0x8a, 255];
    let ink = [0x2a, 0x22, 0x18, 255];
    let blush = [0xe0, 0x78, 0x68, 255];
    let mark = [0xc4, 0x48, 0x2a, 255];
    for y in 0..CELL {
        for x in 0..CELL {
            let dx = x as i32 - 16;
            let dy = y as i32 - 15;
            let body = dx * dx + (dy * dy * 3) / 2 < 120;
            let edge = dx * dx + (dy * dy * 3) / 2 > 96;
            let mut color = if body {
                if edge {
                    ink
                } else {
                    sand
                }
            } else {
                [0, 0, 0, 0]
            };
            // Feet.
            if (8..12).contains(&x) && (26..30).contains(&y)
                || (20..24).contains(&x) && (26..30).contains(&y)
            {
                color = ink;
            }
            // Belly slot.
            if (13..19).contains(&x) && (16..22).contains(&y) && body {
                color = if x == 13 || x == 18 || y == 16 || y == 21 {
                    ink
                } else {
                    mark
                };
            }
            // Eyes.
            let eye_y = 12;
            let shut = blink || mood == Mood::Sleepy;
            for ex in [11, 19] {
                if (ex..ex + 3).contains(&x)
                    && (eye_y..eye_y + if shut { 1 } else { 3 }).contains(&y)
                {
                    color = ink;
                }
            }
            // Mouth.
            let smile = matches!(mood, Mood::Happy | Mood::Excited | Mood::Celebrating);
            if smile && (12..20).contains(&x) && y == 24 {
                color = ink;
            }
            if mood == Mood::Curious && (14..18).contains(&x) && y == 23 {
                color = ink;
            }
            if matches!(mood, Mood::Excited | Mood::Celebrating)
                && ((8..11).contains(&x) || (21..24).contains(&x))
                && (14..17).contains(&y)
            {
                color = blush;
            }
            if mood == Mood::Celebrating && (x + y) % 9 == 0 && !body {
                color = [0xff, 0xd8, 0x6a, 255];
            }
            if color[3] == 0 {
                continue;
            }
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    let at = (((y * SCALE + sy) * n + x * SCALE + sx) * 4) as usize;
                    rgba[at..at + 4].copy_from_slice(&color);
                }
            }
        }
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::*;
    use slot_store::MissionKind;

    fn games() -> Vec<GameSeen> {
        vec![GameSeen {
            platform: Platform::Gba,
            stem: "Emerald".into(),
            played_secs: 0,
            last_played: 0,
        }]
    }

    #[test]
    fn the_controls_launch_refresh_and_come_back() {
        let mut menu = PipMenu::open();
        let mut pip = Companion::default();
        pip.ensure_missions(&games(), 10);
        assert!(!menu.input(Action::GbaDown(Btn::X), &mut pip, &games(), 10));
        assert!(pip.suggestion().is_some() || pip.active().next().is_some());
        menu.row = pip.active().count();
        let closed = menu.input(Action::GbaDown(Btn::A), &mut pip, &games(), 10);
        assert!(!closed);
        assert!(menu.take_launch().is_some() || pip.suggestion().is_none());
        assert!(menu.input(Action::GbaDown(Btn::B), &mut pip, &games(), 10));
    }

    #[test]
    fn settings_can_turn_a_mission_kind_off_and_reset_asks_twice() {
        let mut menu = PipMenu::open();
        let mut pip = Companion::default();
        menu.page = Page::Settings;
        menu.row = 4;
        menu.input(Action::GbaDown(Btn::A), &mut pip, &games(), 1);
        assert!(!pip.categories.contains(&MissionKind::TryNew));
        menu.row = 4 + MissionKind::ALL.len();
        menu.input(Action::GbaDown(Btn::A), &mut pip, &games(), 1);
        assert!(menu.reset_armed);
        pip.xp = 40;
        menu.input(Action::GbaDown(Btn::A), &mut pip, &games(), 1);
        assert_eq!(pip.xp, 0);
        let face = menu.render(&pip, 1, &games(), 0);
        assert_eq!(face.w, OUT_W);
        assert!(face.rgba.iter().any(|b| *b != 0));
        if let Ok(dir) = std::env::var("SLOT_FEATURE_PREVIEW") {
            let dir = std::path::PathBuf::from(dir);
            let _ = std::fs::create_dir_all(&dir);
            let mut shown = PipMenu::open();
            let mut pip = Companion::default();
            pip.xp = 140;
            pip.ensure_missions(&games(), 3);
            write_png(&dir.join("pip.png"), &shown.render(&pip, 3, &games(), 0));
            shown.page = Page::Settings;
            write_png(
                &dir.join("pip-settings.png"),
                &shown.render(&pip, 3, &games(), 2),
            );
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
