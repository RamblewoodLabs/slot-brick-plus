#!/bin/sh
# The two cores that draw with the GPU (N64: mupen64plus-next, PSP: PPSSPP), for the Brick and
# the RG35XX SP. Run inside brick/gpu-cores/Dockerfile (arm64, gcc 10; brick/build-gpu-cores.sh
# does that). Neither links the GL libraries: Slot loads the device's EGL and GLES globally
# before it opens one of these cores, and their GL calls bind to those. glibc_shim.c holds
# them to the glibc 2.17 symbols every other core uses.
set -eu
cd /src
MUPEN=8623e490a8773fcb6c10f424cebbdb0ac44e88ea
PPSSPP=v1.20.4
W=/tmp/gpu-cores
rm -rf "$W" && mkdir -p "$W"
sha256sum -c brick/gpu-cores/sources.sha256
tar --no-same-owner -xzf vendor/mupen64plus_next-src.tar.gz -C "$W"
tar --no-same-owner -xzf vendor/ppsspp-src.tar.gz -C "$W"
gcc -O2 -fPIC -fno-builtin -c brick/gpu-cores/glibc_shim.c -o "$W/glibc_shim.o"
CPU="-mcpu=cortex-a53 -mtune=cortex-a53"

m="$W/mupen64plus-libretro-nx-$MUPEN"
python3 - "$m/Makefile" "$W/glibc_shim.o" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read().replace('-Wl,--no-undefined', '-static-libstdc++ -static-libgcc ' + sys.argv[2])
open(p, 'w').write(s)
PY
make -C "$m" platform=unix FORCE_GLES3=1 WITH_DYNAREC=aarch64 ARCH=aarch64 GL_LIB= \
    CPUFLAGS="$CPU" -j"${JOBS:-8}" > "$W/mupen.log" 2>&1 || { tail -30 "$W/mupen.log"; exit 1; }
cp "$m/mupen64plus_next_libretro.so" vendor/

p="$W/ppsspp-$PPSSPP"
cmake -S "$p" -B "$p/build" -DLIBRETRO=ON -DUSING_GLES2=ON -DCMAKE_BUILD_TYPE=Release \
    -DUSE_DISCORD=OFF -DUSE_MINIUPNPC=OFF -DUSE_FFMPEG=ON -DUSE_SYSTEM_FFMPEG=OFF -DUSING_X11_VULKAN=OFF \
    -DCMAKE_C_FLAGS="$CPU" -DCMAKE_CXX_FLAGS="$CPU" \
    -DCMAKE_SHARED_LINKER_FLAGS="-static-libstdc++ -static-libgcc $W/glibc_shim.o" > "$W/ppsspp.log" 2>&1
cmake --build "$p/build" --target ppsspp_libretro -j"${JOBS:-8}" >> "$W/ppsspp.log" 2>&1 || { tail -30 "$W/ppsspp.log"; exit 1; }
cp "$(find "$p/build" -name ppsspp_libretro.so | head -1)" vendor/

for f in mupen64plus_next ppsspp; do
    so="vendor/${f}_libretro.so"
    strip --strip-unneeded "$so"
    for n in $(readelf -d "$so" | grep NEEDED | grep -o 'lib\(GLESv2\|GLESv1_CM\|EGL\|GL\)[^]]*' || true); do
        patchelf --remove-needed "$n" "$so"
    done
    if objdump -T "$so" | grep -qE 'GLIBC_2\.(1[89]|2[0-9]|3[0-9])'; then
        echo "$so needs a glibc newer than 2.17" >&2; exit 1
    fi
done
echo "gpu cores built"
