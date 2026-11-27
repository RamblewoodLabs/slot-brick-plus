#!/usr/bin/env bash
# Build the first-wave Slotted libretro cores (arm64-v8a) from the vendored
# sources in vendor/, using the Android NDK. These replace the RetroArch
# hand-off for GB/GBC/GBA (mGBA), NES (FCEUmm), SNES (Snes9x), Genesis /
# SMS / Game Gear (Genesis Plus GX), and PS1 (PCSX ReARMed).
#
# Output: android/app/src/main/jniLibs/arm64-v8a/lib<core>_libretro.so
# (jniLibs is gitignored; run this from android/build-apk.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ANDROID_DIR="$ROOT/android"
export JAVA_HOME="${JAVA_HOME:-/opt/homebrew/opt/openjdk@17}"
export PATH="$JAVA_HOME/bin:/opt/homebrew/bin:$PATH"
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

HOST_PREBUILT=""
for candidate in darwin-arm64 darwin-aarch64 darwin-x86_64 linux-x86_64; do
  if [ -d "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$candidate" ]; then
    HOST_PREBUILT="$candidate"
    break
  fi
done
if [ -z "$HOST_PREBUILT" ]; then
  HOST_PREBUILT="$(ls -d "$ANDROID_NDK_ROOT"/toolchains/llvm/prebuilt/* 2>/dev/null | head -1 | xargs -n1 basename)"
fi
TOOLCHAIN="$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/$HOST_PREBUILT"
CC="$TOOLCHAIN/bin/aarch64-linux-android28-clang"
CXX="$TOOLCHAIN/bin/aarch64-linux-android28-clang++"
AR="$TOOLCHAIN/bin/llvm-ar"
RANLIB="$TOOLCHAIN/bin/llvm-ranlib"
if [ ! -x "$CC" ]; then
  echo "missing $CC" >&2
  exit 1
fi
# mGBA is built with CMake + Ninja (or the CMake Makefile generator).
if ! command -v cmake >/dev/null 2>&1; then
  echo "cmake is required to build mGBA. On this Mac: brew install cmake ninja" >&2
  exit 1
fi

API=28
ABI=arm64-v8a
JOBS="$(sysctl -n hw.ncpu 2>/dev/null || nproc)"
SRC="$ANDROID_DIR/core-src"
BUILD="$ANDROID_DIR/core-build"
OUT="$ANDROID_DIR/app/src/main/jniLibs/$ABI"
mkdir -p "$SRC" "$BUILD" "$OUT"

# Same pins as brick/build-cores.sh — fetch only if the archive is missing,
# then verify the committed sha256.
MGBA=e31759b24e7a4e3899285ff720d7b573ac328ae7
FCEUMM=7a542dab1e87679921962a9f056186eca425c0c2
SNES9X=fae2fea08f74180759ef540ee94259213f503480
GENESIS=099c962e7374eb84cae9bba180c9e0e49e767b40
PCSX=c8816799b50388e61cfe237fe2cdbb7d8175f20a

need() {
  local dest="$1" url="$2"
  [ -f "$dest" ] || curl -fL "$url" -o "$dest"
}

need "$ROOT/vendor/mgba-src.tar.gz" "https://github.com/libretro/mgba/archive/$MGBA.tar.gz"
need "$ROOT/vendor/fceumm-src.tar.gz" "https://github.com/libretro/libretro-fceumm/archive/$FCEUMM.tar.gz"
need "$ROOT/vendor/snes9x-src.tar.gz" "https://github.com/libretro/snes9x/archive/$SNES9X.tar.gz"
need "$ROOT/vendor/genesis_plus_gx-src.tar.gz" "https://github.com/libretro/Genesis-Plus-GX/archive/$GENESIS.tar.gz"
need "$ROOT/vendor/pcsx_rearmed-src.tar.gz" "https://github.com/libretro/pcsx_rearmed/archive/$PCSX.tar.gz"

# On macOS, shasum -a 256 is the analogue of sha256sum -c.
verify() {
  local file="$1" expect="$2"
  local got
  got="$(shasum -a 256 "$file" | awk '{print $1}')"
  if [ "$got" != "$expect" ]; then
    echo "sha256 mismatch for $file" >&2
    echo "  expected $expect" >&2
    echo "  got      $got" >&2
    exit 1
  fi
}
verify "$ROOT/vendor/mgba-src.tar.gz"            396d749cce8fe3358b29cbb1db479b1816a151bd688ee45b1d241503cbc40243
verify "$ROOT/vendor/fceumm-src.tar.gz"          f80b5c1df39e22d78791b1867ea79f3ec8c0823e4b1f82b3f89e432c614a6c53
verify "$ROOT/vendor/snes9x-src.tar.gz"          0d4b0c4181d66668ec0040eb17f90a559f7b7fa6cbd9198593c3bdbe79acd029
verify "$ROOT/vendor/genesis_plus_gx-src.tar.gz" 5146e86c16a0e7cf32206533434880f0b45cf294c0575a97ace7bda0e338d219
verify "$ROOT/vendor/pcsx_rearmed-src.tar.gz"    edeb21fdeef8815521cb9d4488d2cd35746607c968ca9b26cce4e42c09f2dc99

extract() {
  local archive="$1" dir="$2"
  if [ ! -d "$dir" ]; then
    tar --no-same-owner -xzf "$archive" -C "$SRC"
  fi
}

mgba_src="$SRC/mgba-$MGBA"
fceumm_src="$SRC/libretro-fceumm-$FCEUMM"
snes9x_src="$SRC/snes9x-$SNES9X"
genesis_src="$SRC/Genesis-Plus-GX-$GENESIS"
pcsx_src="$SRC/pcsx_rearmed-$PCSX"

extract "$ROOT/vendor/mgba-src.tar.gz" "$mgba_src"
if [ ! -f "$mgba_src/.slotted-patched" ]; then
  # Apply only the standalone audio/DMA fixes. The zz-link-* patches are for
  # Slot's two-player cable on the handhelds and are not used in Slotted yet.
  # Use patch(1) rather than git -C: this tree is not a git repo, and git -C
  # would walk up into the worktree and apply against the wrong root.
  for p in       "$ROOT/cores/mgba/685023e05-fifo-dma-source-offset.patch"       "$ROOT/cores/mgba/gb-audio-capacitor.patch"       "$ROOT/cores/mgba/gb-audio-stopped-channel.patch"
  do
    patch -d "$mgba_src" -p1 --forward --batch < "$p"
  done
  touch "$mgba_src/.slotted-patched"
fi
extract "$ROOT/vendor/fceumm-src.tar.gz" "$fceumm_src"
extract "$ROOT/vendor/snes9x-src.tar.gz" "$snes9x_src"
extract "$ROOT/vendor/genesis_plus_gx-src.tar.gz" "$genesis_src"
extract "$ROOT/vendor/pcsx_rearmed-src.tar.gz" "$pcsx_src"

export CC CXX AR RANLIB
# libretro "unix" targets are fine with the Android clang (it already
# produces aarch64-linux-android shared objects). -fPIC is required.
MAKE_COMMON=(
  CC="$CC"
  CXX="$CXX"
  AR="$AR"
  RANLIB="$RANLIB"
  -j"$JOBS"
)

echo "==== mGBA (GB / GBC / GBA) ===="
cmake -S "$mgba_src" -B "$BUILD/mgba" \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_ROOT/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI="$ABI" \
  -DANDROID_PLATFORM="android-$API" \
  -DANDROID_STL=c++_static \
  -DLIBMGBA_ONLY=ON -DBUILD_LIBRETRO=ON -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_DISABLE_FIND_PACKAGE_PNG=ON -DCMAKE_DISABLE_FIND_PACKAGE_ZLIB=ON \
  -DCMAKE_DISABLE_FIND_PACKAGE_LibZip=ON -DCMAKE_POLICY_VERSION_MINIMUM=3.5
cmake --build "$BUILD/mgba" --target mgba_libretro --parallel "$JOBS"

echo "==== FCEUmm (NES) ===="
make -C "$fceumm_src" -f Makefile.libretro platform=unix "${MAKE_COMMON[@]}"

echo "==== Snes9x (SNES) ===="
make -C "$snes9x_src/libretro" platform=unix \
  CC="$CC" CXX="$CXX" AR="$AR" \
  LDFLAGS="-static-libstdc++" \
  -j"$JOBS"

echo "==== Genesis Plus GX (Genesis / SMS / GG) ===="
# Non-commercial license; Slotted is free, so redistribution is allowed.
make -C "$genesis_src" -f Makefile.libretro platform=unix "${MAKE_COMMON[@]}"

echo "==== PCSX ReARMed (PS1) ===="
# arm64: ari64 dynarec + NEON renderer, same family as the Brick's h5 build
# but without pinning Cortex-A53.
# pthread and libdl live in Bionic libc on Android; -lpthread / -ldl fail
# with "unable to find library" under the NDK lld.
make -C "$pcsx_src" -f Makefile.libretro platform=arm64 \
  GIT_VERSION="${PCSX%"${PCSX#???????}"}" \
  CC="$CC" CXX="$CXX" AR="$AR" \
  LIBPTHREAD= LIBDL= \
  -j"$JOBS"

install_so() {
  local src="$1" name="$2"
  local dest="$OUT/lib${name}_libretro.so"
  if [ ! -f "$src" ]; then
    echo "missing $src" >&2
    exit 1
  fi
  cp "$src" "$dest"
  if [ -x "$TOOLCHAIN/bin/llvm-strip" ]; then
    "$TOOLCHAIN/bin/llvm-strip" --strip-unneeded "$dest"
  fi
  echo "installed $dest ($(wc -c < "$dest") bytes)"
}

# mGBA cmake output path can vary slightly; find the .so.
MGBA_SO="$(find "$BUILD/mgba" -name 'mgba_libretro.so' -o -name 'libmgba_libretro.so' | head -1)"
install_so "$MGBA_SO" mgba
install_so "$fceumm_src/fceumm_libretro.so" fceumm
install_so "$snes9x_src/libretro/snes9x_libretro.so" snes9x
install_so "$genesis_src/genesis_plus_gx_libretro.so" genesis_plus_gx
install_so "$pcsx_src/pcsx_rearmed_libretro.so" pcsx_rearmed

echo
echo "Embedded cores in $OUT:"
ls -lh "$OUT"/lib*_libretro.so
