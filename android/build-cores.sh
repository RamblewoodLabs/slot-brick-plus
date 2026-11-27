#!/usr/bin/env bash
# Build Slotted libretro cores (arm64-v8a) from the vendored sources in
# vendor/, using the Android NDK. Pins and hashes match brick/build-cores.sh,
# brick/build-cores-systems2.sh, and brick/gpu-cores/sources.sha256.
#
# Software cores: mGBA, FCEUmm, Snes9x, Genesis Plus GX, PCSX ReARMed,
# FBNeo, Beetle PCE Fast, Stella 2014, FreeIntv, Handy, ProSystem,
# gw-libretro, Gearcoleco, a5200, Atari800, vecx (HAS_GPU=0), PicoDrive,
# melonDS (software renderer, arm64 JIT).
# GPU cores (libretro HW context, GLES3): mupen64plus-next, PPSSPP.
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
# Bionic has pthread and librt in libc; the NDK has no -lpthread/-lrt.
# Empty archives in this path satisfy makefiles that still name them.
STUBS="$BUILD/android-stubs"
mkdir -p "$STUBS"
if [ ! -f "$STUBS/libpthread.a" ]; then
  "$AR" rcs "$STUBS/libpthread.a"
  "$AR" rcs "$STUBS/librt.a"
  "$AR" rcs "$STUBS/libpthread.so.a" 2>/dev/null || true
fi
# Prefer these stubs whenever a makefile appends -lpthread/-lrt/-ldl.
export LIBRARY_PATH="$STUBS${LIBRARY_PATH:+:$LIBRARY_PATH}"

# Same pins as brick/build-cores.sh — fetch only if the archive is missing,
# then verify the committed sha256.
MGBA=e31759b24e7a4e3899285ff720d7b573ac328ae7
FCEUMM=7a542dab1e87679921962a9f056186eca425c0c2
SNES9X=fae2fea08f74180759ef540ee94259213f503480
GENESIS=099c962e7374eb84cae9bba180c9e0e49e767b40
PCSX=c8816799b50388e61cfe237fe2cdbb7d8175f20a
FBNEO=2e6605c4a08b86455e3aee03294fe35e7618fa91
PCE=4cf066e22533186d0cfad85a0fec2aced93e4e29
STELLA=7d1361e407e63f29e52892655069e5fb4096e691
FREEINTV=ef3e0fe322bec62a7f916c0bb0834c08c348d0b4
HANDY=bc55d462f0b2d6b073ea93dc552ebd73cec60fd1
PROSYSTEM=8a88014287c7a01cd568067e5a557d0a2b2a051f
GW=91d599b951e7bfe7e040347f58667cba20074adc
GEARCOLECO=020740f6a6c778200e7fe1bc6b508be13a4dd582
A5200=40c6f2f1ad4a3145b328d5baaf010fae6c7e752b
ATARI800=4e7fbc73765c1a9670c7506616046ad1d4ccda51
VECX=dccff5402a16d1ea7c112095a8e42632bf149506
PICODRIVE=1890c2932234c9d30f4cd3851d02228baae8f09e
MELONDS=4081e8358e66f1ea11879560be6b813dfeaab75a
MUPEN=8623e490a8773fcb6c10f424cebbdb0ac44e88ea
PPSSPP=v1.20.4

need() {
  local dest="$1" url="$2"
  [ -f "$dest" ] || curl -fL "$url" -o "$dest"
}

need "$ROOT/vendor/mgba-src.tar.gz" "https://github.com/libretro/mgba/archive/$MGBA.tar.gz"
need "$ROOT/vendor/fceumm-src.tar.gz" "https://github.com/libretro/libretro-fceumm/archive/$FCEUMM.tar.gz"
need "$ROOT/vendor/snes9x-src.tar.gz" "https://github.com/libretro/snes9x/archive/$SNES9X.tar.gz"
need "$ROOT/vendor/genesis_plus_gx-src.tar.gz" "https://github.com/libretro/Genesis-Plus-GX/archive/$GENESIS.tar.gz"
need "$ROOT/vendor/pcsx_rearmed-src.tar.gz" "https://github.com/libretro/pcsx_rearmed/archive/$PCSX.tar.gz"
need "$ROOT/vendor/fbneo-src.tar.gz" "https://github.com/libretro/FBNeo/archive/$FBNEO.tar.gz"
need "$ROOT/vendor/mednafen_pce_fast-src.tar.gz" "https://github.com/libretro/beetle-pce-fast-libretro/archive/$PCE.tar.gz"
need "$ROOT/vendor/stella2014-src.tar.gz" "https://github.com/libretro/stella2014-libretro/archive/$STELLA.tar.gz"
need "$ROOT/vendor/freeintv-src.tar.gz" "https://github.com/libretro/FreeIntv/archive/$FREEINTV.tar.gz"
need "$ROOT/vendor/handy-src.tar.gz" "https://github.com/libretro/libretro-handy/archive/$HANDY.tar.gz"
need "$ROOT/vendor/prosystem-src.tar.gz" "https://github.com/libretro/prosystem-libretro/archive/$PROSYSTEM.tar.gz"
need "$ROOT/vendor/gw-libretro-src.tar.gz" "https://github.com/libretro/gw-libretro/archive/$GW.tar.gz"
need "$ROOT/vendor/gearcoleco-src.tar.gz" "https://github.com/drhelius/Gearcoleco/archive/$GEARCOLECO.tar.gz"
need "$ROOT/vendor/a5200-src.tar.gz" "https://github.com/libretro/a5200/archive/$A5200.tar.gz"
need "$ROOT/vendor/atari800-src.tar.gz" "https://github.com/libretro/libretro-atari800/archive/$ATARI800.tar.gz"
need "$ROOT/vendor/vecx-src.tar.gz" "https://github.com/libretro/libretro-vecx/archive/$VECX.tar.gz"
need "$ROOT/vendor/melonds-src.tar.gz" "https://github.com/libretro/melonDS/archive/$MELONDS.tar.gz"
# PicoDrive, mupen64plus-next and PPSSPP tarballs are committed (PicoDrive is a
# recursive checkout because GitHub's archive drops submodules).
[ -f "$ROOT/vendor/picodrive-src.tar.gz" ] || { echo "missing vendor/picodrive-src.tar.gz" >&2; exit 1; }
[ -f "$ROOT/vendor/mupen64plus_next-src.tar.gz" ] || { echo "missing vendor/mupen64plus_next-src.tar.gz" >&2; exit 1; }
[ -f "$ROOT/vendor/ppsspp-src.tar.gz" ] || { echo "missing vendor/ppsspp-src.tar.gz" >&2; exit 1; }

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
verify "$ROOT/vendor/fbneo-src.tar.gz"            cb1ac0e28a69ba01826be5dee41dbeb842e04b36a23bd14b28fcc5b83af4a411
verify "$ROOT/vendor/mednafen_pce_fast-src.tar.gz" 6832f9c4b3b960d30f9019d54a64224a0cd4717b18cc45eb0e7ba65115d39934
verify "$ROOT/vendor/stella2014-src.tar.gz"       c0f5e53f3c21e7760e434784634d44a02e57a30917da5279cfd3add4736afd82
verify "$ROOT/vendor/freeintv-src.tar.gz"         912d0a9c314cc63c396f8bc5a5778c341a3f4f860ebae1e069a877b9c98d0f11
verify "$ROOT/vendor/handy-src.tar.gz"            65ad333df22aab7f3c8156c21ffd0b8da1ef09c7a7f1775df964b89986aa7324
verify "$ROOT/vendor/prosystem-src.tar.gz"        23401c8b104ef24db111bbc4b13e37f224ffe16febfb89c704c70cdb269603bc
verify "$ROOT/vendor/gw-libretro-src.tar.gz"      88f47e3fe2ce7a5cdf42a087386a6cff5379dd2fe7901761c80a9f8d0d2185ad
verify "$ROOT/vendor/gearcoleco-src.tar.gz"       77796cb35d9416b305812b269275d0be7f3cda23581a96352420639c148597d4
verify "$ROOT/vendor/a5200-src.tar.gz"            1b1c382028a188f58f3e1cb7e0f62c926c754824659612703647ce81b9fd4d9e
verify "$ROOT/vendor/atari800-src.tar.gz"         475f072f17059a23f402ee2744c00476aba5b7c819b1af3b38c933e45bdf2947
verify "$ROOT/vendor/vecx-src.tar.gz"             684808db7e9d3a81647e25e8a7e34df3e0f62ed317df578143055c6536d23a32
verify "$ROOT/vendor/picodrive-src.tar.gz"        6f87d7b878bd8070d0a244d2eb039a51755d0ba701a7709e3502716f95487a27
verify "$ROOT/vendor/melonds-src.tar.gz"          99950fba5385d03581f4416320ff1ba11f9f5675a1bfbca6605ff0e695b6c60a
verify "$ROOT/vendor/mupen64plus_next-src.tar.gz" e754c9632016485381515916d04c633dd6662e5305e6971ea04f650fff6f000b
verify "$ROOT/vendor/ppsspp-src.tar.gz"           30595d59d878ff87af23f2b47ba0b05a7ea96b6510160d7fb8b2fcd10f3bdf00

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
fbneo_src="$SRC/FBNeo-$FBNEO"
pce_src="$SRC/beetle-pce-fast-libretro-$PCE"
stella_src="$SRC/stella2014-libretro-$STELLA"
freeintv_src="$SRC/FreeIntv-$FREEINTV"
handy_src="$SRC/libretro-handy-$HANDY"
prosystem_src="$SRC/prosystem-libretro-$PROSYSTEM"
gw_src="$SRC/gw-libretro-$GW"
coleco_src="$SRC/Gearcoleco-$GEARCOLECO"
a5200_src="$SRC/a5200-$A5200"
a800_src="$SRC/libretro-atari800-$ATARI800"
vecx_src="$SRC/libretro-vecx-$VECX"
pico_src="$SRC/picodrive-$PICODRIVE"
melonds_src="$SRC/melonDS-$MELONDS"
mupen_src="$SRC/mupen64plus-libretro-nx-$MUPEN"
ppsspp_src="$SRC/ppsspp-$PPSSPP"

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
extract "$ROOT/vendor/fbneo-src.tar.gz" "$fbneo_src"
extract "$ROOT/vendor/mednafen_pce_fast-src.tar.gz" "$pce_src"
extract "$ROOT/vendor/stella2014-src.tar.gz" "$stella_src"
extract "$ROOT/vendor/freeintv-src.tar.gz" "$freeintv_src"
extract "$ROOT/vendor/handy-src.tar.gz" "$handy_src"
extract "$ROOT/vendor/prosystem-src.tar.gz" "$prosystem_src"
extract "$ROOT/vendor/gw-libretro-src.tar.gz" "$gw_src"
extract "$ROOT/vendor/gearcoleco-src.tar.gz" "$coleco_src"
extract "$ROOT/vendor/a5200-src.tar.gz" "$a5200_src"
extract "$ROOT/vendor/atari800-src.tar.gz" "$a800_src"
extract "$ROOT/vendor/vecx-src.tar.gz" "$vecx_src"
extract "$ROOT/vendor/picodrive-src.tar.gz" "$pico_src"
extract "$ROOT/vendor/melonds-src.tar.gz" "$melonds_src"
if [ ! -f "$melonds_src/.slotted-patched" ]; then
  # Bionic strerror_r is the GNU variant; melonDS assumed XSI because __GLIBC__ is unset.
  patch -d "$melonds_src" -p1 --forward --batch < "$ANDROID_DIR/patches/melonds-android-strerror.patch"
  touch "$melonds_src/.slotted-patched"
fi
extract "$ROOT/vendor/mupen64plus_next-src.tar.gz" "$mupen_src"
extract "$ROOT/vendor/ppsspp-src.tar.gz" "$ppsspp_src"

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
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS" \
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

# unix + this NDK clang is the same family as the Brick's unix/aarch64
# targets, without pinning Cortex-A53. Bionic has pthread/dl in libc.
MAKE_UNIX=(
  CC="$CC"
  CXX="$CXX"
  AR="$AR"
  RANLIB="$RANLIB"
  platform=unix
  LIBPTHREAD=
  LIBDL=
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS"
  -j"$JOBS"
)

echo "==== FBNeo (arcade) ===="
# Non-commercial license; Slotted is free, so redistribution is allowed.
make -C "$fbneo_src/src/burner/libretro" -f Makefile \
  CC="$CC" CXX="$CXX" AR="$AR" RANLIB="$RANLIB" \
  platform=unix LIBPTHREAD= LIBDL= \
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS" -j"$JOBS"

echo "==== Beetle PCE Fast (TurboGrafx-16 / PC Engine) ===="
make -C "$pce_src" "${MAKE_UNIX[@]}"

echo "==== Stella 2014 (Atari 2600) ===="
make -C "$stella_src" "${MAKE_UNIX[@]}"

echo "==== FreeIntv (Intellivision) ===="
make -C "$freeintv_src" "${MAKE_UNIX[@]}"

echo "==== Handy (Atari Lynx) ===="
# Handy's makefile links with g++ and ignores LDFLAGS.
make -C "$handy_src" platform=unix \
  CC="$CC" CXX="$CXX -static-libstdc++" AR="$AR" \
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS" \
  LIBPTHREAD= LIBDL= -j"$JOBS"

echo "==== ProSystem (Atari 7800) ===="
make -C "$prosystem_src" "${MAKE_UNIX[@]}"

echo "==== gw-libretro (Game & Watch) ===="
make -C "$gw_src" platform=unix CC="$CC" AR="$AR" \
  LDFLAGS="-shared -lm -llog -L$STUBS" LIBPTHREAD= LIBDL= -j"$JOBS"

echo "==== Gearcoleco (ColecoVision) ===="
make -C "$coleco_src/platforms/libretro" "${MAKE_UNIX[@]}"

echo "==== a5200 (Atari 5200) ===="
make -C "$a5200_src" "${MAKE_UNIX[@]}"

echo "==== Atari800 (Atari 800) ===="
make -C "$a800_src" "${MAKE_UNIX[@]}"

echo "==== vecx (Vectrex, software renderer) ===="
make -C "$vecx_src" -f Makefile.libretro HAS_GPU=0 "${MAKE_UNIX[@]}"

echo "==== PicoDrive (32X) ===="
# aarch64 enables PicoDrive's SH-2 DRC. Non-commercial; Slotted is free.
make -C "$pico_src" -f Makefile.libretro platform=aarch64 \
  CC="$CC" CXX="$CXX" AR="$AR" \
  LDFLAGS="-shared -lm -llog -L$STUBS" LIBPTHREAD= LIBDL= -j"$JOBS"

echo "==== melonDS (Nintendo DS, software renderer) ===="
# ARCH=arm64 turns on the aarch64 JIT; no GL, matching the Brick build.
make -C "$melonds_src" platform=unix ARCH=arm64 \
  CC="$CC" CXX="$CXX -static-libstdc++" AR="$AR" \
  GIT_VERSION=" ${MELONDS%"${MELONDS#???????}"}" \
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS" \
  LIBPTHREAD= LIBDL= -j"$JOBS"

echo "==== mupen64plus-next (N64, GLES3) ===="
make -C "$mupen_src" platform=unix FORCE_GLES3=1 WITH_DYNAREC=aarch64 ARCH=aarch64 \
  CC="$CC" CXX="$CXX" AR="$AR" \
  GL_LIB="-lGLESv3 -lEGL" \
  LDFLAGS="-shared -static-libstdc++ -lm -llog -L$STUBS" \
  LIBPTHREAD= LIBDL= \
  -j"$JOBS"

echo "==== PPSSPP (PSP, GLES) ===="
# Asset folders the libretro core does not use may be absent from the tarball
# even though CMake still copies them; create them empty.
for d in debugger upload lang shaders themes vfpu; do
  mkdir -p "$ppsspp_src/assets/$d"
done
cmake -S "$ppsspp_src" -B "$BUILD/ppsspp" \
  -DCMAKE_TOOLCHAIN_FILE="$ANDROID_NDK_ROOT/build/cmake/android.toolchain.cmake" \
  -DANDROID_ABI="$ABI" \
  -DANDROID_PLATFORM="android-$API" \
  -DANDROID_STL=c++_static \
  -DLIBRETRO=ON -DUSING_GLES2=ON -DCMAKE_BUILD_TYPE=Release \
  -DUSE_DISCORD=OFF -DUSE_MINIUPNPC=OFF -DUSE_FFMPEG=OFF \
  -DUSING_X11_VULKAN=OFF -DCMAKE_POLICY_VERSION_MINIMUM=3.5
# The vendored tarball only includes Linux aarch64 FFmpeg prebuilts (glibc).
# Those will not link against Bionic, and official android/arm64 prebuilts
# are not in vendor/ppsspp-src.tar.gz, so this Android build of PPSSPP
# ships without FFmpeg (in-game cinematics that need it may be silent).
cmake --build "$BUILD/ppsspp" --target ppsspp_libretro --parallel "$JOBS"

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
install_so "$fbneo_src/src/burner/libretro/fbneo_libretro.so" fbneo
install_so "$pce_src/mednafen_pce_fast_libretro.so" mednafen_pce_fast
install_so "$stella_src/stella2014_libretro.so" stella2014
install_so "$freeintv_src/freeintv_libretro.so" freeintv
install_so "$handy_src/handy_libretro.so" handy
install_so "$prosystem_src/prosystem_libretro.so" prosystem
install_so "$gw_src/gw_libretro.so" gw
install_so "$coleco_src/platforms/libretro/gearcoleco_libretro.so" gearcoleco
install_so "$a5200_src/a5200_libretro.so" a5200
install_so "$a800_src/atari800_libretro.so" atari800
install_so "$vecx_src/vecx_libretro.so" vecx
install_so "$pico_src/picodrive_libretro.so" picodrive
install_so "$melonds_src/melonds_libretro.so" melonds
install_so "$mupen_src/mupen64plus_next_libretro.so" mupen64plus_next
PPSSPP_SO="$(find "$BUILD/ppsspp" "$ppsspp_src" \
  \( -name 'ppsspp_libretro.so' -o -name 'libppsspp_libretro.so' -o -name 'ppsspp_libretro_android.so' \) | head -1)"
install_so "$PPSSPP_SO" ppsspp

echo
echo "Embedded cores in $OUT:"
ls -lh "$OUT"/lib*_libretro.so
