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
