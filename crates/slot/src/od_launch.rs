//! Launch an OpenDingux standalone or RetroArch core for the selected ROM.
//!
//! Used on the RG280V / Slotted build instead of loading a Slot-embedded
//! libretro core. The bundled table is `launchers.ini` next to the binary
//! (inside the OPK). A file at `Slot/Config/launchers.ini` on the card, if
//! present, overrides it. We never download BIOS; the emulator looks in its
//! own BIOS dir or in `Slot/BIOS/` if Jeff has placed files there.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use slot_store::Platform;

/// Places we look for `.opk` files on a typical Adam / OpenDingux install.
pub const OPK_SEARCH: &[&str] = &[
    "/media/data/apps",
    "/media/sdcard/apps",
    "/media/ext/apps",
    "/usr/local/share/OPKs",
    "/media/data/local/home/.apps",
];

/// Places we look for RetroArch cores (`*_libretro.so`).
pub const CORE_SEARCH: &[&str] = &[
    "/usr/lib/libretro",
    "/media/data/local/home/.retroarch/cores",
    "/usr/local/lib/libretro",
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Launcher {
    pub opk: Vec<String>,
    pub retroarch_core: Vec<String>,
    pub exec: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LaunchTable {
    pub search: Vec<String>,
    pub cores: Vec<String>,
    pub by_platform: HashMap<String, Launcher>,
}

impl LaunchTable {
    pub fn parse(text: &str) -> Self {
        let mut table = LaunchTable {
            search: OPK_SEARCH.iter().map(|s| (*s).to_string()).collect(),
            cores: CORE_SEARCH.iter().map(|s| (*s).to_string()).collect(),
            by_platform: HashMap::new(),
        };
        let mut section = String::new();
        for raw in text.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                section = name.trim().to_ascii_lowercase();
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            if section.is_empty() || section == "paths" {
                match key {
                    "search" => table.search = split_list(value),
                    "cores" => table.cores = split_list(value),
                    _ => {}
                }
                continue;
            }
            let entry = table.by_platform.entry(section.clone()).or_default();
            match key {
                "opk" => entry.opk = split_list(value),
                "retroarch_core" | "core" => entry.retroarch_core = split_list(value),
                "exec" => {
                    if !value.is_empty() {
                        entry.exec = Some(value.to_string());
                    }
                }
                _ => {}
            }
        }
        table
    }

    pub fn for_platform(&self, platform: Platform) -> Option<&Launcher> {
        self.by_platform.get(&section_name(platform))
    }

    /// Resolve a command that should start `rom` for `platform`.
    pub fn command(&self, platform: Platform, rom: &Path) -> Result<Command, String> {
        let launcher = self
            .for_platform(platform)
            .ok_or_else(|| format!("no launcher configured for {}", platform.name()))?;
        if let Some(tmpl) = &launcher.exec {
            return Ok(command_from_template(tmpl, rom, None, None));
        }
        for name in &launcher.opk {
            if let Some(opk) = find_named(&self.search, name) {
                let mut cmd = Command::new("opkrun");
                cmd.arg(&opk).arg(rom);
                return Ok(cmd);
            }
        }
        for name in &launcher.retroarch_core {
            if let Some(core) = find_named(&self.cores, name) {
                let mut cmd = Command::new("retroarch");
                cmd.arg("-L").arg(&core).arg(rom);
                return Ok(cmd);
            }
        }
        let tried_opk = launcher.opk.join(", ");
        let tried_core = launcher.retroarch_core.join(", ");
        Err(format!(
            "no emulator found for {} (looked for OPK [{}] in {}, cores [{}] in {})",
            platform.name(),
            tried_opk,
            self.search.join(":"),
            tried_core,
            self.cores.join(":")
        ))
    }
}

pub fn section_name(platform: Platform) -> String {
    platform.dir_name().to_ascii_lowercase()
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(&[',', ':'][..])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn find_named(dirs: &[String], name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() && path.is_file() {
        return Some(path.to_path_buf());
    }
    for dir in dirs {
        let p = Path::new(dir).join(name);
        if p.is_file() {
            return Some(p);
        }
        // Some images keep a slightly different capitalisation.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                if e.file_name().eq_ignore_ascii_case(name) && e.path().is_file() {
                    return Some(e.path());
                }
            }
        }
    }
    None
}

fn command_from_template(
    tmpl: &str,
    rom: &Path,
    opk: Option<&Path>,
    core: Option<&Path>,
) -> Command {
    let rom_s = rom.display().to_string();
    let filled = tmpl
        .replace("{rom}", &rom_s)
        .replace("{opk}", &opk.map(|p| p.display().to_string()).unwrap_or_default())
        .replace("{core}", &core.map(|p| p.display().to_string()).unwrap_or_default());
    let mut parts = shell_words(&filled);
    let prog = if parts.is_empty() {
        "opkrun".into()
    } else {
        parts.remove(0)
    };
    let mut cmd = Command::new(prog);
    cmd.args(parts);
    cmd
}

/// Split on whitespace; single and double quotes keep their contents.
fn shell_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut quote: Option<char> = None;
    for c in s.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => buf.push(c),
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c.is_whitespace() => {
                if !buf.is_empty() {
                    out.push(std::mem::take(&mut buf));
                }
            }
            None => buf.push(c),
        }
    }
    if !buf.is_empty() {
        out.push(buf);
    }
    out
}

/// Load the bundled table, then overlay the card's `Config/launchers.ini` if it exists.
pub fn load(root: &Path, bundled: Option<&str>) -> LaunchTable {
    let mut table = bundled
        .or(Some(include_str!("../../../rg280v/launchers.ini")))
        .map(LaunchTable::parse)
        .unwrap_or_default();
    let overlay = root.join("Config/launchers.ini");
    if let Ok(text) = std::fs::read_to_string(&overlay) {
        eprintln!("slot: launchers overlay {}", overlay.display());
        let over = LaunchTable::parse(&text);
        if !over.search.is_empty() && over.search != table.search {
            // Only replace search if the overlay set it explicitly by having a [paths] or default keys.
        }
        // Overlay always wins per platform; paths replace when the overlay mentioned them.
        if text.contains("search=") {
            table.search = over.search;
        }
        if text.contains("cores=") || text.contains("core=") && text.contains("[paths]") {
            table.cores = over.cores;
        }
        for (k, v) in over.by_platform {
            table.by_platform.insert(k, v);
        }
    }
    table
}

pub fn bundled_ini() -> &'static str {
    include_str!("../../../rg280v/launchers.ini")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn the_bundled_table_covers_the_21_shipped_systems() {
        let table = LaunchTable::parse(bundled_ini());
        let missing: Vec<_> = Platform::ALL
            .iter()
            .copied()
            .filter(|p| p.shipped() && !p.hidden_on_rg280v())
            .filter(|p| table.for_platform(*p).is_none())
            .map(|p| p.dir_name().to_string())
            .collect();
        assert!(missing.is_empty(), "missing launchers for {missing:?}");
        // Hidden shelves have no default launcher.
        for p in [Platform::N64, Platform::Nds, Platform::Psp] {
            assert!(table.for_platform(p).is_none(), "{p:?} should stay unconfigured");
        }
    }

    #[test]
    fn parse_reads_opk_lists_and_exec() {
        let t = LaunchTable::parse(
            r#"
[paths]
search = /tmp/apps:/opt/apps
[gb]
opk = gambatte.opk, GAMBATTE.opk
retroarch_core = gambatte_libretro.so
[gba]
exec = /usr/bin/regba {rom}
"#,
        );
        assert_eq!(t.search, ["/tmp/apps", "/opt/apps"]);
        let gb = t.for_platform(Platform::Gb).unwrap();
        assert_eq!(gb.opk, ["gambatte.opk", "GAMBATTE.opk"]);
        assert_eq!(gb.retroarch_core, ["gambatte_libretro.so"]);
        assert_eq!(
            t.for_platform(Platform::Gba).unwrap().exec.as_deref(),
            Some("/usr/bin/regba {rom}")
        );
    }

    #[test]
    fn command_prefers_an_opk_that_is_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let opk = dir.path().join("gambatte.opk");
        std::fs::write(&opk, b"opk").unwrap();
        let t = LaunchTable::parse(&format!(
            "[paths]\nsearch = {}\n[gb]\nopk = missing.opk, gambatte.opk\n",
            dir.path().display()
        ));
        let cmd = t.command(Platform::Gb, Path::new("/roms/GB/game.gb")).unwrap();
        let words: Vec<_> = [cmd.get_program()]
            .into_iter()
            .chain(cmd.get_args())
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words,
            vec![
                "opkrun".into(),
                opk.display().to_string(),
                "/roms/GB/game.gb".into()
            ]
        );
        let _ = writeln!(std::io::sink(), "ok");
    }

    #[test]
    fn command_falls_back_to_retroarch() {
        let dir = tempfile::tempdir().unwrap();
        let core = dir.path().join("fceumm_libretro.so");
        std::fs::write(&core, b"so").unwrap();
        let t = LaunchTable::parse(&format!(
            "[paths]\ncores = {}\n[nes]\nopk = no-such.opk\nretroarch_core = fceumm_libretro.so\n",
            dir.path().display()
        ));
        let cmd = t.command(Platform::Nes, Path::new("/roms/NES/g.nes")).unwrap();
        let words: Vec<_> = [cmd.get_program()]
            .into_iter()
            .chain(cmd.get_args())
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            words,
            vec![
                "retroarch".into(),
                "-L".into(),
                core.display().to_string(),
                "/roms/NES/g.nes".into()
            ]
        );
    }

    #[test]
    fn missing_emulator_is_an_error_not_a_panic() {
        let t = LaunchTable::parse("[gb]\nopk = definitely-missing.opk\n");
        let err = t
            .command(Platform::Gb, Path::new("/roms/GB/g.gb"))
            .unwrap_err();
        assert!(err.contains("no emulator found"), "{err}");
    }
}
