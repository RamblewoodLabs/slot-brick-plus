//! Emulator and system registry for Slotted.
//!
//! This is the source of truth for package names, launch styles, ROM folder
//! aliases, and libretro core names. The Kotlin UI reads it through JNI.

pub const REGISTRY_JSON: &str = r#"{
  "emulators": [
    {
      "id": "dolphin",
      "name": "Dolphin",
      "packages": [
        {"package": "org.dolphinemu.dolphinemu", "activity": "org.dolphinemu.dolphinemu.ui.main.MainActivity"}
      ],
      "launch": "autostart_file",
      "install_url": "https://play.google.com/store/apps/details?id=org.dolphinemu.dolphinemu",
      "install_alt_url": "https://dolphin-emu.org/download/",
      "license": "GPL-2.0-or-later",
      "notes": "Do not use ACTION_VIEW; MainActivity rejects it. Pass AutoStartFile plus a content URI and FLAG_GRANT_READ_URI_PERMISSION."
    },
    {
      "id": "armsx2",
      "name": "ARMSX2",
      "packages": [
        {"package": "come.nanodata.armsx2", "activity": "com.nanodata.MainActivity"},
        {"package": "come.nanodata.armsx2", "activity": "kr.co.iefriends.pcsx2.activities.MainActivity"},
        {"package": "com.armsx2", "activity": "com.armsx2.MainActivity"},
        {"package": "com.armsx2", "activity": "com.armsx2.ui.MainActivity"}
      ],
      "launch": "view",
      "install_url": "https://play.google.com/store/apps/details?id=come.nanodata.armsx2",
      "install_alt_url": "https://armsx2.net/download",
      "license": "GPL-3.0",
      "notes": "Play Store package is come.nanodata.armsx2; GitHub/sideload package is com.armsx2. Try listed activities until one resolves. Never recommend AetherSX2 or NetherSX2."
    },
    {
      "id": "eden",
      "name": "Eden",
      "packages": [
        {"package": "dev.eden.eden_emulator", "activity": "org.yuzu.yuzu_emu.activities.EmulationActivity"},
        {"package": "dev.eden.eden_nightly", "activity": "org.yuzu.yuzu_emu.activities.EmulationActivity"},
        {"package": "com.miHoYo.Yuanshen", "activity": "org.yuzu.yuzu_emu.activities.EmulationActivity"}
      ],
      "launch": "view",
      "install_url": "https://eden-emu.dev/downloads/",
      "license": "GPL-3.0-or-later",
      "notes": "Prefer the official Standard APK (dev.eden.eden_emulator). Nightly is second. com.miHoYo.Yuanshen is Eden's official Genshin-spoof build, used only if the others are missing. We never bundle prod.keys or firmware."
    },
    {
      "id": "azahar",
      "name": "Azahar",
      "packages": [
        {"package": "org.azahar_emu.azahar", "activity": "org.citra.citra_emu.activities.EmulationActivity"},
        {"package": "io.github.lime3ds.android", "activity": "org.citra.citra_emu.activities.EmulationActivity"}
      ],
      "launch": "view",
      "install_url": "https://play.google.com/store/apps/details?id=io.github.lime3ds.android",
      "install_alt_url": "https://github.com/azahar-emu/azahar/releases",
      "license": "GPL-3.0-or-later",
      "notes": "Vanilla (recommended) is org.azahar_emu.azahar since 2125.0. Play Store keeps io.github.lime3ds.android."
    },
    {
      "id": "vita3k",
      "name": "Vita3K",
      "packages": [
        {"package": "org.vita3k.emulator", "activity": "org.vita3k.emulator.Emulator"}
      ],
      "launch": "vita_title_id",
      "install_url": "https://github.com/Vita3K/Vita3K-Android/releases",
      "license": "GPL-2.0",
      "notes": "Frontend launch is by Title ID: AppStartParameters = [-r, TITLEID]. VPKs must already be installed in Vita3K. If the filename contains a PCSG/PCSE/PCSA-style ID we use it; otherwise we open Vita3K and tell the user to install the VPK there first."
    },
    {
      "id": "hakux",
      "name": "hakuX (Hakuchou)",
      "packages": [
        {"package": "com.rfandango.haku_x", "activity": "com.rfandango.haku_x.LauncherActivity"},
        {"package": "com.rfandango.haku_x.debug", "activity": "com.rfandango.haku_x.LauncherActivity"}
      ],
      "launch": "view",
      "install_url": "https://github.com/rfandango/hakuX/releases",
      "license": "GPL-3.0-or-later",
      "notes": "Jeff's 'Hakuchou' is this project: official repo https://github.com/rfandango/hakuX, a xemu-lineage Xbox emulator for arm64 Android. Intent launch is clean (ACTION_VIEW + URI). Official download is GitHub releases only. Current README (v1.3.0) says no BIOS, MCPX ROM or HDD image is required."
    },
    {
      "id": "retroarch",
      "name": "RetroArch",
      "packages": [
        {"package": "com.retroarch.aarch64", "activity": "com.retroarch.browser.retroactivity.RetroActivityFuture"},
        {"package": "com.retroarch", "activity": "com.retroarch.browser.retroactivity.RetroActivityFuture"},
        {"package": "com.retroarch.ra32", "activity": "com.retroarch.browser.retroactivity.RetroActivityFuture"}
      ],
      "launch": "retroarch",
      "install_url": "https://play.google.com/store/apps/details?id=com.retroarch",
      "install_alt_url": "https://www.retroarch.com/?page=platforms",
      "license": "GPL-3.0",
      "notes": "INTERIM host for the 24 handheld systems plus Flycast (Dreamcast) and Beetle Saturn. The user must install the matching core in RetroArch's updater. Direct libretro embedding in Slotted comes later."
    }
  ],
  "systems": [
    {"id":"gb","name":"Game Boy","folders":["GB","gb"],"extensions":[".gb",".zip",".7z"],"emulator":"retroarch","core":"mgba","on_handheld":true},
    {"id":"gbc","name":"Game Boy Color","folders":["GBC","gbc"],"extensions":[".gbc",".gb",".zip",".7z"],"emulator":"retroarch","core":"mgba","on_handheld":true},
    {"id":"gba","name":"Game Boy Advance","folders":["GBA","gba"],"extensions":[".gba",".zip",".7z"],"emulator":"retroarch","core":"mgba","on_handheld":true},
    {"id":"nes","name":"NES","folders":["NES","nes","famicom"],"extensions":[".nes",".zip",".7z"],"emulator":"retroarch","core":"fceumm","on_handheld":true},
    {"id":"snes","name":"SNES","folders":["snes","SNES","sfc"],"extensions":[".sfc",".smc",".zip",".7z"],"emulator":"retroarch","core":"snes9x","on_handheld":true},
    {"id":"ps1","name":"PlayStation","folders":["PSX","PS1","psx","ps1"],"extensions":[".chd",".cue",".bin",".pbp",".iso",".m3u",".exe"],"emulator":"retroarch","core":"pcsx_rearmed","on_handheld":true},
    {"id":"md","name":"Genesis","folders":["genesis","megadrive","MD","md"],"extensions":[".md",".gen",".smd",".bin",".zip",".7z"],"emulator":"retroarch","core":"genesis_plus_gx","on_handheld":true},
    {"id":"sms","name":"Master System","folders":["mastersystem","sms","SMS","mark3"],"extensions":[".sms",".zip",".7z"],"emulator":"retroarch","core":"genesis_plus_gx","on_handheld":true},
    {"id":"gg","name":"Game Gear","folders":["gamegear","gg","GG"],"extensions":[".gg",".zip",".7z"],"emulator":"retroarch","core":"genesis_plus_gx","on_handheld":true},
    {"id":"arcade","name":"Arcade","folders":["FBNEO","fbneo","arcade","ARCADE","mame","neogeo","CPS1","CPS2","CPS3","cps"],"extensions":[".zip"],"emulator":"retroarch","core":"fbneo","on_handheld":true},
    {"id":"pce","name":"TurboGrafx-16","folders":["tg16","pcengine","PCE","pce"],"extensions":[".pce",".sgx",".zip",".7z"],"emulator":"retroarch","core":"mednafen_pce_fast","on_handheld":true},
    {"id":"a2600","name":"Atari 2600","folders":["ATARI2600","atari2600","A2600"],"extensions":[".a26",".bin",".zip"],"emulator":"retroarch","core":"stella2014","on_handheld":true},
    {"id":"intv","name":"Intellivision","folders":["Intellivision","intellivision","INTV"],"extensions":[".int",".bin"],"emulator":"retroarch","core":"freeintv","on_handheld":true},
    {"id":"lynx","name":"Atari Lynx","folders":["atarilynx","lynx","LYNX"],"extensions":[".lnx",".zip"],"emulator":"retroarch","core":"handy","on_handheld":true},
    {"id":"a7800","name":"Atari 7800","folders":["ATARI7800","atari7800","A7800"],"extensions":[".a78",".bin",".zip"],"emulator":"retroarch","core":"prosystem","on_handheld":true},
    {"id":"gw","name":"Game & Watch","folders":["gameandwatch","GW","gw"],"extensions":[".mgw"],"emulator":"retroarch","core":"gw","on_handheld":true},
    {"id":"coleco","name":"ColecoVision","folders":["colecovision","COLECO","coleco"],"extensions":[".col",".rom"],"emulator":"retroarch","core":"gearcoleco","on_handheld":true},
    {"id":"a5200","name":"Atari 5200","folders":["ATARI5200","atari5200","A5200"],"extensions":[".a52",".bin"],"emulator":"retroarch","core":"a5200","on_handheld":true},
    {"id":"a800","name":"Atari 800","folders":["ATARI800","atari800","A800"],"extensions":[".xex",".atr",".atx",".xfd",".car",".cas"],"emulator":"retroarch","core":"atari800","on_handheld":true},
    {"id":"vectrex","name":"Vectrex","folders":["vectrex","VECTREX"],"extensions":[".vec",".gam",".7z"],"emulator":"retroarch","core":"vecx","on_handheld":true},
    {"id":"s32x","name":"32X","folders":["sega32x","32X","s32x"],"extensions":[".32x",".bin"],"emulator":"retroarch","core":"picodrive","on_handheld":true},
    {"id":"n64","name":"Nintendo 64","folders":["n64","N64"],"extensions":[".z64",".n64",".v64",".zip",".7z"],"emulator":"retroarch","core":"mupen64plus_next","on_handheld":true},
    {"id":"nds","name":"Nintendo DS","folders":["nds","NDS"],"extensions":[".nds",".zip"],"emulator":"retroarch","core":"melonds","on_handheld":true},
    {"id":"psp","name":"PSP","folders":["PSP","psp"],"extensions":[".iso",".cso",".chd",".pbp"],"emulator":"retroarch","core":"ppsspp","on_handheld":true},
    {"id":"gc","name":"GameCube","folders":["gc","GC","gamecube"],"extensions":[".iso",".gcz",".ciso",".rvz",".gcm",".tgc",".wbfs"],"emulator":"dolphin","on_handheld":false},
    {"id":"ps2","name":"PlayStation 2","folders":["PS2","ps2"],"extensions":[".iso",".chd",".bin",".cue",".cso"],"emulator":"armsx2","on_handheld":false},
    {"id":"switch","name":"Switch","folders":["Switch","switch"],"extensions":[".nsp",".xci",".nsz",".xcz"],"emulator":"eden","on_handheld":false},
    {"id":"n3ds","name":"Nintendo 3DS","folders":["3DS","3ds","n3ds"],"extensions":[".cia",".cci",".3ds",".cxi",".app"],"emulator":"azahar","on_handheld":false},
    {"id":"dreamcast","name":"Dreamcast","folders":["dreamcast","dc"],"extensions":[".chd",".gdi",".cdi",".cue",".bin",".m3u"],"emulator":"retroarch","core":"flycast","on_handheld":false},
    {"id":"saturn","name":"Saturn","folders":["saturn","saturnjp"],"extensions":[".chd",".cue",".bin",".iso",".m3u"],"emulator":"retroarch","core":"mednafen_saturn","on_handheld":false},
    {"id":"vita","name":"PlayStation Vita","folders":["psvita","PSVITA","vita"],"extensions":[".vpk"],"emulator":"vita3k","on_handheld":false},
    {"id":"xbox","name":"Xbox","folders":["xbox","Xbox"],"extensions":[".iso",".xiso",".chd"],"emulator":"hakux","on_handheld":false}
  ]
}"#;

pub fn core_so_name(core: &str) -> String {
    format!("{core}_libretro_android.so")
}
