# Slotted

This folder holds **Slotted**, the Android app. The Brick and RG35XX SP firmware stays **Slot**, and the repository name is unchanged. The application id is `com.ramblewoodlabs.slotted` (the previous placeholder was `plus.slotbrick.slot`, so this build installs as a new app — uninstall the old one if it is still on the device). The debug APK can pick Jeff's ROM
tree, list games by folder, detect the standalone emulators, and launch them.
The 24 handheld systems (including N64, DS and PSP) play inside Slotted.
Dreamcast and Saturn still open RetroArch until Flycast and Beetle Saturn
are embedded. Missing standalone emulators get an install prompt.

The plan, inventory and art-scraper notes live in
[`../docs/android/PLAN.md`](../docs/android/PLAN.md).

## Toolchain on the Mac mini

- Rust via rustup (`~/.cargo`, `~/.rustup`), including `aarch64-linux-android`
  on the repo-pinned 1.96.1 toolchain
- OpenJDK 17 (`/opt/homebrew/opt/openjdk@17`)
- Android command-line tools, platform 35, build-tools 35, NDK 27.2
  (`/opt/homebrew/share/android-commandlinetools`)
- `cargo-ndk` and the Gradle wrapper under `android/`
- CMake and Ninja (`brew install cmake ninja`) to build the mGBA core

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

`android/app/build/outputs/apk/debug/Slotted-debug.apk`

## Sideload on an Odin / Thor

1. On the handheld: Settings → About, tap Build number 7 times, then enable
   USB debugging (and “Install via USB” if the device has that switch).
2. Plug it into the Mac mini (or any machine with `adb`).
3. `adb devices` should show the pad.
4. `adb install -r android/app/build/outputs/apk/debug/Slotted-debug.apk`
5. Open Slotted, choose the ROM folder that contains `gc`, `PS2`, `Switch`,
   `3DS`, `dreamcast`, `saturn`, `psvita`, `xbox`, and the older system
   folders.
6. Install the standalone emulators Slotted asks for (Dolphin, ARMSX2, Eden,
   Azahar, Vita3K, hakuX) from their official pages. The 24 handheld systems
   (including N64, DS and PSP) play inside Slotted. For Dreamcast and Saturn,
   install RetroArch and the named core from RetroArch’s updater until those
   cores are embedded too.
7. For Switch, install official Eden and supply your own `prod.keys` and
   firmware, dumped from a Switch you own. Slotted never bundles them.

The Rust crate under `android/slot-android` is **not** a member of the
top-level Cargo workspace, so it does not affect Brick or SP CI.

## Art, video, and the second screen

Hover a game (gamepad highlight, or a first tap — a second tap launches). After
a short delay:

- On an AYN Thor, or any device with a second display, and with **Use the
  second screen** on in Settings, the video snap plays there (muted by default,
  or at low volume if you flip that switch). If there is no video, a screenshot
  or box art is shown with the title, year, publisher, players, and a
  RetroAchievements placeholder.
- On a single-screen device the same preview appears as a panel on the shelf.
- Launching a game dismisses the second-screen Presentation so the external
  emulator can use that display.

### ScreenScraper (videos and the full art set)

Slotted does **not** ship a ScreenScraper developer id. Jeff needs to:

1. Register on the [ScreenScraper forum](https://www.screenscraper.fr/) and
   present Slotted as a free GPL-3 frontend (`softname=Slotted`).
2. Enter the issued `devid` / `devpassword` in Settings, plus his own
   ScreenScraper username and password.
3. Optionally, copy `android/screenscraper.properties.example` to
   `android/screenscraper.properties` (gitignored) and rebuild, so the
   developer id is the APK default. Leave the fields empty until then — do
   not invent credentials.

Until that id is in place, Slotted still fetches **stills** (box art, snaps,
titles) from [libretro-thumbnails](https://github.com/libretro-thumbnails/libretro-thumbnails)
with no account. Video snaps only come from ScreenScraper. Attribution is
on the About screen. Quotas and thread limits reported by the API are
respected.

## Embedded cores

`android/build-cores.sh` (also invoked by `android/build-apk.sh`) builds the
24 handheld cores from the vendored sources in `vendor/` with the NDK, using
the same pins as the Brick/SP scripts. Software-rendered systems play through
the in-process host; N64 (mupen64plus-next) and PSP (PPSSPP) use a GLES3
libretro hardware-render path.

Start+Select or Back pauses (save/load state, reset, or hand off to RetroArch).
Dreamcast (Flycast) and Saturn (Beetle Saturn) still open RetroArch.

Genesis Plus GX, FinalBurn Neo and PicoDrive are non-commercial; shipping
them is allowed because Slotted is free. License texts are copied into the APK from `licenses/`. This Android build of
PPSSPP is without FFmpeg (the vendor tarball only has Linux prebuilts). Building mGBA
and PPSSPP requires CMake (`brew install cmake ninja`).
