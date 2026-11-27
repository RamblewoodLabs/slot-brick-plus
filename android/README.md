# Slot for Android

This folder holds the Slot Android app. The debug APK can pick Jeff's ROM
tree, list games by folder, detect the standalone emulators, launch them
(or RetroArch, for now, for libretro systems), and prompt to install a
missing emulator.

The plan, inventory and art-scraper notes live in
[`../docs/android/PLAN.md`](../docs/android/PLAN.md).

## Toolchain on the Mac mini

- Rust via rustup (`~/.cargo`, `~/.rustup`), including `aarch64-linux-android`
  on the repo-pinned 1.96.1 toolchain
- OpenJDK 17 (`/opt/homebrew/opt/openjdk@17`)
- Android command-line tools, platform 35, build-tools 35, NDK 27.2
  (`/opt/homebrew/share/android-commandlinetools`)
- `cargo-ndk` and the Gradle wrapper under `android/`

```bash
export JAVA_HOME=/opt/homebrew/opt/openjdk@17
export PATH="$JAVA_HOME/bin:$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:$PATH"
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.2.12479018"
```

## Build

From the repo root:

```bash
android/build-apk.sh
```

The debug APK is written to:

`android/app/build/outputs/apk/debug/app-debug.apk`

## Sideload on an Odin / Thor

1. On the handheld: Settings → About, tap Build number 7 times, then enable
   USB debugging (and “Install via USB” if the device has that switch).
2. Plug it into the Mac mini (or any machine with `adb`).
3. `adb devices` should show the pad.
4. `adb install -r android/app/build/outputs/apk/debug/app-debug.apk`
5. Open Slot, choose the ROM folder that contains `gc`, `PS2`, `Switch`,
   `3DS`, `dreamcast`, `saturn`, `psvita`, `xbox`, and the older system
   folders.
6. Install the standalone emulators Slot asks for (Dolphin, ARMSX2, Eden,
   Azahar, Vita3K, hakuX) from their official pages. For the 24 older
   systems plus Dreamcast and Saturn, install RetroArch and the named core
   from RetroArch’s updater. This is interim until Slot embeds those cores.
7. For Switch, install official Eden and supply your own `prod.keys` and
   firmware, dumped from a Switch you own. Slot never bundles them.

The Rust crate under `android/slot-android` is **not** a member of the
top-level Cargo workspace, so it does not affect Brick or SP CI.
