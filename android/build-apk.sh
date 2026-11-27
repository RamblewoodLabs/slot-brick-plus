#!/usr/bin/env bash
# Build the Slot Android scaffold APK on the Mac mini.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:$HOME/.cargo/bin:/opt/homebrew/opt/rustup/bin:/opt/homebrew/bin:$PATH"
export ANDROID_HOME="${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}"
export ANDROID_SDK_ROOT="${ANDROID_SDK_ROOT:-$ANDROID_HOME}"
if [ -z "${ANDROID_NDK_ROOT:-}" ]; then
  if [ -d "$ANDROID_HOME/ndk/27.2.12479018" ]; then
    export ANDROID_NDK_ROOT="$ANDROID_HOME/ndk/27.2.12479018"
  else
    # pick the newest installed NDK
    export ANDROID_NDK_ROOT="$(ls -d "$ANDROID_HOME"/ndk/* 2>/dev/null | sort | tail -1)"
  fi
fi
if [ ! -d "${ANDROID_NDK_ROOT:-}" ]; then
  echo "ANDROID_NDK_ROOT is not set and no NDK was found under $ANDROID_HOME/ndk" >&2
  exit 1
fi
echo "JAVA_HOME=$JAVA_HOME"
echo "ANDROID_HOME=$ANDROID_HOME"
echo "ANDROID_NDK_ROOT=$ANDROID_NDK_ROOT"
rustc --version
cd "$ROOT/android/slot-android"
cargo apk build --target aarch64-linux-android
echo
echo "APK(s):"
find "$ROOT/android/slot-android/target" -name '*.apk' -print
