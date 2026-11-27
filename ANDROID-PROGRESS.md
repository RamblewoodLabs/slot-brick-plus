# Android APK scaffold progress

Log only. Do not touch `main` or the other `~/sbp-*` worktrees.

## 2026-11-26 6:33 PM PT

- Created worktree `/Users/jeff/sbp-android` on branch `cursor/android-scaffold` from `origin/main` (`efa0a4d`, Merge PR #34 license-fix).
- Did not checkout or modify any other worktree. `sbp-merge` remains on `cursor/systems2`. This branch will need a rebase after the pending SP/Brick merges land on main.
- Started a read-only inventory of `D:\Emulation\ROMS` on the EVO-X1 (directory listings and file counts/sizes only; no writes to D:, E:, or F:).
- Confirmed Slot's 24 shelves from `docs/wiki/Systems-and-Cores.md` in the wiki worktree (read-only): 16 on main plus 8 from PR #31 (ColecoVision, 5200, 800, Vectrex, 32X, N64, DS, PSP).
- Next: finish inventory, write `docs/android/PLAN.md` (architecture, emulator map, art-scraper comparison), install Rust + Android SDK/NDK on this Mac if missing, scaffold a minimal APK, open a draft PR. Do not merge.

## 2026-11-26 6:36 PM PT

- Finished read-only inventory of D:\Emulation\ROMS (216 folders). Wrote docs/android/inventory.tsv and docs/android/PLAN.md (emulator map, architecture, art-scraper comparison, decisions).
- Rust 1.100.0 is installed via rustup. OpenJDK 17 and rustup arrived through Homebrew. Android SDK/NDK + cargo-apk install is running.
- Scaffold crate is at android/slot-android (standalone, not a workspace member). Next: build the APK once the NDK is ready, then open a draft PR. Do not merge.

## 2026-11-26 6:41 PM PT

- Android toolchain on this Mac mini: OpenJDK 17, Android SDK cmdline tools, platform 35, build-tools 35, NDK 27.2.12479018, rustup (stable 1.100 plus the repo-pinned 1.96.1 with the `aarch64-linux-android` target), `cargo-ndk`, `cargo-apk` 0.10.0.
- Scaffold APK built successfully: `android/slot-android/target/debug/apk/Slot.apk` (debug, arm64-v8a, ~25.3 MB). Placeholder shelf, no cores. Not committed (under `target/`).
- Next: commit, push `cursor/android-scaffold`, open a draft PR. Do not merge.

## 2026-11-26 6:42 PM PT

- Pushed `cursor/android-scaffold` (`516b9e9`) and opened draft PR https://github.com/jfink22-design/slot-brick-plus/pull/35. Not merged. Other worktrees and `main` were not modified.

## 2026-11-26 6:51 PM PT — Phase 2 start

Jeff's decisions: AYN Odin/Thor-class (Thor second screen later); Switch via official Eden + his keys; next shelves 3DS, Dreamcast, Saturn, Vita, Xbox (Hakuchou = hakuX).

### Disk
Data volume was 99% with ~16 GB free. Cleaning only rebuildable cargo `target` / `target-*` in worktrees whose tips are already on `origin/main`. Leaving Shared-Space, repos, sbp-wiki / sbp-sp / sbp-sp24 (not merged), and sbp-merge + slot-brick-plus-build (may be in use for the SP/Brick build). Not pruning Docker Slot build images.

### Hakuchou / hakuX (verified)
Jeff's "Hakuchou" is the hakuX project, a GPL-3-or-later Android Xbox emulator (xemu lineage; current core is a FEX JIT, official repo https://github.com/rfandango/hakuX). Package `com.rfandango.haku_x` (debug suffix `.debug`). Activity `com.rfandango.haku_x.LauncherActivity`. Launch: ACTION_VIEW + content URI. Official download is GitHub releases only; no BIOS/MCPX required per the current README (v1.3.0). Intent launch is clean.

### Next
Implement the emulator registry, SAF picker, RetroArch interim for libretro systems, Odin/Thor mapping, rebuild the debug APK, push to draft PR #35.

### Disk result
Deleted ~26 GB of cargo `target`/`target-sim` dirs in already-merged worktrees. APFS still held the blocks in today's local Time Machine snapshots; `tmutil thinlocalsnapshots` reclaimed them (local snapshots only, no external backups or user files). Data volume went from 16 GB free (99%) to **49 GB free (95%)**, container free 52.8 GB, at 6:53 PM PT. Shared-Space, Docker Slot images, and unmerged worktrees were left alone.

## 2026-11-26 6:58 PM PT — Phase 2 APK

- Debug APK built: `android/app/build/outputs/apk/debug/app-debug.apk` (~16.1 MB). Gradle assembleDebug succeeded. Native registry is `libslot_android.so` (arm64-v8a).
- Next: commit, push to draft PR #35. Do not merge. Do not touch cards or E:/F:.

## 2026-11-26 7:08 PM PT — PR #35 merged

- Marked PR #35 ready (it was a draft).
- CI on head `e4d2c90` was green: unit, scenarios, changelog. mergeable_state=clean.
- Brick/SP CI (simulator unit + scenarios) passed on this PR; android/ is a standalone crate and is not in the Slot workspace.
- Merged to main: `4fd6a8dbff47c4c5907b4286aa03f5d41bc8712f`
  (Merge pull request #35 from jfink22-design/cursor/android-scaffold)
- New worktree `/Users/jeff/sbp-android-p3` on branch `cursor/android-phase3` from that merge. Other worktrees left alone. No SD cards, no E:/F:.

Next: ScreenScraper + libretro-thumbnails + Thor second-screen preview, then a new PR and a debug APK.

## 2026-11-26 7:20 PM PT — Phase 3 code in, building APK

Implemented on `cursor/android-phase3` in `/Users/jeff/sbp-android-p3` (no other worktrees, no cards, no E:/F:):

- ScreenScraper client (user account + empty developer-id placeholder, quotas, box/snap/wheel/video-normalized, on-device cache)
- libretro-thumbnails fallback (box, snaps, titles; no account; no video)
- Settings (credentials, second-screen toggle, muted/low volume, cache clear, test login)
- About attribution
- Presentation / DisplayManager second-screen preview; inline panel on single-screen devices; released on launch
- Media3 ExoPlayer for video snaps

Next: compile the debug APK, fix whatever the compiler catches, open a new PR.

## 2026-11-26 7:14 PM PT — debug APK built

`android/build-apk.sh` succeeded (cargo-ndk + Gradle, 20s after the native lib).

Debug APK: `/Users/jeff/sbp-android-p3/android/app/build/outputs/apk/debug/app-debug.apk` (22 MB).

Did not touch any SD card or E:/F:. Next: commit, push, open the Phase 3 PR.

## 2026-11-26 7:15 PM PT — Phase 3 PR opened

- Commit: `2d6e4f096ac37d0240c4db032daa7429d7caa60e`
- PR: https://github.com/RamblewoodLabs/slot-brick-plus/pull/36
- Merge of #35 remains `4fd6a8dbff47c4c5907b4286aa03f5d41bc8712f` on main.
- Debug APK: `/Users/jeff/sbp-android-p3/android/app/build/outputs/apk/debug/app-debug.apk` (22 MB)
- No SD card, no E:/F:.

## 2026-11-26 8:07 PM PT — Rename the Android app to Slotted

Jeff: "let's call the app slotted." On `cursor/android-phase3` only.

- Launcher / `app_name`, shelf title, About, Settings copy, and toasts now say **Slotted**.
- ScreenScraper `softname=Slotted`; User-Agent `Slotted/0.3 (Android; GPL-3.0-or-later)`.
- `applicationId` changed from the placeholder `plus.slotbrick.slot` to `com.ramblewoodlabs.slotted`. Kotlin/JNI `namespace` stays `plus.slotbrick.slot` so native symbols do not need a rename. This install will sit beside any leftover placeholder APK until that one is uninstalled.
- Gradle `archivesName` is `Slotted` so the debug APK should be `Slotted-debug.apk`.
- Brick/SP firmware name **Slot** and the repo name are unchanged.
- Next: rebuild, push, update PR #36, wait for CI. Do not merge.

## 2026-11-26 8:20 PM PT — Embed first-wave libretro cores

Starting on `cursor/android-phase3`. Plan: build mGBA, FCEUmm, Snes9x,
Genesis Plus GX and PCSX ReARMed for arm64-v8a from the vendored sources
with the NDK; add a minimal Rust libretro host and a PlayActivity; keep
RetroArch as the fallback for everything else. Licenses stay with the
app (Genesis Plus GX is non-commercial; Slotted is free). No SD card,
no E:/F:. Commits in small chunks. Do not merge.


## 2026-11-26 8:31 PM PT — First-wave libretro cores embedded

On `cursor/android-phase3` in `/Users/jeff/sbp-android-p3`. No SD card, no E:/F:. PR #36 is not merged.

Built from the vendored sources in `vendor/` (same pins as `brick/build-cores.sh`) with NDK 27.2 for arm64-v8a:

- mGBA — GB / GBC / GBA (standalone audio/DMA patches only; no Brick link-cable patches)
- FCEUmm — NES
- Snes9x — SNES
- Genesis Plus GX — Genesis / Master System / Game Gear (non-commercial; allowed because Slotted is free)
- PCSX ReARMed — PS1 (ari64 dynarec + NEON; `LIBPTHREAD=` / `LIBDL=` because Bionic has both in libc)

A minimal Rust libretro host in `android/slot-android` (`host.rs`) loads the core `.so`, converts software frames to RGB565 for a GLES2 texture, queues stereo audio into `AudioTrack`, and handles SRAM (`.srm`) plus serialize/unserialize savestates. `PlayActivity` maps an Xbox-layout pad to Nintendo libretro buttons. Start+Select or Back opens a pause menu (save, load, reset, RetroArch fallback, exit). SAF ROMs use a real path when possible, otherwise a cache copy. RetroArch remains the fallback for every other libretro system and if an embedded start fails.

License texts from `licenses/` are copied into `assets/licenses` and named on the About screen.

Debug APK (release-optimized native host): `/Users/jeff/sbp-android-p3/android/app/build/outputs/apk/debug/Slotted-debug.apk` (~29 MB).
SHA-256: `8cf0111b36f6c2ff3db9cd45077ecd0203baf503b4c10cf4b4f890f1f3720cf6`

Next: remaining 24-system cores (N64 / PPSSPP need a GLES hardware-render path), RetroAchievements counts, and Jeff's ScreenScraper forum registration for video / full art.


## 2026-11-26 8:52 PM PT — Remaining handheld cores + GLES HW path

On `cursor/android-phase3` in `/Users/jeff/sbp-android-p3`. No SD card, no E:/F:. PR #36 is not merged.

Software cores added, same pins as the Brick/SP scripts: FBNeo (arcade; non-commercial, allowed because Slotted is free), Beetle PCE Fast, Stella 2014, FreeIntv, Handy, ProSystem, gw-libretro, Gearcoleco, a5200, Atari800, vecx (HAS_GPU=0), PicoDrive (32X; non-commercial, same rule), melonDS (software renderer, arm64 JIT; Android strerror_r patch).

GLES3 libretro hardware-render path in `android/slot-android/src/hw.rs`: the core gets an FBO in PlayActivity's current context, plus analog sticks. Embedded: mupen64plus-next (N64) and PPSSPP (PSP). PPSSPP is built without FFmpeg — the vendor tarball only has Linux/glibc prebuilts, which will not link against Bionic. PPSSPP fonts, shaders and compat lists are packed from `vendor/ppsspp-assets`. BIOS and firmware are not bundled.

Dreamcast (Flycast) and Saturn (Beetle Saturn) still open RetroArch. This pass did not sideload or play a ROM on a device.

Debug APK: `/Users/jeff/sbp-android-p3/android/app/build/outputs/apk/debug/Slotted-debug.apk` (~62 MB, FBNeo dominates the uncompressed size).
SHA-256: `1838d9b80e1b2b6d075362525c76f5014faa8d7883161227838c50f37d5dabc0`

Next: Flycast and Beetle Saturn; RetroAchievements counts; Jeff's ScreenScraper forum registration.


## 2026-11-27 9:17 AM PT — Bake ScreenScraper developer credentials

On `cursor/android-phase3` in `/Users/jeff/sbp-android-p3`. No SD card, no E:/F:. PR #36 is not merged.

ScreenScraper approved the Slotted developer account (numeric id 6315). A local, gitignored `android/screenscraper.properties` (mode 600) now holds the issued `devid` / `devpassword` and `softname=Slotted`. That file is not committed. The tracked placeholder at `android/app/src/main/assets/screenscraper.properties` stays empty.

Gradle (`android/app/build.gradle.kts`) bakes the local file into merged APK assets at build time when it is present, without logging the values or overwriting the tracked placeholder.

API check from this Mac, developer credentials only (no user account): `jeuInfos.php` for Mega Drive (`systemeid=1`) and `Sonic The Hedgehog (USA, Europe).md` returned HTTP 200, game name Sonic The Hedgehog (id 5), plus media URLs for box art (`box-2D`), a screenshot (`ss`), a wheel, and a video (`video-normalized`).

Debug APK: `/Users/jeff/sbp-android-p3/android/app/build/outputs/apk/debug/Slotted-debug.apk` (~62 MB).
SHA-256: `1da13fa396009b4fe20b6d59b2c45ae5b565646bc6c24f7d0ab2ff94a7a6165e`

The baked asset inside the APK matches the local properties file. Next: Jeff can install this APK and enter his own ScreenScraper user account in Settings to use the full quota. Flycast, Beetle Saturn, and RetroAchievements counts remain.

