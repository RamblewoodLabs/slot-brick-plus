#!/bin/sh
# Cross-compile the Slotted shell for the RG280V and pack slotted.opk.
# Run inside the slot-rg280v-build image (see docker-build.sh shell).
set -eu
cd "$(dirname "$0")/.."

export PATH="/opt/gcw0-toolchain/bin:/usr/local/cargo/bin:${PATH:-}"
export RUSTUP_TOOLCHAIN="${RUSTUP_TOOLCHAIN:-nightly-2026-11-25}"
export RUSTFLAGS="${RUSTFLAGS:-} -Zunstable-options -Clink-arg=-latomic"
export RUST_TARGET_PATH="${RUST_TARGET_PATH:-/src/rg280v}"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/src/.docker-cache/rg280v-target}"
export CARGO_HOME="${CARGO_HOME:-/src/.docker-cache/cargo-rg280v}"
OUT="${OUT:-dist-rg280v}"

# The committed target must stay hard-float FPXX uClibc.
grep -q '"env": "uclibc"' rg280v/mipsel-gcw0-linux-uclibc.json
grep -q 'fpxx' rg280v/mipsel-gcw0-linux-uclibc.json
! grep -q 'soft-float' rg280v/mipsel-gcw0-linux-uclibc.json

echo "=== Rust Slotted shell (nightly, -Z build-std, features rg280v) ==="
cargo --version
rustc --version
cargo build --profile rg280v \
    -Z build-std=std,panic_abort \
    --target mipsel-gcw0-linux-uclibc \
    -p slot --no-default-features --features rg280v

BIN="$CARGO_TARGET_DIR/mipsel-gcw0-linux-uclibc/rg280v/slot"
test -f "$BIN"
mipsel-linux-strip -s "$BIN" -o "$OUT/slotted" || cp "$BIN" "$OUT/slotted"
chmod +x "$OUT/slotted"
mipsel-linux-readelf -h "$OUT/slotted"
mipsel-linux-readelf -A "$OUT/slotted" | sed -n '1,20p'

STAGE="$OUT/opk-slotted"
rm -rf "$STAGE"
mkdir -p "$STAGE"
cp "$OUT/slotted" "$STAGE/slotted"
cp rg280v/slotted.desktop "$STAGE/slotted.desktop"
cp rg280v/slotted.png "$STAGE/slotted.png"
cp rg280v/launchers.ini "$STAGE/launchers.ini"
# A short note inside the OPK; flashing a card is still Jeff's yes.
cat > "$STAGE/README.txt" << 'TXT'
Slotted for the Anbernic RG280V
Copy this OPK to TF2/apps once a bootable Adam v2.1 card is in TF1.
Do not flash a card until Jeff says yes. Do not download BIOS.
Orientation: Slot/Config/orientation or SLOTTED_ORIENTATION
  (auto, portrait, landscape, portrait-flip, landscape-flip, rotate-90, rotate-270)
Launchers: Slot/Config/launchers.ini overrides the bundled table.
TXT

mksquashfs "$STAGE" "$OUT/slotted.opk" -noappend -no-xattrs -all-root -comp gzip
ls -lh "$OUT/slotted" "$OUT/slotted.opk"
echo "shell OPK built."
