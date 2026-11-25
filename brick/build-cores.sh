#!/bin/sh
set -eu
: "${BRICK_TOOLCHAIN:?Set BRICK_TOOLCHAIN to the extracted TrimUI toolchain directory}"
cd "$(dirname "$0")/.."
export PATH="$BRICK_TOOLCHAIN/bin:$PATH"
MGBA=e31759b24e7a4e3899285ff720d7b573ac328ae7
GPSP=7b2dd37394e2822110cacef321e9bb291a305b77
FCEUMM=7a542dab1e87679921962a9f056186eca425c0c2
SNES9X=fae2fea08f74180759ef540ee94259213f503480
PCSX=c8816799b50388e61cfe237fe2cdbb7d8175f20a
mkdir -p vendor brick/core-src
[ -f vendor/mgba-src.tar.gz ] || curl -fL "https://github.com/libretro/mgba/archive/$MGBA.tar.gz" -o vendor/mgba-src.tar.gz
[ -f vendor/gpsp-src.tar.gz ] || curl -fL "https://github.com/libretro/gpsp/archive/$GPSP.tar.gz" -o vendor/gpsp-src.tar.gz
[ -f vendor/fceumm-src.tar.gz ] || curl -fL "https://github.com/libretro/libretro-fceumm/archive/$FCEUMM.tar.gz" -o vendor/fceumm-src.tar.gz
[ -f vendor/snes9x-src.tar.gz ] || curl -fL "https://github.com/libretro/snes9x/archive/$SNES9X.tar.gz" -o vendor/snes9x-src.tar.gz
[ -f vendor/pcsx_rearmed-src.tar.gz ] || curl -fL "https://github.com/libretro/pcsx_rearmed/archive/$PCSX.tar.gz" -o vendor/pcsx_rearmed-src.tar.gz
# The archives are committed; a fresh download must match them byte for byte.
sha256sum -c - <<'SUMS'
396d749cce8fe3358b29cbb1db479b1816a151bd688ee45b1d241503cbc40243  vendor/mgba-src.tar.gz
665876b87cb2d1c23264462be72ff2b5bd7dff7e0efb0c79d66bee87b72114ec  vendor/gpsp-src.tar.gz
f80b5c1df39e22d78791b1867ea79f3ec8c0823e4b1f82b3f89e432c614a6c53  vendor/fceumm-src.tar.gz
0d4b0c4181d66668ec0040eb17f90a559f7b7fa6cbd9198593c3bdbe79acd029  vendor/snes9x-src.tar.gz
edeb21fdeef8815521cb9d4488d2cd35746607c968ca9b26cce4e42c09f2dc99  vendor/pcsx_rearmed-src.tar.gz
SUMS
mgba_src="$PWD/brick/core-src/mgba-$MGBA"
gpsp_src="$PWD/brick/core-src/gpsp-$GPSP"
fceumm_src="$PWD/brick/core-src/libretro-fceumm-$FCEUMM"
snes9x_src="$PWD/brick/core-src/snes9x-$SNES9X"
pcsx_src="$PWD/brick/core-src/pcsx_rearmed-$PCSX"
if [ ! -d "$mgba_src" ]; then
    tar --no-same-owner -xzf vendor/mgba-src.tar.gz -C brick/core-src
    for p in cores/mgba/*.patch; do git -C "$mgba_src" apply "$PWD/$p"; done
fi
if [ ! -d "$gpsp_src" ]; then
    tar --no-same-owner -xzf vendor/gpsp-src.tar.gz -C brick/core-src
    for p in cores/gpsp/*.patch brick/gpsp-old-binutils.patch; do git -C "$gpsp_src" apply "$PWD/$p"; done
fi
[ -d "$fceumm_src" ] || tar --no-same-owner -xzf vendor/fceumm-src.tar.gz -C brick/core-src
[ -d "$snes9x_src" ] || tar --no-same-owner -xzf vendor/snes9x-src.tar.gz -C brick/core-src
if [ ! -d "$pcsx_src" ]; then
    tar --no-same-owner -xzf vendor/pcsx_rearmed-src.tar.gz -C brick/core-src
    git -C "$pcsx_src" apply "$PWD/brick/pcsx-old-binutils.patch"
fi
cmake -S "$mgba_src" -B brick/core-build/mgba \
    -DCMAKE_SYSTEM_NAME=Linux -DCMAKE_SYSTEM_PROCESSOR=aarch64 \
    -DCMAKE_C_COMPILER=aarch64-linux-gnu-gcc -DCMAKE_CXX_COMPILER=aarch64-linux-gnu-g++ \
    -DLIBMGBA_ONLY=ON -DBUILD_LIBRETRO=ON -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_C_FLAGS=-mcpu=cortex-a53 \
    -DCMAKE_DISABLE_FIND_PACKAGE_PNG=ON -DCMAKE_DISABLE_FIND_PACKAGE_ZLIB=ON \
    -DCMAKE_DISABLE_FIND_PACKAGE_LibZip=ON -DCMAKE_POLICY_VERSION_MINIMUM=3.5
cmake --build brick/core-build/mgba --target mgba_libretro --parallel 8
make -C "$gpsp_src" platform=arm64 CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
# rpi3_64 is FCEUmm's Cortex-A53 target, the Brick's CPU.
make -C "$fceumm_src" -f Makefile.libretro platform=rpi3_64 CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
# snes9x is C++: its runtime is linked in, as the firmware's libstdc++ may be too old or absent.
CXXFLAGS="-mcpu=cortex-a53 -mtune=cortex-a53" LDFLAGS="-static-libstdc++ -static-libgcc" \
    make -C "$snes9x_src/libretro" platform=unix CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
# h5 is PCSX ReARMed's Allwinner Cortex-A53 target: the arm64 dynarec and the NEON renderer.
make -C "$pcsx_src" -f Makefile.libretro platform=h5 GIT_VERSION="${PCSX%"${PCSX#???????}"}" CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
cp brick/core-build/mgba/mgba_libretro.so vendor/
cp "$gpsp_src/gpsp_libretro.so" vendor/
cp "$fceumm_src/fceumm_libretro.so" vendor/
cp "$snes9x_src/libretro/snes9x_libretro.so" vendor/
cp "$pcsx_src/pcsx_rearmed_libretro.so" vendor/

# Genesis / Master System / Game Gear, Arcade, TurboGrafx-16, Atari 2600,
# Intellivision, Lynx and 7800. Same toolchain as the cores above, so both the
# Brick card and the RG35XX SP card (rg35xxsp/docker-build.sh --cores) get them.
GENESIS=099c962e7374eb84cae9bba180c9e0e49e767b40
FBNEO=2e6605c4a08b86455e3aee03294fe35e7618fa91
PCE=4cf066e22533186d0cfad85a0fec2aced93e4e29
STELLA=7d1361e407e63f29e52892655069e5fb4096e691
FREEINTV=ef3e0fe322bec62a7f916c0bb0834c08c348d0b4
HANDY=bc55d462f0b2d6b073ea93dc552ebd73cec60fd1
PROSYSTEM=8a88014287c7a01cd568067e5a557d0a2b2a051f
[ -f vendor/genesis_plus_gx-src.tar.gz ] || curl -fL "https://github.com/libretro/Genesis-Plus-GX/archive/$GENESIS.tar.gz" -o vendor/genesis_plus_gx-src.tar.gz
[ -f vendor/fbneo-src.tar.gz ] || curl -fL "https://github.com/libretro/FBNeo/archive/$FBNEO.tar.gz" -o vendor/fbneo-src.tar.gz
[ -f vendor/mednafen_pce_fast-src.tar.gz ] || curl -fL "https://github.com/libretro/beetle-pce-fast-libretro/archive/$PCE.tar.gz" -o vendor/mednafen_pce_fast-src.tar.gz
[ -f vendor/stella2014-src.tar.gz ] || curl -fL "https://github.com/libretro/stella2014-libretro/archive/$STELLA.tar.gz" -o vendor/stella2014-src.tar.gz
[ -f vendor/freeintv-src.tar.gz ] || curl -fL "https://github.com/libretro/FreeIntv/archive/$FREEINTV.tar.gz" -o vendor/freeintv-src.tar.gz
[ -f vendor/handy-src.tar.gz ] || curl -fL "https://github.com/libretro/libretro-handy/archive/$HANDY.tar.gz" -o vendor/handy-src.tar.gz
[ -f vendor/prosystem-src.tar.gz ] || curl -fL "https://github.com/libretro/prosystem-libretro/archive/$PROSYSTEM.tar.gz" -o vendor/prosystem-src.tar.gz
sha256sum -c - <<'SUMS'
5146e86c16a0e7cf32206533434880f0b45cf294c0575a97ace7bda0e338d219  vendor/genesis_plus_gx-src.tar.gz
cb1ac0e28a69ba01826be5dee41dbeb842e04b36a23bd14b28fcc5b83af4a411  vendor/fbneo-src.tar.gz
6832f9c4b3b960d30f9019d54a64224a0cd4717b18cc45eb0e7ba65115d39934  vendor/mednafen_pce_fast-src.tar.gz
c0f5e53f3c21e7760e434784634d44a02e57a30917da5279cfd3add4736afd82  vendor/stella2014-src.tar.gz
912d0a9c314cc63c396f8bc5a5778c341a3f4f860ebae1e069a877b9c98d0f11  vendor/freeintv-src.tar.gz
65ad333df22aab7f3c8156c21ffd0b8da1ef09c7a7f1775df964b89986aa7324  vendor/handy-src.tar.gz
23401c8b104ef24db111bbc4b13e37f224ffe16febfb89c704c70cdb269603bc  vendor/prosystem-src.tar.gz
SUMS
genesis_src="$PWD/brick/core-src/Genesis-Plus-GX-$GENESIS"
fbneo_src="$PWD/brick/core-src/FBNeo-$FBNEO"
pce_src="$PWD/brick/core-src/beetle-pce-fast-libretro-$PCE"
stella_src="$PWD/brick/core-src/stella2014-libretro-$STELLA"
freeintv_src="$PWD/brick/core-src/FreeIntv-$FREEINTV"
handy_src="$PWD/brick/core-src/libretro-handy-$HANDY"
prosystem_src="$PWD/brick/core-src/prosystem-libretro-$PROSYSTEM"
[ -d "$genesis_src" ] || tar --no-same-owner -xzf vendor/genesis_plus_gx-src.tar.gz -C brick/core-src
[ -d "$fbneo_src" ] || tar --no-same-owner -xzf vendor/fbneo-src.tar.gz -C brick/core-src
[ -d "$pce_src" ] || tar --no-same-owner -xzf vendor/mednafen_pce_fast-src.tar.gz -C brick/core-src
[ -d "$stella_src" ] || tar --no-same-owner -xzf vendor/stella2014-src.tar.gz -C brick/core-src
[ -d "$freeintv_src" ] || tar --no-same-owner -xzf vendor/freeintv-src.tar.gz -C brick/core-src
[ -d "$handy_src" ] || tar --no-same-owner -xzf vendor/handy-src.tar.gz -C brick/core-src
[ -d "$prosystem_src" ] || tar --no-same-owner -xzf vendor/prosystem-src.tar.gz -C brick/core-src
# unix plus this aarch64 gcc: Genesis Plus GX's rpi3 target is 32-bit ARM.
make -C "$genesis_src" -f Makefile.libretro platform=unix CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
# rpi3_64 is FBNeo's Cortex-A53 aarch64 target, the same CPU the Brick uses.
LDFLAGS="-static-libstdc++ -static-libgcc -pthread" \
    make -C "$fbneo_src/src/burner/libretro" -f Makefile platform=rpi3_64 CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
for src in "$pce_src" "$stella_src" "$freeintv_src" "$prosystem_src"; do
    LDFLAGS="-static-libstdc++ -static-libgcc" \
        make -C "$src" platform=unix CC=aarch64-linux-gnu-gcc CXX=aarch64-linux-gnu-g++ AR=aarch64-linux-gnu-ar -j8
done
# Handy's makefile links with g++ and ignores LDFLAGS, so the runtime has to be named on the compiler.
make -C "$handy_src" platform=unix CC=aarch64-linux-gnu-gcc \
    CXX="aarch64-linux-gnu-g++ -static-libstdc++ -static-libgcc" AR=aarch64-linux-gnu-ar -j8
cp "$genesis_src/genesis_plus_gx_libretro.so" \
    "$fbneo_src/src/burner/libretro/fbneo_libretro.so" \
    "$pce_src/mednafen_pce_fast_libretro.so" \
    "$stella_src/stella2014_libretro.so" \
    "$freeintv_src/freeintv_libretro.so" \
    "$handy_src/handy_libretro.so" \
    "$prosystem_src/prosystem_libretro.so" vendor/
for f in genesis_plus_gx fbneo mednafen_pce_fast stella2014 freeintv handy prosystem; do
    aarch64-linux-gnu-strip --strip-unneeded "vendor/${f}_libretro.so"
done
