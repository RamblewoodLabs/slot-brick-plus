# Slot for Android — Plan

**Branch:** `cursor/android-scaffold`  
**Worktree:** `~/sbp-android` on the Mac mini  
**Base:** `origin/main` at `efa0a4d` (PR #34, license-fix), 26 Nov 2026.  
**Status:** planning + scaffold only. This branch must be rebased after the pending SP/Brick merges (#27–#33 and companions) land on `main`. This worktree does not touch `main` or the other `~/sbp-*` worktrees.

Slot itself is **GPL-3.0-or-later**. Some bundled cores are non-commercial (FinalBurn Neo, Genesis Plus GX). The Android app must stay fully free: no paid listing, no donations if those cores ship in the APK. Every core, font and library license goes in the APK, the same work already started for the cards.

We never download or ship BIOS, firmware, or console keys. The owner supplies dumps from hardware they own.

---

## 1. Library inventory

Read-only listing of `D:\Emulation\ROMS` on the EVO-X1 (`Games` volume, about 3.78 TB used) on 26 Nov 2026, 6:33 PM PT. Directory listings and file counts/sizes only. No writes, moves or renames. `E:` (SSK) and `F:` (card reader) were not touched.

The raw table is in [`inventory.tsv`](inventory.tsv). There are 216 system folders. Most are empty Batocera/ES placeholders (one or two `.txt` files). The tables below are the folders that actually hold games, plus the 24 shelves Slot already supports.

File counts include small metadata files (`.txt`, `.db`, `.dat`). Sizes are the full tree.

### 1.1 The 24 shelves already on SP / Brick

These 24 are the Slot shelves documented in `docs/wiki/Systems-and-Cores.md`. They are already on this branch's base (`main` after PRs #31 and #32).

| Shelf (Slot folder) | Source folder | Files | Size | On SP/Brick? |
| :--- | :--- | ---: | :--- | :--- |
| Game Boy (`GB`) | `GB` | 541 | 115.6 MB | Yes |
| Game Boy Color (`GBC`) | `GBC` | 461 | 597.5 MB | Yes |
| Game Boy Advance (`GBA`) | `GBA` | 951 | 6.99 GB | Yes |
| NES / Famicom (`NES`) | `NES` | 690 | 139.5 MB | Yes |
| SNES (`SNES`) | `snes` | 768 | 1.01 GB | Yes |
| PlayStation (`PS1`) | `PSX` | 135 | 24.35 GB | Yes |
| Genesis / Mega Drive (`MD`) | `genesis` | 975 | 1.23 GB | Yes |
| Master System (`SMS`) | `mastersystem` | 242 | 68.2 MB | Yes |
| Game Gear (`GG`) | `gamegear` | 228 | 75.9 MB | Yes |
| Arcade (`ARCADE`) | `FBNEO` (see note) | 303 | 3.16 GB | Yes |
| TurboGrafx-16 (`PCE`) | `tg16` | 95 | 37.0 MB | Yes |
| Atari 2600 (`A2600`) | `ATARI2600` | 52 | 232 KB | Yes |
| Intellivision (`INTV`) | `Intellivision` | 168 | 2.82 MB | Yes |
| Atari Lynx (`LYNX`) | `atarilynx` | 87 | 18.2 MB | Yes |
| Atari 7800 (`A7800`) | `ATARI7800` | 32 | 998 KB | Yes |
| Game & Watch (`GW`) | `gameandwatch` | 61 | 52.8 MB | Yes |
| ColecoVision (`COLECO`) | `colecovision` | 97 | 1.75 MB | Yes |
| Atari 5200 (`A5200`) | `ATARI5200` | 35 | 521 KB | Yes |
| Atari 800 (`A800`) | `ATARI800` | 51 | 1.49 MB | Yes |
| Vectrex (`VECTREX`) | `vectrex` | 37 | 203 KB | Yes |
| 32X (`32X`) | `sega32x` | 36 | 99.5 MB | Yes |
| Nintendo 64 (`N64`) | `n64` | 731 | 8.61 GB | Yes |
| Nintendo DS (`NDS`) | `nds` | 246 | 14.32 GB | Yes |
| PSP (`PSP`) | `PSP` | 636 | 337.14 GB | Yes |

**Arcade note:** `D:\Emulation\ROMS\arcade` is empty (placeholder only). The arcade games in this library are in `FBNEO` (303 zips, 3.16 GB), `mame` (379 zips, 1.15 GB), `neogeo` (123 zips, 2.23 GB), `CPS1` (53, 129 MB), `CPS2` (40, 519 MB) and `CPS3` (14, 426 MB). The Android app should map those folders onto one Arcade shelf (and keep the FBNeo short names).

Empty sibling folders such as `megadrive`, `pcengine`, `n3ds`, and `pico8` are placeholders; the real sets are `genesis`, `tg16`, `3DS`, and `PICO`.

### 1.2 High-end and other populated folders not on the handhelds

These are the reason for the Android build.

| Source folder | Files | Size | Suggested Android shelf | Notes |
| :--- | ---: | :--- | :--- | :--- |
| `gc` | 125 | 163.33 GB | GameCube | `.iso` / `.gcz`. **Wii folder is empty.** |
| `PS2` | 119 | 390.84 GB | PlayStation 2 | `.iso` (2 `.chd`) |
| `Switch` | 270 | 753.28 GB | Nintendo Switch | `.nsp` / `.xci` |
| `Switch Updates` | 208 | 104.17 GB | (with Switch) | updates / DLC, not a separate shelf |
| `Switch2` | 217,568 | 106.99 GB | *none* | **On `D:`.** Unpacked dump tree (`.zs`, `.extm`, `.bwav`, …), not packaged `.xci`/`.nsp`. No viable Android emulator as of this plan. |
| `3DS` | 283 | 263.54 GB | Nintendo 3DS | `.cia` / `.cci` |
| `dreamcast` | 428 | 182.77 GB | Dreamcast | mostly `.chd` |
| `wiiu` | 33 | 166.98 GB | Wii U (later, weak) | 31 `.wua` (Cemu compact). No solid Android emulator yet. |
| `xbox` | 22 | 54.71 GB | Original Xbox | `.iso` / `.chd` |
| `psvita` | 23 | 33.02 GB | PlayStation Vita | `.vpk` |
| `saturn` | 134 | 21.54 GB | Sega Saturn | `.chd` / `.bin` |
| `PS3` | 54,723 | 341.45 GB | *none for v1* | Looks like **unpacked game data** (`.snu`, `.xvag`, `.str`, …), not ISO/PKG. RPCS3 is not practical on Android. |
| `3DO` | 46 | 4.24 GB | 3DO | `.chd`; BIOS required |
| `atarijaguar` | 48 | 111.1 MB | Atari Jaguar | `.jag` |
| `PICO` | 1,131 | 33.5 MB | PICO-8 | `.png` carts |
| `CPC` | 281 | 50.9 MB | Amstrad CPC | `.dsk` |
| `C64` | 40 | 3.19 MB | Commodore 64 | `.7z` |
| `DOS` | 40 | 738.9 MB | DOS | plus a small `MSDOS` zip set (5.1 MB) |
| `TeknoParrot ROMS` | 26 | 42.25 GB | *none* | PC arcade (Windows). Not an Android target. |

`xbox360` and `ps4` are empty placeholders. `segacd` / `megacd` are empty — there is no Sega CD set in this folder.

### 1.3 Switch 2

The library is already on `D:\Emulation\ROMS\Switch2` (107 GB, 217k files). The files are unpacked dump contents, not playable packages. There is no recommended Android emulator for Switch 2, and we will not add a shelf that cannot launch. A later "archive only" view is optional if Jeff wants to browse it.

---

## 2. Android emulator mapping

Sources checked 26 Nov 2026: Held Games' 2026 Android emulator list, official project sites and repositories (Dolphin, ARMSX2 / armsx2.net, Eden / eden-emu.dev, Azahar, Vita3K, Flycast), libretro core docs, and the RetroArch issue that documents AetherSX2 / NetherSX2 as non-free.

**Embedding** here means loading a libretro core (or linking an emulator) inside our APK. **Intent** means the user installs the official standalone app and Slot hands it a content URI.

BIOS, firmware and keys are always supplied by Jeff, never downloaded by us.

| System | Recommended on Android | Embed or intent? | License | BIOS / keys |
| :--- | :--- | :--- | :--- | :--- |
| The current 24 shelves | Same cores Slot already vendors (mGBA, gpSP, FCEUmm, Snes9x, PCSX ReARMed, Genesis Plus GX, FBNeo, Beetle PCE Fast, Stella 2014, FreeIntv, Handy, ProSystem, gw, Gearcoleco, a5200, Atari800, vecx, PicoDrive, mupen64plus-next, melonDS, PPSSPP) | **Embed** as libretro `.so` via `slot-retro`. PPSSPP and N64 may also offer a standalone fallback if the embedded core struggles. | Mix of MPL/GPL/zlib; **FBNeo and Genesis Plus GX are non-commercial**. Handy is zlib (label was corrected in PR #34). | Same as the handhelds: Intellivision `exec.bin`/`grom.bin` required; Coleco BIOS required; PS1 optional; Lynx boot image; DS BIOS optional. Jeff already has these on the cards. |
| GameCube (and Wii if a set appears) | **Dolphin** standalone. A libretro Dolphin core exists (GPLv2+) but the Android standalone build is the one that actually plays the library well. | **Intent first**; consider embed later. | GPLv2+ — compatible with Slot's GPLv3. | No required BIOS; optional IPL from a console Jeff owns. |
| PlayStation 2 | **ARMSX2** (current PCSX2-based, still developed, official Play Store + [armsx2.net](https://armsx2.net/download)). Do **not** ship or recommend AetherSX2 or NetherSX2: AetherSX2's source was never published; NetherSX2 is a patch on that non-free APK. | **Intent** to `com.armsx2` / Play package `come.nanodata.armsx2`. Embedding later is possible in principle (ARMSX2 is a GPL-3 PCSX2 fork) but is a large port. | GPL-3.0 | PS2 BIOS dumped from a console Jeff owns, installed in ARMSX2. We never fetch it. |
| Nintendo Switch | **Eden** ([eden-emu.dev](https://eden-emu.dev/)) is the current maintained Android default on Snapdragon 8 Gen 2-class devices. Citron is slowing; Sudachi and other Yuzu forks have already been taken down. The scene is legally noisy. | **Intent only.** Do not bundle a Switch emulator in our APK. Jeff decides whether we support this shelf at all (see §7). | GPL-3-or-later | `prod.keys` and firmware **dumped from a Switch Jeff owns**. We never download, bundle, or link to keys or firmware. |
| Nintendo Switch 2 | None. | No shelf. | — | — |
| Nintendo 3DS | **Azahar** (Citra lineage), official releases. | Intent. | GPL-3.0-or-later | Usually none; user-dumped DSP firmware optional. |
| PlayStation Vita | **Vita3K** Android. | Intent. | GPL-2.0 | User-dumped Vita firmware. |
| Dreamcast | **Flycast**, libretro core and standalone. | **Embed** (Flycast libretro is strong on Android). | GPL-2.0 | Usually none. |
| Sega Saturn | Beetle Saturn (libretro) or Yaba Sanshiro 2 standalone. | Embed Beetle Saturn if BIOS is present; otherwise intent. | GPL | Saturn BIOS from a console Jeff owns, strongly recommended. |
| 3DO | Opera (libretro). | Embed. | | Required 3DO BIOS. |
| Original Xbox | **hakuX** is the Android name being used in 2026 guides; confirm the live project at implementation time. xemu is still a PC emulator. | Intent, if a maintained official APK exists; otherwise defer the shelf. | Reportedly GPL-3 (confirm). | MCPX / BIOS dumps from a console Jeff owns. Never download. |
| Wii U | No recommended Android emulator. Cemu is a PC app; the `.wua` set should wait. | Defer. | — | — |
| PS3, PS4, Xbox 360 | Not practical on Android. The PS3 folder also does not look like a packaged library. | No shelf. | — | — |
| Atari Jaguar | Virtual Jaguar libretro, or standalone if quality is poor. | Embed, best-effort. | | |
| PICO-8 | fake-08 / Retro8 libretro. Do not bundle Lexaloffle's official (commercial) PICO-8. | Embed. | fake-08 license (confirm) | |
| DOS | DOSBox Pure (libretro). | Embed. | GPL | |
| Commodore 64 | VICE (libretro). | Embed. | GPL | |
| Amstrad CPC | Caprice32 (libretro). | Embed. | GPL | |
| TeknoParrot / Model 2 / 3 / Naomi | Not Android targets. | No shelf. | — | — |

PPSSPP, DuckStation and melonDS also have excellent Android standalone apps. For systems already in Slot we still prefer embedding so the shelf, insert animation and RetroAchievements stay in one UI. Standalone remains a fallback.

---

## 3. Architecture

### 3.1 Recommendation: one Rust UI, two launch paths

Keep Slot's own frontend. Do not rewrite the shelf in Kotlin or Flutter.

```
┌─────────────────────────────────────────────────────────┐
│  Slot Android APK  (GPL-3.0-or-later)                   │
│  rustc → libslot.so  + thin Java/Kotlin GameActivity    │
│                                                         │
│  slot-ui / slot-gfx / slot-store / slot-input           │
│  slot-cheevos / slot-retro                              │
│                                                         │
│  New slot-gfx host: EGL + ANativeWindow (GameActivity)  │
│  New I/O: Storage Access Framework ROM tree             │
│  New input: Android KeyEvent / MotionEvent + touch      │
│                                                         │
│  ┌──────────────┐     ┌──────────────────────────────┐  │
│  │ Embed        │     │ Intent-launch                │  │
│  │ libretro .so │     │ Dolphin, ARMSX2, Eden,       │  │
│  │ (24 + DC,    │     │ Azahar, Vita3K, (hakuX)      │  │
│  │  Saturn, …)  │     │ content URI + persist grant  │  │
│  └──────────────┘     └──────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

**Why not a Kotlin-only shell?** The shelf, 3D cart spins, filters, favorites, and RetroAchievements already exist in Rust. A second UI would drift immediately.

**Why not embed everything?** Dolphin, ARMSX2 and Eden are large, GPU-driver-sensitive projects. Their Android builds already handle Turnip drivers, BIOS installers and per-game settings. Intent-launch gets Jeff playing GameCube / PS2 / Switch sooner and avoids shipping a second copy of those emulators. We can revisit embedding Dolphin (GPL, compatible) later.

**Tooling:** `cargo-ndk` + official NDK r27, and a small Gradle wrapper (or `cargo apk`) to produce the APK. Target `aarch64-linux-android`. minSdk 26 (Android 8), targetSdk 35. Rust 1.100 is now on this Mac mini; the workspace wants 1.96+.

### 3.2 Porting the existing crates

| Crate | Android work |
| :--- | :--- |
| `slot-gfx` | New host behind a feature `android`: EGL 3 / GLES 3 on `ANativeWindow`. Keep `host` (winit/glutin, desktop sim) and `device` (fbdev) untouched. |
| `slot-ui` | Mostly reusable. Replace hardcoded 640×480 / 1024×768 assumptions with a letterboxed or scale-to-height shelf on 16:9 / 20:9 handheld panels (1080p and 1920×1200 are typical). |
| `slot-input` | Map Android axis/hat/button codes. Ship profiles for AYN Odin-class and Retroid-class pads, plus a generic HID profile. Touch: swipe the shelf, tap a cart. |
| `slot-store` | Scan the user-picked tree. Understand both Slot's `Slot/Games/<SYS>` layout and Jeff's `D:\Emulation\ROMS\<folder>` names (a small alias table). |
| `slot-retro` | Load `.so` cores from the APK's `jniLibs` or an app-writable cores directory. GLES vs Vulkan is a per-core choice (PPSSPP, mupen, Flycast prefer GLES/Vulkan). |
| `slot-cheevos` | Reuse as-is (Encore included once #27 is on main). HTTPS via rustls already. |
| `slot-power` | Mostly unused on Android; the OS handles sleep. Optional: listen for power-button / pause to auto-save. |

### 3.3 Controller input

- Read the built-in controls of an Android handheld as a gamepad (they already show up as one).
- Also support a Bluetooth pad on a phone or tablet.
- Keep Slot's existing button legends, with a fallback on-screen map if no pad is present.
- Analog sticks matter here (N64, PSP, GameCube, PS2, Switch). The handhelds had to fake a stick with the D-pad; Android should use real sticks.

### 3.4 Storage (SAF)

- First launch: a folder picker (`ACTION_OPEN_DOCUMENT_TREE`) for the ROM root. Persist the URI permission.
- Optionally a second picker for BIOS/keys (or a `BIOS` folder next to the ROMs).
- Large ISO/NSP/CIA files perform better with real filesystem access (`MANAGE_EXTERNAL_STORAGE` / "all files" on some handhelds). Try SAF first; offer the broader permission only if launches fail or copy-to-cache is too slow.
- Saves, states, labels and the achievement cache live in app storage (or a `Slot` folder next to the ROMs, Jeff's choice).
- We do not scan the whole 2+ TB library into memory the way a naïve walk would. Index on first scan, then watch for changes.

The full `D:\Emulation\ROMS` tree is well over **2 TB** of real games (Switch alone is 753 GB + 104 GB updates, PS2 391 GB, PSP 337 GB, 3DS 264 GB, Dreamcast 183 GB, GameCube 163 GB, Wii U 167 GB). Jeff will need that tree on a fast SD card or the device's internal storage. Ask how he wants to place it.

### 3.5 RetroAchievements

Carry over the existing client: login, badge cache limited to favorites + recent (PR #27), Encore mode, unlock popups. For intent-launched systems, achievements only work if the standalone emulator has its own RA support (Dolphin does on some builds; ARMSX2 / Eden need a per-app check). Embedded libretro cores go through `slot-cheevos` as they do today.

### 3.6 Target devices

Default assumption, until Jeff picks one: a **modern Snapdragon Android handheld** in the AYN Odin / Retroid class (Snapdragon 8 Gen 2 or better, 8 GB+ RAM, 6–7" 1080p, Hall sticks, shoulder buttons). That class is what Eden and Dolphin actually need for Switch and the heavier GameCube titles. ARMSX2 is happier from a Snapdragon 865 upward.

The same APK should still run on a phone or tablet with a gamepad, at a lower expectation for Switch / PS2 / GC.

---

## 4. Poster / box art scraping

Slot already fetches labels on the handhelds (ScreenScraper via Cart Studio for GB/GBC/GBA; LaunchBox cart/disc scans for NES, SNES and PS1; ES-DE media and libretro-thumbnails for the later shelves). Android should let Jeff scrape art for the much larger, high-end library as well. **We never bundle scraped art in the APK or in git.** The app only downloads pictures for games in his own folder, onto his device.

### 4.1 Comparison

| Source | Auth | Rate limits | Commercial / license | User can supply their own account? | Attribution | Fit for Slot | Deal needed? |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **ScreenScraper.fr** WebAPI v2 | Developer `devid` / `devpassword` issued after a forum presentation of the app, plus the **user's** `ssid` / `sspassword`. `softname` required. | Per-user quota: threads (1 for a new free account, more for contributors/donors), requests per minute (≈ threads × 50) and per day (about 20k–100k). The app **must** honour the quota fields returned by the API. | **Free apps only**, or written permission from the ScreenScraper team. Matches our GPL + FBNeo non-commercial rule. | **Yes, and they should.** Their level sets the quota. | Not a formal logo rule; credit ScreenScraper in Settings / Credits. | **Best primary source.** Official box, cart, disc, wheel and marquee art, CRC matching. Already in Slot's pipeline (Cart Studio / ES-DE). | No contract if we stay fully free and register the developer account. |
| **libretro-thumbnails** | None. Static files at `thumbnails.libretro.com` / GitHub. | None published. Be polite (no bulk mirror). | **No license.** Maintainers have said they do not own the box art and recommended against packaging the set in distros. Grey area. | N/A | Credit libretro-thumbnails and the original publishers in Credits. | **Easy fallback**, already used by `brick/build_sp_art.py`. On-demand only; never vendor the 8 GB set. | No. |
| **TheGamesDB** | Project API key, requested on their forum with a description of the app. Keys are **not** for end users. | After a Feb 2026 key revocation, new keys have been reported as **1,000 scrapes per month per user/IP**. That is too small for this library (Genesis alone is ~975 games). | No clear official commercial-use page found this session. Third-party writeups say personal use, no commercial redistribution. Cache locally; back off on 429. | No. A project key is shared by everyone, which is how earlier keys got burned. | Keep source URLs; credit TheGamesDB. | Weak as a primary Android source because of the quota and shared key. Optional later if they issue a higher-limit key. | Asking for a key is a forum request, not a paid deal; still not enough quota to rely on. |
| **IGDB** (Twitch) | Twitch application Client ID + secret, client-credentials token. **Not** end-user Twitch logins. | 4 requests/s, 8 in flight; HTTP 429 if exceeded. | Free for non-commercial use under the Twitch Developer Services Agreement. Commercial use needs a partnership (`partner@igdb.com`) and **visible, static attribution to IGDB.com**. | Not in the usual sense. A client secret must not be shipped in the APK. Jeff could paste his own Twitch app credentials, or we add a tiny proxy later. | Required for a commercial partnership; good practice anyway. | Strong **text** metadata (descriptions, genres, dates, ratings). Weaker than ScreenScraper for real cart/box scans. | No deal if we stay non-commercial and keep secrets out of the APK. A partnership only if the app were ever monetized (we should not). |
| **SteamGridDB** | API key from the user's own account preferences. Straightforward REST API. | A free-key quota exists; exact published numbers were not on a crawlable official page this session. Page results, back off on 401/429. | Official ToS page is JavaScript-heavy and was not readable here. The art is **user-uploaded** (often fan-made Steam-style grids, not official boxes). The API wrapper is MIT; the images stay with their authors. | **Yes.** This is the right model: each user pastes their own key. | Credit the artist and SteamGridDB. | Excellent for **modern** shelves (Switch, GameCube, PS2, Xbox) when Jeff wants a consistent grid look. Not a substitute for real cart labels. | No licensing deal apparent; user-owned key, no bundling. Confirm ToS at implementation time. |
| **LaunchBox Games DB** | Already used on-device for NES / SNES / PS1 labels. | Informal; be gentle. | Not a documented third-party API. | N/A | Credit LaunchBox, as the wiki already does. | Keep as a desktop/build-time source. Do not grow this into a first-class Android API without their terms. | Do not assume permission for a new app integration. |

MobyGames and others were not evaluated in depth; they are generally more restrictive.

### 4.2 Recommendation

**Yes — add a "Get box art" flow, and it does not need a licensing deal** as long as Slot for Android stays free and we do not ship artwork.

1. **Primary: ScreenScraper.** Register "Slot" / "Slot Brick Plus" on their forum, get developer credentials, and let Jeff (and any later user) enter their own ScreenScraper account in Settings. Honour quotas and threads. Use box, cart, disc and marquee media the same way the handhelds already do.
2. **Fallback: libretro-thumbnails**, on demand, no account. Same grey-area practice as today's SP/Brick label build. Never vendor the set.
3. **Optional: SteamGridDB**, with Jeff's own API key, for high-end systems that look better as a cover grid than as a cart.
4. Skip TheGamesDB as a default (quota). Skip IGDB for v1 art; reconsider later for descriptions only, with credentials kept off the APK.
5. Credit every source on the Credits screen (the wiki work in PR #33 already started this).
6. Box art is copyrighted by the publishers. We only fetch it for games present in the user's folder, for display in this frontend, and we do not redistribute a scraped pack.

This is ordinary frontend work (ES-DE, Pegasus, RomM all do it). It is in scope after the scaffold, not in this PR's runtime code.

---

## 5. Scaffold and Phase 2 (this PR, still a draft)

Branch `cursor/android-scaffold`, worktree `~/sbp-android`. **Do not merge.**

### What works in the debug APK

- A Slot-styled shelf UI (Kotlin) plus a Rust JNI registry (`android/slot-android`).
- A Storage Access Framework ROM-folder picker. The tree URI is persisted. Folder names are matched case-insensitively to Jeff's library: `gc`, `PS2`, `Switch`, `3DS`, `dreamcast`, `saturn`, `psvita`, `xbox`, plus the 24 handheld folders (`GB`, `genesis`, `FBNEO`, `tg16`, …). Arcade also maps `mame`, `neogeo`, `CPS1`/`CPS2`/`CPS3`.
- Installed-app detection for every standalone emulator below (Android 11 `<queries>` included). If one is missing, Slot shows an **Install X** dialog that opens the official download page.
- Intent launch for GameCube, PS2, Switch, 3DS, Vita and Xbox, as in the table.
- **Interim** launch for the current 24 systems plus Dreamcast (Flycast) and Saturn (Beetle Saturn) through RetroArch and the named core. Embedding those libretro cores in Slot is the next step; it was too large for this chunk.
- Odin / Thor / Retroid-class gamepad mapping (Xbox layout, hat D-pad, left stick, L1/R1 to change system, A to launch, Start for the folder picker). The Thor's second screen is not used yet.

Build: `android/build-apk.sh` on the Mac mini. Output: `android/app/build/outputs/apk/debug/app-debug.apk`. Sideload with `adb install -r` after enabling USB debugging. Details are in `android/README.md`.

### Intent table (verified 26 Nov 2026)

| App | Package(s) | Activity | How we launch | Official install | License |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Dolphin | `org.dolphinemu.dolphinemu` | `.ui.main.MainActivity` | **No ACTION_VIEW** (MainActivity rejects it). `AutoStartFile` extra + content URI + `FLAG_GRANT_READ_URI_PERMISSION`. | Play Store / [dolphin-emu.org](https://dolphin-emu.org/download/) | GPLv2+ |
| ARMSX2 | Play: `come.nanodata.armsx2`; sideload: `com.armsx2` | `com.nanodata.MainActivity` or `kr.co.iefriends.pcsx2.activities.MainActivity`; sideload `com.armsx2.MainActivity` | `ACTION_VIEW` + URI. We try the listed activities until one resolves. | Play Store / [armsx2.net](https://armsx2.net/download) | GPL-3.0 |
| Eden | Preferred: `dev.eden.eden_emulator`; also `dev.eden.eden_nightly`; last resort: `com.miHoYo.Yuanshen` (official Genshin-spoof build) | `org.yuzu.yuzu_emu.activities.EmulationActivity` | `ACTION_VIEW` + URI. Jeff's own `prod.keys` and firmware. | [eden-emu.dev/downloads](https://eden-emu.dev/downloads/) | GPL-3-or-later |
| Azahar | Vanilla (preferred): `org.azahar_emu.azahar` (since 2125.0); Play: `io.github.lime3ds.android` | `org.citra.citra_emu.activities.EmulationActivity` | `ACTION_VIEW` + URI | Play Store / [GitHub releases](https://github.com/azahar-emu/azahar/releases) | GPL-3-or-later |
| Vita3K | `org.vita3k.emulator` | `org.vita3k.emulator.Emulator` | **Title ID**, not the VPK: extra `AppStartParameters` = `[-r, TITLEID]`. If the filename contains a `PCSE`/`PCSG`/… ID we use it. Otherwise we open Vita3K and tell Jeff to install the VPK there first, then rename the file to include the ID. | [Vita3K-Android releases](https://github.com/Vita3K/Vita3K-Android/releases) | GPL-2.0 |
| hakuX (Hakuchou) | `com.rfandango.haku_x` (debug `.debug`) | `com.rfandango.haku_x.LauncherActivity` | `ACTION_VIEW` + URI. Clean; no workaround. | [GitHub releases](https://github.com/rfandango/hakuX/releases) only | GPL-3-or-later |
| RetroArch (interim) | `com.retroarch.aarch64`, `com.retroarch`, `com.retroarch.ra32` | `com.retroarch.browser.retroactivity.RetroActivityFuture` | extras `ROM`, `LIBRETRO` (`<pkg>/cores/<core>_libretro_android.so`), `CONFIGFILE`. Jeff must install the core in RetroArch's updater. | Play Store | GPL-3.0 |

Do not use or recommend AetherSX2 or NetherSX2.

### Still to do after this chunk

1. Rebase onto merged main if more SP/Brick PRs land.
2. EGL `slot-gfx` host and the real `slot-ui` shelf on Android.
3. Embed libretro cores for the 24 + Flycast + Beetle Saturn (retire the RetroArch interim).
4. ScreenScraper art flow.
5. Thor second-screen layout, if Jeff wants it.
6. Vita Title ID lookup that does not depend on the filename, if Vita3K adds a better intent.

## 6. Decisions Jeff has made (26 Nov 2026)

1. **Target device:** an AYN Odin / Thor-class handheld (Snapdragon 8 Gen 2 or better). The Thor's second screen is optional later; v1 uses the primary display only.
2. **Switch:** include a Switch shelf that intent-launches official Eden. Jeff supplies his own `prod.keys` and firmware. We never bundle keys or firmware.
3. **Shelves after GameCube and PS2:** 3DS, Dreamcast, Saturn, Vita, and Xbox.
4. **Xbox emulator:** "hakux" means **Hakuchou / hakuX** (https://github.com/rfandango/hakuX), a GPL-3-or-later Android Xbox emulator in the xemu lineage. Verified 26 Nov 2026: package `com.rfandango.haku_x` (debug `com.rfandango.haku_x.debug`), activity `com.rfandango.haku_x.LauncherActivity`, launch `ACTION_VIEW` + content URI. Official download is GitHub releases only. Current README (v1.3.0) says no BIOS / MCPX / HDD image is required. Intent launch is clean; no workaround needed.
5. Still open: Switch 2 (archive-only vs ignore), where the 2+ TB library lives on the device, ScreenScraper developer registration, and cover style (box art vs grids).
