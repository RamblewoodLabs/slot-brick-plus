#!/bin/sh
# ColecoVision, Atari 5200, Atari 800, Vectrex, Sega 32X and Nintendo DS cores.
# Same toolchain and flags as brick/build-cores.sh (which runs this at its end), so
# both the Brick card and the RG35XX SP card get them.
set -eu
: "${BRICK_TOOLCHAIN:?Set BRICK_TOOLCHAIN to the extracted TrimUI toolchain directory}"
cd "$(dirname "$0")/.."
export PATH="$BRICK_TOOLCHAIN/bin:$PATH"
GEARCOLECO=020740f6a6c778200e7fe1bc6b508be13a4dd582
A5200=40c6f2f1ad4a3145b328d5baaf010fae6c7e752b
ATARI800=4e7fbc73765c1a9670c7506616046ad1d4ccda51
VECX=dccff5402a16d1ea7c112095a8e42632bf149506
# PicoDrive needs its submodules (libchdr, emu2413, dr_libs, libpicofe), which GitHub's
# archive leaves out, so its tarball is a recursive checkout of this commit, .git removed.
PICODRIVE=1890c2932234c9d30f4cd3851d02228baae8f09e
# melonDS, not DeSmuME 2015: DeSmuME has no arm64 JIT, so it would interpret both DS CPUs.
MELONDS=4081e8358e66f1ea11879560be6b813dfeaab75a
mkdir -p vendor brick/core-src
get() { [ -f "vendor/$1-src.tar.gz" ] || curl -fL "$2" -o "vendor/$1-src.tar.gz"; }
get gearcoleco "https://github.com/drhelius/Gearcoleco/archive/$GEARCOLECO.tar.gz"
get a5200 "https://github.com/libretro/a5200/archive/$A5200.tar.gz"
get atari800 "https://github.com/libretro/libretro-atari800/archive/$ATARI800.tar.gz"
get vecx "https://github.com/libretro/libretro-vecx/archive/$VECX.tar.gz"
get melonds "https://github.com/libretro/melonDS/archive/$MELONDS.tar.gz"
[ -f vendor/picodrive-src.tar.gz ]
if [ -f brick/cores-systems2.sha256 ]; then sha256sum -c brick/cores-systems2.sha256; fi
x() { [ -d "brick/core-src/$2" ] || tar --no-same-owner --exclude "*/.agents" -xzf "vendor/$1-src.tar.gz" -C brick/core-src; echo "$PWD/brick/core-src/$2"; }
coleco_src=$(x gearcoleco "Gearcoleco-$GEARCOLECO")
a5200_src=$(x a5200 "a5200-$A5200")
a800_src=$(x atari800 "libretro-atari800-$ATARI800")
vecx_src=$(x vecx "libretro-vecx-$VECX")
pico_src=$(x picodrive "picodrive-$PICODRIVE")
melonds_src=$(x melonds "melonDS-$MELONDS")
T="CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar"
STATIC="-static-libstdc++ -static-libgcc"
J=${JOBS:-8}
# Gearcoleco is C++: its runtime is linked in, as with snes9x.
LDFLAGS="$STATIC" make -C "$coleco_src/platforms/libretro" platform=unix $T -j$J
make -C "$a5200_src" platform=unix $T -j$J
make -C "$a800_src" platform=unix $T -j$J
# HAS_GPU=0: the software line renderer; Slot gives cores no GL context.
make -C "$vecx_src" -f Makefile.libretro platform=unix HAS_GPU=0 $T -j$J
# aarch64 has PicoDrive's DRC for the 32X's SH-2s.
make -C "$pico_src" -f Makefile.libretro platform=aarch64 $T -j$J
# ARCH=arm64 turns on melonDS's aarch64 JIT; no GL (the software renderer, on its own thread).
make -C "$melonds_src" platform=unix ARCH=arm64 $T CXX="aarch64-linux-gnu-g++ $STATIC" GIT_VERSION=" ${MELONDS%"${MELONDS#???????}"}" -j$J
cp "$coleco_src/platforms/libretro/gearcoleco_libretro.so" \
    "$a5200_src/a5200_libretro.so" \
    "$a800_src/atari800_libretro.so" \
    "$vecx_src/vecx_libretro.so" \
    "$pico_src/picodrive_libretro.so" \
    "$melonds_src/melonds_libretro.so" vendor/
for f in gearcoleco a5200 atari800 vecx picodrive melonds; do
    aarch64-linux-gnu-strip --strip-unneeded "vendor/${f}_libretro.so"
done
