//! Reads `assets/carts/<sys>/meta.toml` and the shell PNG's size, and emits
//! `shells.rs` for the compositor. Dropping in a new render is a new directory
//! plus a rebuild: shell.png, optional mask.png and shade.png, meta.toml.
use std::fs;
use std::path::Path;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let carts = manifest.join("../../assets/carts");
    println!("cargo:rerun-if-changed={}", carts.display());
    let mut dirs: Vec<_> = fs::read_dir(&carts)
        .unwrap_or_else(|e| panic!("assets/carts: {e}"))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.join("shell.png").is_file() && p.join("meta.toml").is_file())
        .collect();
    dirs.sort();
    let mut out = String::from(
        "pub struct ShellPng {\n    \
         pub dir: &'static str,\n    \
         pub w: u32,\n    \
         pub h: u32,\n    \
         pub shell: &'static [u8],\n    \
         pub mask: Option<&'static [u8]>,\n    \
         pub shade: Option<&'static [u8]>,\n    \
         pub tint: Option<&'static [u8]>,\n    \
         pub backlit: bool,\n    \
         pub x: u32,\n    \
         pub y: u32,\n    \
         pub lw: u32,\n    \
         pub lh: u32,\n    \
         pub rotation: f32,\n    \
         pub quad: Option<[[f32; 2]; 4]>,\n    \
         pub aspect: f32,\n}\n",
    );
    let mut arms = String::new();
    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy();
        let ident = name.replace('-', "_").to_ascii_uppercase();
        let meta = fs::read_to_string(dir.join("meta.toml")).unwrap();
        let (x, y, lw, lh, rotation, aspect, quad) = parse_meta(&meta, &name);
        let backlit = meta_flag(&meta, "backlit");
        let (w, h) = png_size(&fs::read(dir.join("shell.png")).unwrap(), &name);
        let shell = dir.join("shell.png");
        let mask = dir.join("mask.png");
        let mask_expr = if mask.is_file() {
            format!("Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/carts/{name}/mask.png\")))")
        } else {
            "None".to_string()
        };
        let shade = dir.join("shade.png");
        let shade_expr = if shade.is_file() {
            format!("Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/carts/{name}/shade.png\")))")
        } else {
            "None".to_string()
        };
        // v3 models: the plastic's light and cover, recoloured per game (see model.rs)
        let tint = dir.join("tint.png");
        let tint_expr = if tint.is_file() {
            println!("cargo:rerun-if-changed={}", tint.display());
            format!("Some(include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/carts/{name}/tint.png\")))")
        } else {
            "None".to_string()
        };
        let quad_expr = match quad {
            Some(q) => format!("Some([{q}])"),
            None => "None".to_string(),
        };
        out.push_str(&format!(
            "pub static {ident}: ShellPng = ShellPng {{\n    \
             dir: \"{name}\",\n    \
             w: {w},\n    \
             h: {h},\n    \
             shell: include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/carts/{name}/shell.png\")),\n    \
             mask: {mask_expr},\n    \
             shade: {shade_expr},\n    \
             tint: {tint_expr},\n    \
             backlit: {backlit},\n    \
             x: {x},\n    \
             y: {y},\n    \
             lw: {lw},\n    \
             lh: {lh},\n    \
             rotation: {rotation}f32,\n    \
             quad: {quad_expr},\n    \
             aspect: {aspect}f32,\n}};\n"
        ));
        let _ = shell;
        arms.push_str(&format!("\"{name}\" => Some(&{ident}),\n"));
        println!("cargo:rerun-if-changed={}", dir.join("shell.png").display());
        println!("cargo:rerun-if-changed={}", dir.join("meta.toml").display());
        if mask.is_file() {
            println!("cargo:rerun-if-changed={}", mask.display());
        }
        if shade.is_file() {
            println!("cargo:rerun-if-changed={}", shade.display());
        }
    }
    out.push_str(&format!(
        "pub fn by_dir(dir: &str) -> Option<&'static ShellPng> {{\n    match dir {{\n        {arms}        _ => None,\n    }}\n}}\n"
    ));
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("shells.rs");
    fs::write(dest, out).unwrap();
    turns(&dirs);
}

/// `assets/carts/<sys>/insert/`: the frames a cart turns through on its way into the slot
/// (see `docs/cart-shells.md`). Each frame is the shell with the label window clear
/// (`NN.png`), the light and cover of the label on it (`NN-map.png`, grey + alpha), and
/// `uv`: the projective map from a frame pixel to the flat label's u, v.
fn turns(dirs: &[std::path::PathBuf]) {
    let mut out = String::from(
        "pub struct TurnFrame {\n    pub x: i32,\n    pub y: i32,\n    pub w: u32,\n    pub h: u32,\n    \
         pub uv: [f32; 9],\n    pub shell: &'static [u8],\n    pub map: &'static [u8],\n    pub tint: Option<&'static [u8]>,\n}\n\
         pub struct Turn {\n    pub dir: &'static str,\n    pub fps: f32,\n    pub flip: bool,\n    \
         pub hold: f32,\n    pub frames: &'static [TurnFrame],\n}\n",
    );
    let mut arms = String::new();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let toml = dir.join("insert/insert.toml");
        println!("cargo:rerun-if-changed={}", dir.join("insert").display());
        let Ok(text) = fs::read_to_string(&toml) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", toml.display());
        let ident = format!("{}_TURN", name.replace('-', "_").to_ascii_uppercase());
        let mut fps = 30.0f32;
        let mut hold = 0.0f32;
        let flip = meta_flag(&text, "flip");
        let mut frames = String::new();
        let mut n = 0;
        let mut uv: Option<String> = None;
        for line in text.lines() {
            let line = line.split('#').next().unwrap().trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "fps" => {
                    fps = value
                        .parse()
                        .unwrap_or_else(|_| panic!("{name}: insert fps"))
                }
                "hold" => {
                    hold = value
                        .parse()
                        .unwrap_or_else(|_| panic!("{name}: insert hold"))
                }
                "uv" => {
                    let v: Vec<f64> = value
                        .split(',')
                        .map(|n| {
                            n.trim()
                                .parse()
                                .unwrap_or_else(|_| panic!("{name}: insert uv"))
                        })
                        .collect();
                    assert!(v.len() == 9, "{name}: insert uv needs 9 numbers");
                    uv = Some(
                        v.iter()
                            .map(|n| format!("{n:?}f32"))
                            .collect::<Vec<_>>()
                            .join(", "),
                    );
                }
                "frame" => {
                    let v: Vec<i64> = value
                        .split(',')
                        .map(|n| {
                            n.trim()
                                .parse()
                                .unwrap_or_else(|_| panic!("{name}: insert frame"))
                        })
                        .collect();
                    assert!(v.len() == 4, "{name}: insert frame needs x, y, w, h");
                    for part in ["", "-map"] {
                        let f = dir.join(format!("insert/{n:02}{part}.png"));
                        assert!(f.is_file(), "{name}: missing {}", f.display());
                        println!("cargo:rerun-if-changed={}", f.display());
                    }
                    let inc = |part: &str| {
                        format!("include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/carts/{name}/insert/{n:02}{part}.png\"))")
                    };
                    let tint = dir.join(format!("insert/{n:02}-tint.png"));
                    let tint_expr = if tint.is_file() {
                        println!("cargo:rerun-if-changed={}", tint.display());
                        format!("Some({})", inc("-tint"))
                    } else {
                        "None".to_string()
                    };
                    frames.push_str(&format!(
                        "    TurnFrame {{ x: {}, y: {}, w: {}, h: {}, uv: [{}], shell: {}, map: {}, tint: {} }},\n",
                        v[0], v[1], v[2], v[3],
                        uv.take().unwrap_or_else(|| panic!("{name}: frame {n} needs a uv line before it")),
                        inc(""), inc("-map"), tint_expr
                    ));
                    n += 1;
                }
                _ => {}
            }
        }
        assert!(n > 0, "{name}: insert.toml lists no frames");
        out.push_str(&format!(
            "pub static {ident}: Turn = Turn {{\n    dir: \"{name}\",\n    fps: {fps}f32,\n    flip: {flip},\n    hold: {hold}f32,\n    frames: &[\n{frames}    ],\n}};\n"
        ));
        arms.push_str(&format!("\"{name}\" => Some(&{ident}),\n"));
    }
    out.push_str(&format!(
        "pub fn turn_by_dir(dir: &str) -> Option<&'static Turn> {{\n    match dir {{\n        {arms}        _ => None,\n    }}\n}}\n"
    ));
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("turns.rs");
    fs::write(dest, out).unwrap();
}

fn parse_meta(text: &str, name: &str) -> (u32, u32, u32, u32, f32, f32, Option<String>) {
    let mut x = None;
    let mut y = None;
    let mut w = None;
    let mut h = None;
    let mut rotation = 0.0f32;
    let mut aspect = None;
    let mut quad = None;
    for line in text.lines() {
        let line = line.split('#').next().unwrap().trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "x" => x = value.parse().ok(),
            "y" => y = value.parse().ok(),
            "w" => w = value.parse().ok(),
            "h" => h = value.parse().ok(),
            "rotation" => rotation = value.parse().unwrap_or(0.0),
            "aspect" => aspect = value.parse().ok(),
            "quad" => {
                let nums: Vec<f32> = value
                    .trim_matches(|c| c == '[' || c == ']')
                    .split(',')
                    .filter_map(|n| n.trim().parse().ok())
                    .collect();
                if nums.len() == 8 {
                    let mut parts = Vec::new();
                    for i in 0..4 {
                        parts.push(format!("[{}, {}]", nums[i * 2], nums[i * 2 + 1]));
                    }
                    quad = Some(parts.join(", "));
                } else {
                    panic!("{name}: quad needs 8 numbers");
                }
            }
            _ => {}
        }
    }
    let (x, y, w, h) = (
        x.unwrap_or_else(|| panic!("{name}: meta.toml needs x")),
        y.unwrap_or_else(|| panic!("{name}: meta.toml needs y")),
        w.unwrap_or_else(|| panic!("{name}: meta.toml needs w")),
        h.unwrap_or_else(|| panic!("{name}: meta.toml needs h")),
    );
    let aspect = aspect.unwrap_or(w as f32 / h as f32);
    (x, y, w, h, rotation, aspect, quad)
}

/// `key = true` in meta.toml (anything else, or no key, is false).
fn meta_flag(text: &str, key: &str) -> bool {
    text.lines().any(|line| {
        let line = line.split('#').next().unwrap().trim();
        line.split_once('=')
            .is_some_and(|(k, v)| k.trim() == key && v.trim() == "true")
    })
}

fn png_size(bytes: &[u8], name: &str) -> (u32, u32) {
    if bytes.len() < 24 || &bytes[12..16] != b"IHDR" {
        panic!("{name}: shell.png is not a PNG");
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    (w, h)
}
