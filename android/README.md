# Slot for Android (scaffold)

This folder holds the first Android APK for Slot. The app shows a placeholder
shelf with sample systems. It does not yet load ROMs or emulator cores.

The plan, inventory and art-scraper notes live in [`../docs/android/PLAN.md`](../docs/android/PLAN.md).

## Toolchain on the Mac mini

Already installed (or installed by the scaffold work) under Jeff's account:

- Rust stable via rustup (`~/.cargo`, `~/.rustup`), target `aarch64-linux-android`
- OpenJDK 17 (`/opt/homebrew/opt/openjdk@17`)
- Android command-line tools, platform 35, build-tools 35, NDK 27.2
  (`/opt/homebrew/share/android-commandlinetools`)
- `cargo-ndk` and `cargo-apk`

```bash
export JAVA_HOME=/opt/homebrew/opt/openjdk@17
export PATH="$JAVA_HOME/bin:$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.2.12479018"
```

## Build

From the repo root, or from this folder:

```bash
android/build-apk.sh
```

The debug APK is written to `android/slot-android/target/debug/apk/Slot.apk`
(about 26 MB unoptimized; printed at the end of the script). Sideload it with
`adb install -r android/slot-android/target/debug/apk/Slot.apk`.

This crate is **not** a workspace member of the top-level `Cargo.toml`, so it
does not affect Brick or SP CI.
