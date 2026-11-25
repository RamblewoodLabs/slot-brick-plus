#!/bin/sh
set -eu
: "${BRICK_TOOLCHAIN:?Set BRICK_TOOLCHAIN}"
cd "$(dirname "$0")/.."
OUT="${1:-dist-brick}"
if [ -e "$OUT" ]; then echo "Output already exists: $OUT. Choose a new directory." >&2; exit 1; fi
mkdir -p "$OUT"
cp -R brick/card/. "$OUT/"

# Boot Logo app: picture converter plus its own copy of the file checker.
( cd brick/bootlogo-tool && cargo build --locked --release --target aarch64-unknown-linux-gnu )
cp brick/bootlogo-tool/target/aarch64-unknown-linux-gnu/release/slot-bootlogo "$OUT/Apps/BootLogo/"
cp brick/card/Slot/System/slot-filecheck "$OUT/Apps/BootLogo/"
cp target/aarch64-unknown-linux-gnu/device/slot "$OUT/Slot/System/"
cp vendor/mgba_libretro.so vendor/gpsp_libretro.so vendor/fceumm_libretro.so vendor/snes9x_libretro.so vendor/pcsx_rearmed_libretro.so \
    vendor/genesis_plus_gx_libretro.so vendor/fbneo_libretro.so vendor/mednafen_pce_fast_libretro.so \
    vendor/stella2014_libretro.so vendor/freeintv_libretro.so vendor/handy_libretro.so vendor/prosystem_libretro.so vendor/gw_libretro.so \
    "$OUT/Slot/System/"
for f in slot mgba_libretro.so gpsp_libretro.so fceumm_libretro.so snes9x_libretro.so pcsx_rearmed_libretro.so \
    genesis_plus_gx_libretro.so fbneo_libretro.so mednafen_pce_fast_libretro.so stella2014_libretro.so \
    freeintv_libretro.so handy_libretro.so prosystem_libretro.so gw_libretro.so; do "$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-strip" "$OUT/Slot/System/$f"; done
for d in BIOS Config Games/GB Games/GBC Games/GBA Games/NES Games/SNES Games/PS1 Games/MD Games/SMS Games/GG Games/ARCADE Games/PCE Games/A2600 Games/INTV Games/LYNX Games/A7800 Games/GW Labels/GB Labels/GBC Labels/GBA Labels/NES Labels/SNES Labels/PS1 Labels/MD Labels/SMS Labels/GG Labels/ARCADE Labels/PCE Labels/A2600 Labels/INTV Labels/LYNX Labels/A7800 Labels/GW Saves/GB Saves/GBC Saves/GBA Saves/NES Saves/SNES Saves/PS1 Saves/MD Saves/SMS Saves/GG Saves/ARCADE Saves/PCE Saves/A2600 Saves/INTV Saves/LYNX Saves/A7800 Saves/GW States/GB States/GBC States/GBA States/NES States/SNES States/PS1 States/MD States/SMS States/GG States/ARCADE States/PCE States/A2600 States/INTV States/LYNX States/A7800 States/GW Overlays/GB Overlays/GBC Overlays/GBA Overlays/NES Overlays/SNES Overlays/PS1 Overlays/MD Overlays/SMS Overlays/GG Overlays/ARCADE Overlays/PCE Overlays/A2600 Overlays/INTV Overlays/LYNX Overlays/A7800 Overlays/GW Wallpapers Logs; do mkdir -p "$OUT/Slot/$d"; done
cp brick/README.md "$OUT/Slot/README.md"
# Overlays kept on this machine only (brick/overlays-local is not in git, so a card built by
# GitHub never has them): PNGs only, for a card built here for its own owner.
for d in GB GBC GBA NES SNES PS1 MD SMS GG ARCADE PCE A2600 INTV LYNX A7800 GW; do
    for f in brick/overlays-local/"$d"/*.png; do
        if [ -f "$f" ]; then cp "$f" "$OUT/Slot/Overlays/$d/"; fi
    done
done
cp -R licenses "$OUT/Slot/System/"
cp LICENSE "$OUT/Slot/System/licenses/slot-GPL-3.0.txt"
# Labels kept on this machine only (brick/labels-local is not in git): a Game & Watch's
# shelf face and opening frames are rendered from its owner's own media (brick/gw-art).
if [ -d brick/labels-local ]; then
    cp -R brick/labels-local/. "$OUT/Slot/Labels/"
fi
chmod +x "$OUT/Slot/System/slot" "$OUT/Slot/System/"*.sh "$OUT/Apps/"*/launch.sh "$OUT/Apps/BootLogo/slot-bootlogo" "$OUT/Apps/BootLogo/slot-filecheck"
echo "Card files written to $OUT"
