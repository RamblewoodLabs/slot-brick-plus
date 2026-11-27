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
# Wi-Fi and suspend are the Brick helpers. They talk to wpa_supplicant, udhcpc
# and /sys/power/state, which BaseOS has. They do not call TrimUI binaries
# when those binaries are absent.
cp brick/card/Slot/System/brick_wifi.sh "$OUT/Slot/System/"
cp brick/card/Slot/System/sleep.sh "$OUT/Slot/System/"
cp brick/card/Slot/System/cart-art-games.txt "$OUT/Slot/System/"
for f in brick/card/Slot/System/*-labels.txt brick/card/Slot/System/*-cart-labels.txt; do
    if [ -f "$f" ]; then
        cp "$f" "$OUT/Slot/System/"
    fi
done
mkdir -p "$OUT/Slot/System/Fonts"
# Labels kept on this machine only (brick/labels-local is not in git): a Game & Watch's
# shelf face and opening frames are rendered from its owner's own media (brick/gw-art).
if [ -d brick/labels-local ]; then
    cp -R brick/labels-local/. "$OUT/Slot/Labels/"
fi
cp brick/card/Slot/System/Fonts/GohuFont.ttf "$OUT/Slot/System/Fonts/"
cp -R brick/card/Slot/System/licenses "$OUT/Slot/System/"
cp LICENSE "$OUT/Slot/System/licenses/slot-GPL-3.0.txt"

cp target/aarch64-unknown-linux-gnu/device/slot "$OUT/Slot/System/"
cp vendor/mgba_libretro.so vendor/gpsp_libretro.so vendor/fceumm_libretro.so \
    vendor/snes9x_libretro.so vendor/pcsx_rearmed_libretro.so \
    vendor/genesis_plus_gx_libretro.so vendor/fbneo_libretro.so vendor/mednafen_pce_fast_libretro.so \
    vendor/stella2014_libretro.so vendor/freeintv_libretro.so vendor/handy_libretro.so vendor/prosystem_libretro.so vendor/gw_libretro.so \
    vendor/gearcoleco_libretro.so vendor/a5200_libretro.so vendor/atari800_libretro.so vendor/vecx_libretro.so vendor/picodrive_libretro.so vendor/melonds_libretro.so vendor/mupen64plus_next_libretro.so vendor/ppsspp_libretro.so \
    "$OUT/Slot/System/"
for f in slot mgba_libretro.so gpsp_libretro.so fceumm_libretro.so snes9x_libretro.so pcsx_rearmed_libretro.so \
    genesis_plus_gx_libretro.so fbneo_libretro.so mednafen_pce_fast_libretro.so stella2014_libretro.so \
    freeintv_libretro.so handy_libretro.so prosystem_libretro.so gw_libretro.so \
    gearcoleco_libretro.so a5200_libretro.so atari800_libretro.so vecx_libretro.so picodrive_libretro.so melonds_libretro.so mupen64plus_next_libretro.so ppsspp_libretro.so; do
    "$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-strip" "$OUT/Slot/System/$f"
done

for d in BIOS Config \
    Games/GB Games/GBC Games/GBA Games/NES Games/SNES Games/PS1 \
    Games/MD Games/SMS Games/GG Games/ARCADE Games/PCE Games/A2600 Games/INTV Games/LYNX Games/A7800 Games/GW Games/COLECO Games/A5200 Games/A800 Games/VECTREX Games/32X Games/N64 Games/NDS Games/PSP \
    Labels/GB Labels/GBC Labels/GBA Labels/NES Labels/SNES Labels/PS1 \
    Labels/MD Labels/SMS Labels/GG Labels/ARCADE Labels/PCE Labels/A2600 Labels/INTV Labels/LYNX Labels/A7800 Labels/GW Labels/COLECO Labels/A5200 Labels/A800 Labels/VECTREX Labels/32X Labels/N64 Labels/NDS Labels/PSP \
    Saves/GB Saves/GBC Saves/GBA Saves/NES Saves/SNES Saves/PS1 \
    Saves/MD Saves/SMS Saves/GG Saves/ARCADE Saves/PCE Saves/A2600 Saves/INTV Saves/LYNX Saves/A7800 Saves/GW Saves/COLECO Saves/A5200 Saves/A800 Saves/VECTREX Saves/32X Saves/N64 Saves/NDS Saves/PSP \
    States/GB States/GBC States/GBA States/NES States/SNES States/PS1 \
    States/MD States/SMS States/GG States/ARCADE States/PCE States/A2600 States/INTV States/LYNX States/A7800 States/GW States/COLECO States/A5200 States/A800 States/VECTREX States/32X States/N64 States/NDS States/PSP \
    Overlays/GB Overlays/GBC Overlays/GBA Overlays/NES Overlays/SNES Overlays/PS1 \
    Overlays/MD Overlays/SMS Overlays/GG Overlays/ARCADE Overlays/PCE Overlays/A2600 Overlays/INTV Overlays/LYNX Overlays/A7800 Overlays/GW Overlays/COLECO Overlays/A5200 Overlays/A800 Overlays/VECTREX Overlays/32X Overlays/N64 Overlays/NDS Overlays/PSP \
    Wallpapers Logs Fonts; do
    mkdir -p "$OUT/Slot/$d"
done

# PPSSPP's own data (its fonts and the atlas of its messages), which it reads from the
# folder it is given for BIOS files.
mkdir -p "$OUT/Slot/BIOS/PPSSPP" && cp -R vendor/ppsspp-assets/. "$OUT/Slot/BIOS/PPSSPP/"
chmod +x "$OUT/System/launch_frontend.sh" "$OUT/Slot/System/launch.sh" \
    "$OUT/Slot/System/brick_wifi.sh" "$OUT/Slot/System/sleep.sh" \
    "$OUT/Slot/System/slot"
echo "TF2 files written to $OUT"
