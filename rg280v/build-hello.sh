#!/bin/sh
# Runs inside the slot-rg280v-build container. Builds the target spec (if
# needed), a tiny C SDL2 smoke binary, then the Rust hello OPK.
set -eu
cd /src
OUT="${OUT:-dist-rg280v}"
SYSROOT="${GCW0_SYSROOT:-/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot}"
# The repo-root rust-toolchain.toml is 1.96 (Brick/SP). This build must use the
# pinned nightly, with rust-src, regardless of the working directory.
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly-2026-11-25}"
export RUST_TARGET_PATH=/src/rg280v
export CARGO_TARGET_DIR=/src/.docker-cache/rg280v-target
export PATH="/opt/gcw0-toolchain/bin:/usr/local/cargo/bin:${PATH:-}"
# Custom target JSON requires -Z unstable-options on rustc, including cargo's target probe.
export RUSTFLAGS="${RUSTFLAGS:-} -Zunstable-options"

mkdir -p "$OUT"

echo "=== toolchain ==="
mipsel-linux-gcc -v 2>&1 | tail -3
pkg-config --modversion sdl2
ls -l "$SYSROOT/usr/lib/libSDL2.so" "$SYSROOT/usr/lib/libGLESv2.so"

echo "=== Rust target spec (committed, hard-float uClibc) ==="
SPEC=/src/rg280v/mipsel-gcw0-linux-uclibc.json
test -f "$SPEC"
grep -q '"env": "uclibc"' "$SPEC"
grep -q '"vendor": "gcw0"' "$SPEC"
grep -q mipsel-linux-gcc "$SPEC"
# Must not be the builtin soft-float target.
if grep -q soft-float "$SPEC"; then
    echo "target spec still has soft-float" >&2
    exit 1
fi
grep -E "env|features|linker|vendor" "$SPEC"

echo "=== C SDL2 smoke (static check that gcc + sysroot work) ==="
cat > /tmp/sdl-smoke.c <<'C'
#include <stdio.h>
#include <SDL.h>
int main(void) {
    /* Link and relocation only. Do not open a display in this smoke test. */
    printf("SDL %d.%d.%d\n", SDL_MAJOR_VERSION, SDL_MINOR_VERSION, SDL_PATCHLEVEL);
    return 0;
}
C
mipsel-linux-gcc --sysroot="$SYSROOT" -o "$OUT/sdl-smoke" /tmp/sdl-smoke.c \
    $(pkg-config --cflags --libs sdl2)
mipsel-linux-readelf -h "$OUT/sdl-smoke"
mipsel-linux-readelf -A "$OUT/sdl-smoke" | head -20

echo "=== Rust hello (nightly, -Z build-std) ==="
cargo --version
rustc --version
cargo build \
    -Z build-std=std,panic_abort \
    --target mipsel-gcw0-linux-uclibc \
    --manifest-path /src/rg280v/hello/Cargo.toml \
    --release

BIN="$CARGO_TARGET_DIR/mipsel-gcw0-linux-uclibc/release/slotted-hello"

test -f "$BIN"
mipsel-linux-readelf -h "$BIN"
mipsel-linux-strip -s "$BIN" -o /tmp/slotted-hello
mipsel-linux-readelf -h /tmp/slotted-hello

echo "=== pack OPK ==="
STAGE="$OUT/opk-stage"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp /tmp/slotted-hello "$STAGE/slotted-hello"
chmod 755 "$STAGE/slotted-hello"
cp /src/rg280v/hello/slotted-hello.desktop "$STAGE/"
cp /src/rg280v/hello/slotted-hello.png "$STAGE/slotted-hello.png"
# mksquashfs is shipped in the gcw0 toolchain.
mksquashfs "$STAGE" "$OUT/slotted-hello.opk" -all-root -noappend -no-exports -no-xattrs -comp gzip
ls -lh "$OUT/slotted-hello.opk" "$OUT/sdl-smoke"
echo "hello OPK built."
