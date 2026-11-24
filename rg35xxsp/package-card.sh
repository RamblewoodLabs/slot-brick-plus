#!/bin/sh
# Assemble the TF2 tree. Does not build Slot; build-frontend.sh does that.
# Cores are the files in vendor/, the same ones brick/package-card.sh copies,
# unless --cores was passed to docker-build.sh and rebuilt those same commits.
set -eu
: "${BRICK_TOOLCHAIN:?Set BRICK_TOOLCHAIN}"
cd "$(dirname "$0")/.."
OUT="${1:-dist-rg35xxsp}"
if [ -e "$OUT" ]; then
    echo "Output already exists: $OUT. Choose a new directory." >&2
    exit 1
fi
mkdir -p "$OUT/System" "$OUT/Slot/System"

cp rg35xxsp/card/System/launch_frontend.sh "$OUT/System/"
cp rg35xxsp/card/Slot/System/launch.sh "$OUT/Slot/System/"
cp rg35xxsp/card/README.txt "$OUT/README.txt"
# Wi-Fi is the Brick helper: it talks to wpa_supplicant and udhcpc, which
# BaseOS has, and it does not call TrimUI binaries when those are absent.
# Sleep is the SP script. The Brick one uses Super Standby, which will not
# wake when the lid opens.
cp brick/card/Slot/System/brick_wifi.sh "$OUT/Slot/System/"
cp rg35xxsp/card/Slot/System/sleep.sh "$OUT/Slot/System/"
cp brick/card/Slot/System/cart-art-games.txt "$OUT/Slot/System/"
for f in brick/card/Slot/System/*-labels.txt brick/card/Slot/System/*-cart-labels.txt; do
    if [ -f "$f" ]; then
        cp "$f" "$OUT/Slot/System/"
    fi
done
mkdir -p "$OUT/Slot/System/Fonts"
cp brick/card/Slot/System/Fonts/GohuFont.ttf "$OUT/Slot/System/Fonts/"
cp -R brick/card/Slot/System/licenses "$OUT/Slot/System/"
cp LICENSE "$OUT/Slot/System/licenses/slot-GPL-3.0.txt"

cp target/aarch64-unknown-linux-gnu/device/slot "$OUT/Slot/System/"
cp vendor/mgba_libretro.so vendor/gpsp_libretro.so vendor/fceumm_libretro.so \
    vendor/snes9x_libretro.so vendor/pcsx_rearmed_libretro.so "$OUT/Slot/System/"
for f in slot mgba_libretro.so gpsp_libretro.so fceumm_libretro.so snes9x_libretro.so pcsx_rearmed_libretro.so; do
    "$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-strip" "$OUT/Slot/System/$f"
done

for d in BIOS Config \
    Games/GB Games/GBC Games/GBA Games/NES Games/SNES Games/PS1 \
    Labels/GB Labels/GBC Labels/GBA Labels/NES Labels/SNES Labels/PS1 \
    Saves/GB Saves/GBC Saves/GBA Saves/NES Saves/SNES Saves/PS1 \
    States/GB States/GBC States/GBA States/NES States/SNES States/PS1 \
    Overlays/GB Overlays/GBC Overlays/GBA Overlays/NES Overlays/SNES Overlays/PS1 \
    Wallpapers Logs Fonts; do
    mkdir -p "$OUT/Slot/$d"
done

chmod +x "$OUT/System/launch_frontend.sh" "$OUT/Slot/System/launch.sh" \
    "$OUT/Slot/System/brick_wifi.sh" "$OUT/Slot/System/sleep.sh" \
    "$OUT/Slot/System/slot"
echo "TF2 files written to $OUT"
