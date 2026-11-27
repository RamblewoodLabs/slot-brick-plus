#!/usr/bin/env bash
# Build the Slotted Android debug APK on the Mac mini.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID_DIR="$ROOT/android"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
if [ -z "${ANDROID_NDK_ROOT:-}" ]; then
  if [ -d "$ANDROID_HOME/ndk/27.2.12479018" ]; then
    export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.2.12479018"
  else
    export ANDROID_NDK_ROOT="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort | tail -1)"
  fi
fi
if [ ! -d "${ANDROID_NDK_ROOT:-}" ]; then
  echo "ANDROID_NDK_ROOT is not set and no NDK was found under $ANDROID_HOME/ndk" >&2
  exit 1
fi
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"
echo "JAVA_HOME=$JAVA_HOME"
echo "ANDROID_HOME=$ANDROID_HOME"
echo "ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"
rustc --version

# Tell Gradle where the SDK lives.
printf 'sdk.dir=%s\n' "$ANDROID_HOME" > "$ANDROID_DIR/local.properties"

echo "==== libretro cores (arm64-v8a) ===="
"$ANDROID_DIR/build-cores.sh"
# License texts for the embedded cores, copied from the repo's licenses/ folder.
mkdir -p "$ANDROID_DIR/app/src/main/assets/licenses"
cp "$ROOT/licenses/mgba-MPL-2.0.txt" \
   "$ROOT/licenses/fceumm-GPL-2.0.txt" \
   "$ROOT/licenses/snes9x-LICENSE.txt" \
   "$ROOT/licenses/genesis-plus-gx-LICENSE.txt" \
   "$ROOT/licenses/pcsx_rearmed-GPL-2.0.txt" \
   "$ROOT/licenses/GPL-3.0.txt" \
   "$ROOT/licenses/fbneo-LICENSE.txt" \
   "$ROOT/licenses/handy-zlib.txt" \
   "$ROOT/licenses/gw-libretro-zlib.txt" \
   "$ROOT/licenses/picodrive-COPYING.txt" \
   "$ANDROID_DIR/app/src/main/assets/licenses/"
# PPSSPP's own assets (not BIOS). Rebuilt from vendor/ each APK; gitignored.
if [ -d "$ROOT/vendor/ppsspp-assets" ]; then
  rm -rf "$ANDROID_DIR/app/src/main/assets/ppsspp"
  mkdir -p "$ANDROID_DIR/app/src/main/assets/ppsspp"
  cp -R "$ROOT/vendor/ppsspp-assets/." "$ANDROID_DIR/app/src/main/assets/ppsspp/"
fi

echo "==== native lib (arm64-v8a) ===="
cd "$ANDROID_DIR/slot-android"
# cargo-ndk writes jniLibs layout under -o
mkdir -p "$ANDROID_DIR/app/src/main/jniLibs"
cargo ndk -t arm64-v8a -o "$ANDROID_DIR/app/src/main/jniLibs" build --release

echo "==== gradle assembleDebug ===="
cd "$ANDROID_DIR"
./gradlew :app:assembleDebug --no-daemon

echo
echo "APK(s):"
find "$ANDROID_DIR/app/build/outputs/apk" -name '*.apk' -print
