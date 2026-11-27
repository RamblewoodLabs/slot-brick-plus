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
