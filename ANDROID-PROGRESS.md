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
