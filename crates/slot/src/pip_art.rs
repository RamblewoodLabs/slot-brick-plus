//! Pip, a chunky walking cartridge. Original pixel art on a 64×64 grid, scaled ×2
//! with whole pixels so the Brick's 1024×768 picture stays crisp.
//!
//! Orange shell with a top tab, side shade and highlight, a large cream label for the
//! face, big eyes, cream sneakers, and arms that actually change pose.
use slot_store::Mood;

pub const CELL: u32 = 64;
pub const SCALE: u32 = 2;
pub const FRAMES: u32 = 8;

const ORANGE: [u8; 4] = [0xf0, 0x78, 0x28, 255];
const DARK: [u8; 4] = [0xc4, 0x4c, 0x14, 255];
const LIGHT: [u8; 4] = [0xff, 0xb0, 0x58, 255];
const CREAM: [u8; 4] = [0xff, 0xf0, 0xc8, 255];
const CREAM_DK: [u8; 4] = [0xe8, 0xd0, 0x98, 255];
const SHOE: [u8; 4] = [0xff, 0xf6, 0xe4, 255];
const INK: [u8; 4] = [0x1c, 0x14, 0x12, 255];
const WHITE: [u8; 4] = [0xff, 0xff, 0xff, 255];
const BLUSH: [u8; 4] = [0xf0, 0x90, 0x80, 255];
const CLOUD: [u8; 4] = [0xc8, 0xd0, 0xd8, 255];
const CLOUD_DK: [u8; 4] = [0x8a, 0x94, 0x9e, 255];
const RAIN: [u8; 4] = [0x5c, 0xa8, 0xe0, 255];
const RED: [u8; 4] = [0xe0, 0x38, 0x30, 255];
const GOLD: [u8; 4] = [0xff, 0xd0, 0x40, 255];
const MINT: [u8; 4] = [0x70, 0xd8, 0x88, 255];
const SKY: [u8; 4] = [0x60, 0xb0, 0xf0, 255];
const PINK: [u8; 4] = [0xf0, 0x70, 0xb0, 255];

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

    fn fill_disk(&mut self, cx: i32, cy: i32, r: i32, c: [u8; 4]) {
        for y in cy - r..=cy + r {
            for x in cx - r..=cx + r {
                if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r {
                    self.dot(x, y, c);
                }
            }
        }
    }

    fn outline_since(&mut self, before: &[u8]) {
        let snap = self.px.clone();
        let w = CELL as i32;
        for y in 0..w {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                if snap[i + 3] == 0 || before[i + 3] != 0 {
                    continue;
                }
                let edge = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .into_iter()
                    .any(|(dx, dy)| {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx < 0 || ny < 0 || nx >= w || ny >= w {
                            return true;
                        }
                        snap[((ny * w + nx) * 4 + 3) as usize] == 0
                    });
                if edge {
                    self.px[i..i + 4].copy_from_slice(&INK);
                }
            }
        }
    }

    fn scaled(&self) -> Vec<u8> {
        let n = CELL * SCALE;
        let mut out = vec![0u8; (n * n * 4) as usize];
        for y in 0..CELL {
            for x in 0..CELL {
                let src = ((y * CELL + x) * 4) as usize;
                for sy in 0..SCALE {
                    for sx in 0..SCALE {
                        let at = (((y * SCALE + sy) * n + x * SCALE + sx) * 4) as usize;
                        out[at..at + 4].copy_from_slice(&self.px[src..src + 4]);
                    }
                }
            }
        }
        out
    }
}

#[derive(Clone, Copy)]
enum Eye {
    Open,
    Shut,
    Spark,
    Droop,
}

#[derive(Clone, Copy)]
enum Mouth {
    Smile,
    Flat,
    Frown,
    Teeth,
    Open,
    O,
}

#[derive(Clone, Copy)]
enum Brow {
    None,
    Angry,
    Sad,
    Up,
}

#[derive(Clone, Copy)]
enum Arm {
    Down,
    Out,
    Up,
    High,
}

struct Pose {
    /// Feet baseline. Higher on the canvas is a smaller y.
    foot: i32,
    squash: i32,
    tilt: i32,
    shake: i32,
    eye: Eye,
    mouth: Mouth,
    brow: Brow,
    arm_l: Arm,
    arm_r: Arm,
    lift_l: i32,
    lift_r: i32,
    tears: i32,
    cloud: bool,
    rain: i32,
    scribble: bool,
    confetti: i32,
    star: bool,
    question: bool,
    zzz: i32,
    blush: bool,
}

/// One frame, already scaled ×2. Animations off holds the resting pose.
pub fn frame(mood: Mood, tick: u64, animate: bool) -> Vec<u8> {
    let tick = if animate { tick % u64::from(FRAMES) } else { 0 };
    let mut pix = Pix::new();
    paint(&mut pix, pose(mood, tick as i32));
    pix.scaled()
}

fn pose(mood: Mood, i: i32) -> Pose {
    let mut p = Pose {
        foot: 56,
        squash: 0,
        tilt: 0,
        shake: 0,
        eye: Eye::Open,
        mouth: Mouth::Flat,
        brow: Brow::None,
        arm_l: Arm::Down,
        arm_r: Arm::Down,
        lift_l: 0,
        lift_r: 0,
        tears: 0,
        cloud: false,
        rain: 0,
        scribble: false,
        confetti: 0,
        star: false,
        question: false,
        zzz: 0,
        blush: false,
    };
    match mood {
        Mood::Neutral => {
            p.squash = [0, 1, 2, 1, 0, 0, -1, 0][i as usize];
            p.foot += [0, 1, 2, 1, 0, 0, -1, 0][i as usize];
            if i == 4 || i == 5 {
                p.eye = Eye::Shut;
            }
        }
        Mood::Happy => {
            p.foot += [0, -3, -6, -3, 0, -2, 0, -4][i as usize];
            p.squash = [0, 1, -1, 1, 0, 0, 1, -1][i as usize];
            p.mouth = Mouth::Smile;
            p.blush = true;
            p.arm_r = [
                Arm::Down,
                Arm::Out,
                Arm::Up,
                Arm::High,
                Arm::Up,
                Arm::Out,
                Arm::High,
                Arm::Up,
            ][i as usize];
            if i == 5 {
                p.eye = Eye::Shut;
            }
        }
        Mood::Sad => {
            p.foot += 3;
            p.squash = 1;
            p.mouth = Mouth::Frown;
            p.brow = Brow::Sad;
            p.eye = Eye::Droop;
            p.tears = i;
            p.cloud = true;
            p.rain = i;
            p.arm_l = Arm::Down;
            p.arm_r = Arm::Down;
        }
        Mood::Frustrated => {
            p.shake = [-2, 2, -3, 3, -2, 2, -3, 2][i as usize];
            p.mouth = Mouth::Teeth;
            p.brow = Brow::Angry;
            p.scribble = true;
            p.lift_r = [0, -8, 0, -8, -2, 0, -8, 0][i as usize];
            p.squash = if i % 2 == 0 { 1 } else { 0 };
        }
        Mood::Excited => {
            p.foot += [0, -6, -12, -14, -10, -4, 0, -2][i as usize];
            p.squash = [1, -1, -2, -2, -1, 1, 2, 0][i as usize];
            p.mouth = Mouth::Open;
            p.eye = Eye::Spark;
            p.blush = true;
            p.arm_l = Arm::Out;
            p.arm_r = Arm::Up;
            p.confetti = i + 1;
        }
        Mood::Sleepy => {
            p.foot += 2;
            p.eye = if (2..6).contains(&i) {
                Eye::Shut
            } else {
                Eye::Droop
            };
            p.mouth = if (3..6).contains(&i) {
                Mouth::Open
            } else {
                Mouth::Flat
            };
            p.brow = Brow::Sad;
            p.zzz = i;
        }
        Mood::Celebrating => {
            p.foot += [0, -4, -8, -10, -6, -2, -8, -2][i as usize];
            p.mouth = Mouth::Open;
            p.eye = Eye::Spark;
            p.blush = true;
            p.arm_l = Arm::High;
            p.arm_r = Arm::High;
            p.confetti = i + 1;
            p.star = true;
        }
        Mood::Curious => {
            p.tilt = [3, 4, 5, 4, 3, 4, 5, 3][i as usize];
            p.mouth = Mouth::O;
            p.brow = Brow::Up;
            p.question = true;
            p.eye = Eye::Open;
        }
    }
    p
}

fn paint(p: &mut Pix, pose: Pose) {
    let w = 30 + pose.squash;
    let h = 34 - pose.squash;
    let foot = pose.foot;
    let top = foot - 6 - h;
    let x = 18 - pose.squash + pose.shake;
    shell(p, x, top, w, h, pose.tilt);
    let label_x = x + 4;
    let label_y = top + 8;
    let label_w = w - 8;
    let label_h = h - 14;
    label(p, label_x, label_y, label_w, label_h);
    face(p, label_x, label_y, label_w, label_h, &pose);
    let shoulder_y = top + h / 2;
    arm(p, x + 1, shoulder_y, pose.arm_l, true);
    arm(p, x + w - 2, shoulder_y, pose.arm_r, false);
    shoe(p, x + 4, foot - 4 + pose.lift_l, 10);
    shoe(p, x + w - 14, foot - 4 + pose.lift_r, 10);
    if pose.cloud {
        rain_cloud(p, pose.rain);
    }
    if pose.scribble {
        scribble(p, pose.shake);
    }
    if pose.confetti > 0 {
        confetti(p, pose.confetti);
    }
    if pose.star {
        star(p, 30 + pose.shake, top - 10);
    }
    if pose.question {
        question(p, x + w - 2, top - 8);
    }
    if pose.zzz > 0 || matches!((pose.eye, pose.mouth), (Eye::Droop, Mouth::Open)) {
        zeds(p, x + w - 4, top - 2 - pose.zzz * 2);
    }
}

fn shell(p: &mut Pix, x: i32, y: i32, w: i32, h: i32, tilt: i32) {
    let before = p.px.clone();
    // Tab on top, the bit a real cart's label sticks up.
    let tab_w = w / 2;
    let tab_x = x + (w - tab_w) / 2 + tilt;
    round_fill(p, tab_x, y - 6, tab_w, 8, 1);
    p.dot(tab_x + 3, y - 4, DARK);
    p.dot(tab_x + tab_w - 4, y - 4, DARK);
    for ry in 0..h {
        let shift = if h <= 1 {
            0
        } else {
            tilt * (h - 1 - ry) / (h - 1)
        };
        let inset = if ry == 0 || ry == h - 1 {
            2
        } else if ry == 1 || ry == h - 2 {
            1
        } else {
            0
        };
        for rx in inset..w - inset {
            let shade = if rx < inset + 3 {
                DARK
            } else if rx >= w - inset - 3 {
                LIGHT
            } else {
                ORANGE
            };
            p.dot(x + rx + shift, y + ry, shade);
        }
    }
    p.outline_since(&before);
}

fn round_fill(p: &mut Pix, x: i32, y: i32, w: i32, h: i32, inset_cap: i32) {
    for ry in 0..h {
        let inset = if ry == 0 || ry == h - 1 {
            inset_cap + 1
        } else {
            0
        };
        for rx in inset..w - inset {
            let c = if rx < 2 {
                DARK
            } else if rx >= w - 2 {
                LIGHT
            } else {
                ORANGE
            };
            p.dot(x + rx, y + ry, c);
        }
    }
}

fn label(p: &mut Pix, x: i32, y: i32, w: i32, h: i32) {
    let before = p.px.clone();
    for ry in 1..h - 1 {
        for rx in 1..w - 1 {
            let c = if ry > h - 4 { CREAM_DK } else { CREAM };
            p.dot(x + rx, y + ry, c);
        }
    }
    p.outline_since(&before);
}

fn face(p: &mut Pix, x: i32, y: i32, w: i32, _h: i32, pose: &Pose) {
    let eye_w = 9;
    let gap = 2;
    let pair = eye_w * 2 + gap;
    let ex = x + (w - pair) / 2;
    let ey = y + 2;
    eye(p, ex, ey, pose.eye, true, pose.brow);
    eye(p, ex + eye_w + gap, ey, pose.eye, false, pose.brow);
    if pose.blush {
        p.fill_disk(ex - 1, ey + 7, 1, BLUSH);
        p.fill_disk(ex + pair + 1, ey + 7, 1, BLUSH);
    }
    mouth(p, x + w / 2 - 5, ey + 10, pose.mouth);
    if pose.tears > 0 {
        let drop = (pose.tears % 5) + 1;
        tear(p, ex + 3, ey + 8 + drop);
        tear(p, ex + eye_w + gap + 4, ey + 6 + drop);
    }
}

fn eye(p: &mut Pix, x: i32, y: i32, kind: Eye, left: bool, brow: Brow) {
    let _ = left;
    for ry in 0..8 {
        for rx in 0..9 {
            let edge =
                ry == 0 || ry == 7 || rx == 0 || rx == 8 || (ry == 1 && (rx == 1 || rx == 7));
            let c = if edge { INK } else { WHITE };
            p.dot(x + rx, y + ry, c);
        }
    }
    // Soften the four outer corners so the eye is round.
    for (cx, cy) in [
        (0, 0),
        (8, 0),
        (0, 7),
        (8, 7),
        (1, 0),
        (7, 0),
        (0, 1),
        (8, 1),
    ] {
        p.dot(x + cx, y + cy, CREAM);
    }
    p.dot(x + 1, y + 1, INK);
    p.dot(x + 7, y + 1, INK);
    p.dot(x + 1, y + 6, INK);
    p.dot(x + 7, y + 6, INK);
    match kind {
        Eye::Shut => {
            for ry in 1..7 {
                for rx in 1..8 {
                    p.dot(x + rx, y + ry, CREAM);
                }
            }
            for rx in 1..8 {
                p.dot(x + rx, y + 4, INK);
            }
        }
        Eye::Droop => {
            for ry in 1..4 {
                for rx in 1..8 {
                    p.dot(x + rx, y + ry, INK);
                }
            }
            p.dot(x + 2, y + 5, INK);
            p.dot(x + 3, y + 5, INK);
        }
        Eye::Spark => {
            for (dx, dy) in [
                (3, 2),
                (4, 2),
                (5, 2),
                (4, 1),
                (4, 3),
                (2, 3),
                (6, 3),
                (3, 4),
                (5, 4),
            ] {
                p.dot(x + dx, y + dy, GOLD);
            }
            p.dot(x + 3, y + 2, WHITE);
        }
        Eye::Open => {
            for ry in 3..6 {
                for rx in 3..6 {
                    p.dot(x + rx, y + ry, INK);
                }
            }
            p.dot(x + 2, y + 2, WHITE);
            p.dot(x + 3, y + 2, WHITE);
            p.dot(x + 2, y + 3, WHITE);
        }
    }
    brow_of(p, x, y - 2, brow, left);
}

fn brow_of(p: &mut Pix, x: i32, y: i32, brow: Brow, left: bool) {
    match brow {
        Brow::Angry => {
            for i in 0..7 {
                let dy = if left { i / 2 } else { 3 - i / 2 };
                p.dot(x + 1 + i, y + dy, INK);
            }
        }
        Brow::Sad => {
            for i in 0..7 {
                let dy = if left { 2 - i / 3 } else { i / 3 };
                p.dot(x + 1 + i, y + dy, INK);
            }
        }
        Brow::Up if left => {
            for i in 0..7 {
                p.dot(x + 1 + i, y - 1 + i / 4, INK);
            }
        }
        Brow::Up => {
            for i in 0..6 {
                p.dot(x + 2 + i, y + 1, INK);
            }
        }
        Brow::None => {}
    }
}

fn mouth(p: &mut Pix, x: i32, y: i32, kind: Mouth) {
    match kind {
        Mouth::Smile => {
            for i in 0..10 {
                let dy = if i == 0 || i == 9 {
                    0
                } else if i < 3 || i > 6 {
                    1
                } else {
                    2
                };
                p.dot(x + i, y + dy, INK);
                if (2..8).contains(&i) {
                    p.dot(x + i, y + dy - 1, INK);
                }
            }
        }
        Mouth::Frown => {
            for i in 0..10 {
                let dy = if i == 0 || i == 9 {
                    2
                } else if i < 3 || i > 6 {
                    1
                } else {
                    0
                };
                p.dot(x + i, y + dy, INK);
            }
        }
        Mouth::Teeth => {
            for i in 0..10 {
                p.dot(x + i, y, INK);
                p.dot(x + i, y + 3, INK);
                if i % 2 == 0 {
                    p.dot(x + i, y + 1, WHITE);
                    p.dot(x + i, y + 2, WHITE);
                } else {
                    p.dot(x + i, y + 1, INK);
                    p.dot(x + i, y + 2, INK);
                }
            }
        }
        Mouth::Open => {
            for ry in 0..6 {
                for rx in 0..10 {
                    let edge = ry == 0 || ry == 5 || rx == 0 || rx == 9;
                    p.dot(x + rx, y + ry, if edge { INK } else { CREAM_DK });
                }
            }
            p.dot(x, y, CREAM);
            p.dot(x + 9, y, CREAM);
            p.dot(x, y + 5, CREAM);
            p.dot(x + 9, y + 5, CREAM);
        }
        Mouth::O => {
            for ry in 0..5 {
                for rx in 2..8 {
                    let edge = ry == 0 || ry == 4 || rx == 2 || rx == 7;
                    p.dot(x + rx, y + ry, if edge { INK } else { CREAM_DK });
                }
            }
        }
        Mouth::Flat => {
            for i in 1..9 {
                p.dot(x + i, y + 1, INK);
            }
        }
    }
}

fn tear(p: &mut Pix, x: i32, y: i32) {
    p.dot(x, y, RAIN);
    p.dot(x, y + 1, RAIN);
    p.dot(x, y + 2, SKY);
    p.dot(x + 1, y + 1, RAIN);
}

fn arm(p: &mut Pix, shoulder_x: i32, shoulder_y: i32, pose: Arm, left: bool) {
    let dir = if left { -1 } else { 1 };
    let (dx, dy) = match pose {
        Arm::Down => (dir * 2, 14),
        Arm::Out => (dir * 12, 1),
        Arm::Up => (dir * 3, -16),
        Arm::High => (dir * 8, -18),
    };
    let before = p.px.clone();
    limb(p, shoulder_x, shoulder_y, shoulder_x + dx, shoulder_y + dy);
    // Hand.
    p.fill_disk(shoulder_x + dx, shoulder_y + dy, 2, SHOE);
    p.outline_since(&before);
}

fn limb(p: &mut Pix, x0: i32, y0: i32, x1: i32, y1: i32) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    for i in 0..=steps {
        let x = x0 + (x1 - x0) * i / steps;
        let y = y0 + (y1 - y0) * i / steps;
        p.fill_disk(x, y, 3, ORANGE);
    }
}

fn shoe(p: &mut Pix, x: i32, y: i32, w: i32) {
    let before = p.px.clone();
    for ry in 0..7 {
        for rx in 0..w {
            p.dot(x + rx, y + ry, SHOE);
        }
    }
    p.outline_since(&before);
    // Put the cream back inside the outline, with a dark sole and an orange lace.
    for ry in 1..4 {
        for rx in 1..w - 1 {
            p.dot(x + rx, y + ry, SHOE);
        }
    }
    for rx in 1..w - 1 {
        p.dot(x + rx, y + 2, ORANGE);
        p.dot(x + rx, y + 5, INK);
        p.dot(x + rx, y + 6, INK);
    }
}

fn rain_cloud(p: &mut Pix, phase: i32) {
    let before = p.px.clone();
    blob(p, 40, 2, 16, 8, CLOUD, CLOUD_DK);
    p.outline_since(&before);
    for (n, x) in [42, 47, 52].into_iter().enumerate() {
        let y = 10 + ((phase + n as i32 * 2) % 6) * 3;
        p.dot(x, y, RAIN);
        p.dot(x, y + 1, RAIN);
        p.dot(x, y + 2, SKY);
    }
}

fn blob(p: &mut Pix, x: i32, y: i32, w: i32, h: i32, fill: [u8; 4], shade: [u8; 4]) {
    for ry in 0..h {
        let inset = if ry == 0 || ry == h - 1 { 2 } else { 0 };
        for rx in inset..w - inset {
            p.dot(x + rx, y + ry, if ry > h - 3 { shade } else { fill });
        }
    }
}

fn scribble(p: &mut Pix, shake: i32) {
    let y = 4;
    for i in 0..12 {
        let jag = if i % 2 == 0 { 0 } else { 3 };
        p.dot(36 + shake.signum() + i, y + jag, RED);
        p.dot(36 + i, y + 6 - jag, RED);
    }
    p.dot(40, y + 3, RED);
    p.dot(44, y + 2, RED);
    p.dot(42, y + 5, RED);
}

fn confetti(p: &mut Pix, phase: i32) {
    let bits = [
        (4, 18, GOLD),
        (8, 28, MINT),
        (6, 40, PINK),
        (58, 16, SKY),
        (54, 30, GOLD),
        (60, 42, RED),
        (14, 10, SKY),
        (50, 8, PINK),
        (10, 50, MINT),
        (52, 52, GOLD),
    ];
    for (n, (x, y, c)) in bits.into_iter().enumerate() {
        let spread = phase + n as i32;
        let ox = if n % 2 == 0 { -spread } else { spread };
        let oy = (spread * 2) % 7;
        p.dot(x + ox / 2, y + oy, c);
        p.dot(x + ox / 2 + 1, y + oy, c);
    }
}

fn star(p: &mut Pix, x: i32, y: i32) {
    for (dx, dy) in [
        (0, 2),
        (1, 2),
        (2, 2),
        (3, 2),
        (4, 2),
        (2, 0),
        (2, 1),
        (2, 3),
        (2, 4),
        (1, 1),
        (3, 1),
        (1, 3),
        (3, 3),
    ] {
        p.dot(x + dx, y + dy, GOLD);
    }
    p.dot(x + 2, y + 2, WHITE);
}

fn question(p: &mut Pix, x: i32, y: i32) {
    // A small cream bubble and a dark question mark.
    let before = p.px.clone();
    blob(p, x, y, 10, 10, CREAM, CREAM_DK);
    p.outline_since(&before);
    for (dx, dy) in [(3, 2), (4, 2), (5, 2), (6, 3), (5, 4), (4, 5), (4, 7)] {
        p.dot(x + dx, y + dy, INK);
    }
}

fn zeds(p: &mut Pix, x: i32, y: i32) {
    zee(p, x, y, 5);
    zee(p, x + 6, y - 6, 3);
}

fn zee(p: &mut Pix, x: i32, y: i32, w: i32) {
    for i in 0..w {
        p.dot(x + i, y, INK);
        p.dot(x + w - 1 - i, y + i, INK);
        p.dot(x + i, y + w - 1, INK);
    }
}

/// Eight frames side by side, ×2, with a 2px gap.
#[cfg(test)]
pub fn strip(mood: Mood) -> (u32, u32, Vec<u8>) {
    let gap = 2u32;
    let fw = CELL * SCALE;
    let fh = fw;
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

/// Looping GIF, one indexed frame per pose, ×2 so the pixels stay square.
#[cfg(test)]
pub fn write_gif(path: &std::path::Path, mood: Mood) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let palette = [
        [0, 0, 0],
        [ORANGE[0], ORANGE[1], ORANGE[2]],
        [DARK[0], DARK[1], DARK[2]],
        [LIGHT[0], LIGHT[1], LIGHT[2]],
        [CREAM[0], CREAM[1], CREAM[2]],
        [CREAM_DK[0], CREAM_DK[1], CREAM_DK[2]],
        [SHOE[0], SHOE[1], SHOE[2]],
        [INK[0], INK[1], INK[2]],
        [WHITE[0], WHITE[1], WHITE[2]],
        [BLUSH[0], BLUSH[1], BLUSH[2]],
        [CLOUD[0], CLOUD[1], CLOUD[2]],
        [CLOUD_DK[0], CLOUD_DK[1], CLOUD_DK[2]],
        [RAIN[0], RAIN[1], RAIN[2]],
        [RED[0], RED[1], RED[2]],
        [GOLD[0], GOLD[1], GOLD[2]],
        [MINT[0], MINT[1], MINT[2]],
        [SKY[0], SKY[1], SKY[2]],
        [PINK[0], PINK[1], PINK[2]],
    ];
    let mut frames = Vec::new();
    for i in 0..FRAMES {
        let rgba = frame(mood, u64::from(i), true);
        let mut idx = Vec::with_capacity(rgba.len() / 4);
        for px in rgba.chunks(4) {
            if px[3] == 0 {
                idx.push(0);
                continue;
            }
            let (best, _) = palette
                .iter()
                .enumerate()
                .min_by_key(|(_, c)| {
                    let dr = px[0] as i32 - c[0] as i32;
                    let dg = px[1] as i32 - c[1] as i32;
                    let db = px[2] as i32 - c[2] as i32;
                    dr * dr + dg * dg + db * db
                })
                .unwrap();
            idx.push(best as u8);
        }
        frames.push(idx);
    }
    let w = (CELL * SCALE) as u16;
    let h = w;
    std::fs::write(path, gif_bytes(w, h, &palette, &frames, 12))
}

#[cfg(test)]
fn gif_bytes(w: u16, h: u16, palette: &[[u8; 3]], frames: &[Vec<u8>], delay: u16) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"GIF89a");
    out.extend_from_slice(&w.to_le_bytes());
    out.extend_from_slice(&h.to_le_bytes());
    // 256-color global table, so the minimum code size stays 8.
    out.push(0xF7);
    out.push(0);
    out.push(0);
    for c in palette {
        out.extend_from_slice(c);
    }
    for _ in palette.len()..256 {
        out.extend_from_slice(&[0, 0, 0]);
    }
    // Loop forever.
    out.extend_from_slice(&[0x21, 0xFF, 11]);
    out.extend_from_slice(b"NETSCAPE2.0");
    out.extend_from_slice(&[3, 1, 0, 0, 0]);
    for frame in frames {
        out.extend_from_slice(&[0x21, 0xF9, 4, 0x09]);
        out.extend_from_slice(&delay.to_le_bytes());
        out.push(0);
        out.push(0);
        out.push(0x2C);
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&w.to_le_bytes());
        out.extend_from_slice(&h.to_le_bytes());
        out.push(0);
        out.push(8);
        let lzw = lzw_encode(frame, 8);
        for chunk in lzw.chunks(255) {
            out.push(chunk.len() as u8);
            out.extend_from_slice(chunk);
        }
        out.push(0);
    }
    out.push(0x3B);
    out
}

#[cfg(test)]
fn lzw_encode(indices: &[u8], min_code_size: u8) -> Vec<u8> {
    let clear = 1u16 << min_code_size;
    let eoi = clear + 1;
    let mut next = eoi + 1;
    let mut code_size = min_code_size as u32 + 1;
    let mut dict: std::collections::HashMap<(u16, u8), u16> = std::collections::HashMap::new();
    let mut bits: u64 = 0;
    let mut nbits: u32 = 0;
    let mut bytes = Vec::new();
    let mut push =
        |code: u16, code_size: u32, bits: &mut u64, nbits: &mut u32, bytes: &mut Vec<u8>| {
            *bits |= u64::from(code) << *nbits;
            *nbits += code_size;
            while *nbits >= 8 {
                bytes.push(*bits as u8);
                *bits >>= 8;
                *nbits -= 8;
            }
        };
    push(clear, code_size, &mut bits, &mut nbits, &mut bytes);
    if indices.is_empty() {
        push(eoi, code_size, &mut bits, &mut nbits, &mut bytes);
        if nbits > 0 {
            bytes.push(bits as u8);
        }
        return bytes;
    }
    let mut prefix = u16::from(indices[0]);
    for &b in &indices[1..] {
        if let Some(&code) = dict.get(&(prefix, b)) {
            prefix = code;
            continue;
        }
        push(prefix, code_size, &mut bits, &mut nbits, &mut bytes);
        if next < 4096 {
            dict.insert((prefix, b), next);
            if next == (1 << code_size) && code_size < 12 {
                code_size += 1;
            }
            next += 1;
        } else {
            push(clear, code_size, &mut bits, &mut nbits, &mut bytes);
            dict.clear();
            next = eoi + 1;
            code_size = min_code_size as u32 + 1;
        }
        prefix = u16::from(b);
    }
    push(prefix, code_size, &mut bits, &mut nbits, &mut bytes);
    push(eoi, code_size, &mut bits, &mut nbits, &mut bytes);
    if nbits > 0 {
        bytes.push(bits as u8);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(rgba: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * w + x) * 4) as usize;
        rgba[i..i + 4].try_into().unwrap()
    }

    fn changed(a: &[u8], b: &[u8]) -> usize {
        a.chunks(4).zip(b.chunks(4)).filter(|(p, q)| p != q).count()
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
            assert!(px.chunks(4).any(|p| p == ORANGE), "{mood:?} has no shell");
            assert!(px.chunks(4).any(|p| p == CREAM), "{mood:?} has no label");
            assert!(px.chunks(4).any(|p| p == INK), "{mood:?} has no outline");
            assert!(
                px.chunks(4).any(|p| p == WHITE),
                "{mood:?} eyes have no highlight"
            );
            assert_eq!(at(&px, n, 0, 0)[3], 0, "the corner stays clear");
        }
    }

    #[test]
    fn the_poses_actually_move() {
        let idle_a = frame(Mood::Neutral, 0, true);
        let idle_b = frame(Mood::Neutral, 2, true);
        assert!(
            changed(&idle_a, &idle_b) > 80,
            "idle squash should be obvious"
        );
        assert_ne!(
            frame(Mood::Neutral, 0, true),
            frame(Mood::Neutral, 4, true),
            "a blink"
        );
        assert_eq!(frame(Mood::Happy, 0, false), frame(Mood::Happy, 4, false));
        assert!(
            changed(&frame(Mood::Happy, 0, true), &frame(Mood::Happy, 3, true)) > 200,
            "the wave should move the whole arm"
        );
        assert!(
            changed(
                &frame(Mood::Excited, 0, true),
                &frame(Mood::Excited, 3, true)
            ) > 200
        );
        let (w, h, sad) = strip(Mood::Sad);
        assert_eq!(
            (w, h),
            (FRAMES * CELL * SCALE + (FRAMES - 1) * 2, CELL * SCALE)
        );
        assert!(sad.chunks(4).any(|p| p == RAIN));
        assert!(sad.chunks(4).any(|p| p == CLOUD));
        assert!(strip(Mood::Frustrated).2.chunks(4).any(|p| p == RED));
        assert!(strip(Mood::Celebrating).2.chunks(4).any(|p| p == GOLD));
        assert!(strip(Mood::Curious).2.chunks(4).any(|p| p == INK));
    }

    #[test]
    fn preview_strips_and_gifs() {
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
            write_gif(&dir.join(format!("{name}.gif")), mood).unwrap();
        }
    }
}
