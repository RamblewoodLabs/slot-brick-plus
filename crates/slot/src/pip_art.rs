//! Pip, drawn as a small walking cartridge. Original pixel art, 48×48, scaled ×2
//! with whole pixels so the Brick's 1024×768 picture stays crisp.
//!
//! Warm orange shell, cream label, black outline, a face on the label, and little arms and legs.
use slot_store::Mood;

pub const CELL: u32 = 48;
pub const SCALE: u32 = 2;
pub const FRAMES: u32 = 8;

const ORANGE: [u8; 4] = [0xe8, 0x6a, 0x2a, 255];
const SHELL: [u8; 4] = [0xc4, 0x4e, 0x16, 255];
const CREAM: [u8; 4] = [0xfb, 0xf0, 0xd0, 255];
const INK: [u8; 4] = [0x1a, 0x14, 0x12, 255];
const BLUSH: [u8; 4] = [0xe8, 0x78, 0x68, 255];
const CLOUD: [u8; 4] = [0xf4, 0xf7, 0xfb, 255];
const RAIN: [u8; 4] = [0x5c, 0xa8, 0xe0, 255];
const RED: [u8; 4] = [0xd2, 0x32, 0x28, 255];
const GOLD: [u8; 4] = [0xff, 0xd2, 0x4a, 255];
const MINT: [u8; 4] = [0x7d, 0xd6, 0x8a, 255];
const SKY: [u8; 4] = [0x6e, 0xb6, 0xf0, 255];

struct Pix {
    px: Vec<u8>,
}

impl Pix {
    fn new() -> Self {
        Self {
            px: vec![0; (CELL * CELL * 4) as usize],
        }
    }

    fn dot(&mut self, x: i32, y: i32, c: [u8; 4]) {
        if x < 0 || y < 0 || x >= CELL as i32 || y >= CELL as i32 || c[3] == 0 {
            return;
        }
        let at = ((y as u32 * CELL + x as u32) * 4) as usize;
        self.px[at..at + 4].copy_from_slice(&c);
    }

    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: [u8; 4]) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.dot(xx, yy, c);
            }
        }
    }

    fn box_(&mut self, x: i32, y: i32, w: i32, h: i32, fill: [u8; 4]) {
        self.rect(x, y, w, h, fill);
        for xx in x..x + w {
            self.dot(xx, y, INK);
            self.dot(xx, y + h - 1, INK);
        }
        for yy in y..y + h {
            self.dot(x, yy, INK);
            self.dot(x + w - 1, yy, INK);
        }
    }

    fn scaled(&self) -> Vec<u8> {
        let n = CELL * SCALE;
        let mut out = vec![0u8; (n * n * 4) as usize];
        for y in 0..CELL {
            for x in 0..CELL {
                let src = ((y * CELL + x) * 4) as usize;
                let px = &self.px[src..src + 4];
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let at = (((y * SCALE + sy) * n + x * SCALE + sx) * 4) as usize;
                        out[at..at + 4].copy_from_slice(px);
                    }
                }
            }
        }
        out
    }
}

struct Look {
    /// Whole-sprite shift. Negative is up (a jump).
    dy: i32,
    blink: bool,
    /// 0 smile, 1 flat, 2 frown, 3 teeth, 4 yawn, 5 o.
    mouth: u8,
    /// 0 none, 1 angry, 2 sad, 3 one brow up.
    brow: u8,
    /// 0 at the side, 1 out, 2 waving up.
    wave: u8,
    /// Both hands up, for a level-up. A wave lifts only the right hand.
    both: bool,
    stomp: bool,
    /// 0 none, 1 rain, 2 red scribble, 3 zzz, 4 confetti.
    extra: u8,
    phase: u8,
    blush: bool,
}

/// One animation frame, already scaled ×2. `animate` false is the resting pose.
pub fn frame(mood: Mood, tick: u64, animate: bool) -> Vec<u8> {
    let tick = if animate { tick % u64::from(FRAMES) } else { 0 };
    let mut pix = Pix::new();
    cart(&mut pix, look(mood, tick as u8));
    pix.scaled()
}

fn look(mood: Mood, i: u8) -> Look {
    let bob = [0, 0, 1, 1, 0, 0, 1, 1][i as usize];
    match mood {
        Mood::Neutral => Look {
            dy: bob,
            blink: i >= 5,
            mouth: 1,
            brow: 0,
            wave: 0,
            both: false,
            stomp: false,
            extra: 0,
            phase: i,
            blush: false,
        },
        Mood::Curious => Look {
            dy: bob,
            blink: i == 6,
            mouth: 5,
            brow: 3,
            wave: 0,
            both: false,
            stomp: false,
            extra: 0,
            phase: i,
            blush: false,
        },
        Mood::Happy => Look {
            dy: if i % 2 == 0 { 0 } else { -1 },
            blink: i == 4,
            mouth: 0,
            brow: 0,
            wave: [0, 1, 2, 2, 1, 0, 1, 2][i as usize],
            both: false,
            stomp: false,
            extra: 0,
            phase: i,
            blush: true,
        },
        Mood::Sad => Look {
            dy: 2,
            blink: false,
            mouth: 2,
            brow: 2,
            wave: 0,
            both: false,
            stomp: false,
            extra: 1,
            phase: i,
            blush: false,
        },
        Mood::Frustrated => Look {
            dy: if i % 2 == 0 { 0 } else { 2 },
            blink: false,
            mouth: 3,
            brow: 1,
            wave: 0,
            both: false,
            stomp: i % 2 == 1,
            extra: 2,
            phase: i,
            blush: false,
        },
        Mood::Excited => Look {
            dy: [0, -3, -6, -6, -3, 0, -1, 0][i as usize],
            blink: false,
            mouth: 0,
            brow: 0,
            wave: 2,
            both: false,
            stomp: false,
            extra: 4,
            phase: i,
            blush: true,
        },
        Mood::Sleepy => Look {
            dy: 1,
            blink: i < 3 || i > 5,
            mouth: if (3..6).contains(&i) { 4 } else { 1 },
            brow: 2,
            wave: 0,
            both: false,
            stomp: false,
            extra: 3,
            phase: i,
            blush: false,
        },
        Mood::Celebrating => Look {
            dy: [0, -2, -5, -5, -2, 0, -2, 0][i as usize],
            blink: false,
            mouth: 0,
            brow: 0,
            wave: 2,
            both: true,
            stomp: false,
            extra: 4,
            phase: i,
            blush: true,
        },
    }
}

fn cart(p: &mut Pix, look: Look) {
    let s = look.dy;
    // Connector along the top, then the orange shell.
    p.box_(18, 6 + s, 12, 5, SHELL);
    p.dot(21, 7 + s, INK);
    p.dot(26, 7 + s, INK);
    p.box_(14, 10 + s, 20, 24, ORANGE);
    // Side ribs.
    for y in [14, 18, 22, 26] {
        p.dot(15, y + s, SHELL);
        p.dot(32, y + s, SHELL);
    }
    // Cream label, where the face sits.
    p.box_(17, 13 + s, 14, 16, CREAM);
    face(p, s, &look);
    arm(p, 11, 20 + s, if look.both { look.wave } else { 0 });
    arm(p, 34, 20 + s, look.wave);
    leg(p, 18, 33 + s, look.stomp);
    leg(p, 26, 33 + s, look.stomp && look.phase % 2 == 0);
    match look.extra {
        1 => rain(p, look.phase),
        2 => scribble(p, look.phase),
        3 => zzz(p, look.phase),
        4 => confetti(p, look.phase),
        _ => {}
    }
}

fn face(p: &mut Pix, s: i32, look: &Look) {
    let y = 16 + s;
    brow(p, 20, y - 2, look.brow, true);
    brow(p, 26, y - 2, look.brow, false);
    eye(p, 20, y, look.blink);
    eye(p, 26, y, look.blink);
    if look.blush {
        p.dot(18, y + 3, BLUSH);
        p.dot(29, y + 3, BLUSH);
    }
    mouth(p, 22, y + 6, look.mouth);
}

fn brow(p: &mut Pix, x: i32, y: i32, kind: u8, left: bool) {
    match kind {
        1 => {
            // Angry: inner end down.
            let drop = if left { 1 } else { 0 };
            p.dot(x, y + drop, INK);
            p.dot(x + 1, y + 1 - drop, INK);
            p.dot(x + 2, y + if left { 0 } else { 1 }, INK);
        }
        2 => {
            p.dot(x, y + 1, INK);
            p.dot(x + 1, y + 1, INK);
            p.dot(x + 2, y, INK);
        }
        3 if left => {
            p.dot(x, y - 1, INK);
            p.dot(x + 1, y, INK);
            p.dot(x + 2, y, INK);
        }
        3 => {
            p.dot(x, y, INK);
            p.dot(x + 1, y, INK);
            p.dot(x + 2, y, INK);
        }
        _ => {}
    }
}

fn eye(p: &mut Pix, x: i32, y: i32, shut: bool) {
    if shut {
        p.dot(x, y + 1, INK);
        p.dot(x + 1, y + 1, INK);
        return;
    }
    p.rect(x, y, 2, 2, INK);
}

fn mouth(p: &mut Pix, x: i32, y: i32, kind: u8) {
    match kind {
        0 => {
            p.dot(x, y, INK);
            p.dot(x + 1, y + 1, INK);
            p.dot(x + 2, y + 1, INK);
            p.dot(x + 3, y, INK);
        }
        2 => {
            p.dot(x, y + 1, INK);
            p.dot(x + 1, y, INK);
            p.dot(x + 2, y, INK);
            p.dot(x + 3, y + 1, INK);
        }
        3 => {
            for dx in 0..4 {
                p.dot(x + dx, y, INK);
                if dx % 2 == 0 {
                    p.dot(x + dx, y + 1, CREAM);
                    p.dot(x + dx, y - 1, INK);
                }
            }
        }
        4 => {
            p.box_(x, y - 1, 4, 4, INK);
            p.rect(x + 1, y, 2, 2, CREAM);
        }
        5 => {
            p.dot(x + 1, y, INK);
            p.dot(x + 2, y, INK);
            p.dot(x + 1, y + 1, INK);
            p.dot(x + 2, y + 1, INK);
        }
        _ => {
            p.dot(x, y, INK);
            p.dot(x + 1, y, INK);
            p.dot(x + 2, y, INK);
            p.dot(x + 3, y, INK);
        }
    }
}

fn arm(p: &mut Pix, x: i32, y: i32, wave: u8) {
    let lift = match wave {
        2 => -6,
        1 => -3,
        _ => 0,
    };
    p.rect(x, y + lift, 3, 5, ORANGE);
    p.dot(x, y + lift, INK);
    p.dot(x + 2, y + lift, INK);
    p.dot(x, y + lift + 4, INK);
    p.dot(x + 1, y + lift + 4, INK);
    p.dot(x + 2, y + lift + 4, INK);
}

fn leg(p: &mut Pix, x: i32, y: i32, tucked: bool) {
    let h = if tucked { 3 } else { 5 };
    p.rect(x, y, 3, h, ORANGE);
    p.rect(x, y + h - 1, 4, 2, INK);
}

fn rain(p: &mut Pix, phase: u8) {
    cloud(p, 32, 1);
    for (n, x) in [34, 37, 40].into_iter().enumerate() {
        let y = 8 + ((phase as i32 + n as i32 * 3) % 8);
        p.dot(x, y, RAIN);
        p.dot(x, y + 1, RAIN);
    }
}

fn cloud(p: &mut Pix, x: i32, y: i32) {
    p.rect(x + 1, y + 1, 8, 4, CLOUD);
    p.rect(x + 2, y, 5, 2, CLOUD);
    for dx in 0..10 {
        p.dot(x + dx, y + 1, INK);
        p.dot(x + dx, y + 5, INK);
    }
    p.dot(x, y + 2, INK);
    p.dot(x, y + 3, INK);
    p.dot(x + 9, y + 2, INK);
    p.dot(x + 9, y + 3, INK);
}

fn scribble(p: &mut Pix, phase: u8) {
    let y = 2 + i32::from(phase % 2);
    for dx in 0..9 {
        let jag = if dx % 2 == 0 { 0 } else { 2 };
        p.dot(31 + dx, y + jag, RED);
        p.dot(31 + dx, y + 4 - jag, RED);
    }
    p.dot(33, y + 2, RED);
    p.dot(36, y + 2, RED);
}

fn zzz(p: &mut Pix, phase: u8) {
    let rise = i32::from(phase % 4);
    zee(p, 34, 8 - rise, 3);
    if phase > 2 {
        zee(p, 38, 4 - rise / 2, 2);
    }
}

fn zee(p: &mut Pix, x: i32, y: i32, w: i32) {
    for dx in 0..w {
        p.dot(x + dx, y, INK);
        p.dot(x + w - 1 - dx, y + dx.min(2), INK);
        p.dot(x + dx, y + 2, INK);
    }
}

fn confetti(p: &mut Pix, phase: u8) {
    let bits = [
        (6, 8, GOLD),
        (10, 14, MINT),
        (8, 20, SKY),
        (40, 8, RED),
        (42, 16, GOLD),
        (38, 22, MINT),
        (4, 28, SKY),
        (44, 28, RED),
    ];
    for (n, (x, y, c)) in bits.into_iter().enumerate() {
        let dy = (i32::from(phase) + n as i32) % 4;
        p.dot(x, y + dy, c);
        if n % 2 == 0 {
            p.dot(x + 1, y + dy, c);
        }
    }
}

/// Eight frames of one mood, side by side, with a one-pixel gap. ×2, so the pixels stay square.
#[cfg(test)]
pub fn strip(mood: Mood) -> (u32, u32, Vec<u8>) {
    let gap = 2u32;
    let fw = CELL * SCALE;
    let fh = CELL * SCALE;
    let w = FRAMES * fw + (FRAMES - 1) * gap;
    let mut out = vec![0u8; (w * fh * 4) as usize];
    for i in 0..FRAMES {
        let frame = frame(mood, u64::from(i), true);
        let x0 = i * (fw + gap);
        for y in 0..fh {
            for x in 0..fw {
                let src = ((y * fw + x) * 4) as usize;
                let dst = ((y * w + x0 + x) * 4) as usize;
                out[dst..dst + 4].copy_from_slice(&frame[src..src + 4]);
            }
        }
    }
    (w, fh, out)
}

#[cfg(test)]
pub fn write_png(path: &std::path::Path, w: u32, h: u32, rgba: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, w, h);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paint_at(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * w + x) * 4) as usize;
        rgba[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn every_mood_is_an_orange_cartridge_with_a_cream_label() {
        for mood in [
            Mood::Neutral,
            Mood::Happy,
            Mood::Sad,
            Mood::Frustrated,
            Mood::Excited,
            Mood::Sleepy,
            Mood::Celebrating,
            Mood::Curious,
        ] {
            let px = frame(mood, 0, true);
            let n = CELL * SCALE;
            assert_eq!(px.len(), (n * n * 4) as usize);
            assert!(px.chunks(4).any(|p| p == ORANGE), "{mood:?} has no orange");
            assert!(px.chunks(4).any(|p| p == CREAM), "{mood:?} has no label");
            assert!(px.chunks(4).any(|p| p == INK), "{mood:?} has no outline");
            assert_eq!(paint_at(&px, n, 0, 0)[3], 0, "the corner stays clear");
        }
    }

    #[test]
    fn animations_move_and_the_still_setting_holds_the_first_frame() {
        let a = frame(Mood::Neutral, 0, true);
        let b = frame(Mood::Neutral, 5, true);
        assert_ne!(a, b, "a blink should change the idle frame");
        assert_eq!(frame(Mood::Happy, 0, false), frame(Mood::Happy, 5, false));
        let jump = frame(Mood::Excited, 2, true);
        let land = frame(Mood::Excited, 5, true);
        assert_ne!(jump, land);
        let (w, h, sad) = strip(Mood::Sad);
        assert_eq!(
            (w, h),
            (FRAMES * CELL * SCALE + (FRAMES - 1) * 2, CELL * SCALE)
        );
        assert!(sad.chunks(4).any(|p| p == RAIN));
        assert!(strip(Mood::Frustrated).2.chunks(4).any(|p| p == RED));
        assert!(strip(Mood::Sleepy).2.chunks(4).any(|p| p == INK));
        assert!(strip(Mood::Celebrating).2.chunks(4).any(|p| p == GOLD));
    }

    #[test]
    fn preview_strips_are_rgba_pngs() {
        let Some(dir) = std::env::var_os("SLOT_PIP_STRIPS") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        for (mood, name) in [
            (Mood::Neutral, "idle"),
            (Mood::Happy, "happy"),
            (Mood::Sad, "sad"),
            (Mood::Frustrated, "frustrated"),
            (Mood::Excited, "excited"),
            (Mood::Sleepy, "sleepy"),
            (Mood::Celebrating, "celebrating"),
            (Mood::Curious, "curious"),
        ] {
            let (w, h, rgba) = strip(mood);
            write_png(&dir.join(format!("{name}.png")), w, h, &rgba).unwrap();
        }
    }
}
