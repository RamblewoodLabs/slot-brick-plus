#!/bin/bash
# Copy a Brick library onto a TF2 card. Skips files that are already there.
# Never deletes anything.
#
# Usage: sh rg35xxsp/copy-from-brick.sh BRICK_CARD TF2_CARD
#   BRICK_CARD  the Brick card's root (the folder that contains Slot/), or Slot/ itself
#   TF2_CARD    the TF2 volume root, or its Slot/ folder
#
# Copied, when missing on TF2: Games, BIOS, Saves, States, Labels, Fonts,
# Wallpapers, and Config.
#
# Left behind (device-specific, or not the player's library):
#   System/          binaries, cores, launch scripts
#   Overlays/        Brick frames are 1024x768; the SP screen is 640x480
#   Logs/
#   Apps/, trimui/, .slot-boot/   Brick firmware files, if the source is a card root
#   Config/brick_leds.ini, Config/brick_video_mode*.ini, and any file named brick_*
#   Config/sync.txt and Config/continue.txt (save sync is not in this Slot)
#
# sh rg35xxsp/copy-from-brick.sh --self-test
#   checks the skip and no-delete rules in a temporary directory.
set -euo pipefail

self_test() {
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT INT TERM
    brick="$tmp/brick"
    tf2="$tmp/tf2"
    mkdir -p "$brick/Slot/Games/GB" "$brick/Slot/BIOS" "$brick/Slot/Saves/GB" \
        "$brick/Slot/States/GB" "$brick/Slot/Config" "$brick/Slot/System" \
        "$brick/Slot/Overlays/GB" "$brick/Slot/Logs" "$brick/Apps" "$brick/.slot-boot"
    echo rom > "$brick/Slot/Games/GB/Zelda.gb"
    echo bios > "$brick/Slot/BIOS/gba_bios.bin"
    echo sav > "$brick/Slot/Saves/GB/Zelda.sav"
    echo state > "$brick/Slot/States/GB/Zelda.state"
    echo wifi=home > "$brick/Slot/Config/wifi.txt"
    echo host=home > "$brick/Slot/Config/sync.txt"
    echo go > "$brick/Slot/Config/continue.txt"
    echo leds > "$brick/Slot/Config/brick_leds.ini"
    echo mode > "$brick/Slot/Config/brick_video_mode_v3.ini"
    echo bin > "$brick/Slot/System/slot"
    echo overlay > "$brick/Slot/Overlays/GB/grid.png"
    echo log > "$brick/Slot/Logs/slot.log"
    echo boot > "$brick/.slot-boot/boot.sh"
    # Already on the TF2 card, and must not be replaced or removed.
    mkdir -p "$tf2/Slot/Games/GB" "$tf2/Slot/System" "$tf2/System"
    echo mine > "$tf2/Slot/Games/GB/Zelda.gb"
    echo sp-binary > "$tf2/Slot/System/slot"
    echo launcher > "$tf2/System/launch_frontend.sh"
    bash "$0" "$brick" "$tf2"
    [ "$(cat "$tf2/Slot/Games/GB/Zelda.gb")" = mine ]
    [ "$(cat "$tf2/Slot/BIOS/gba_bios.bin")" = bios ]
    [ "$(cat "$tf2/Slot/Saves/GB/Zelda.sav")" = sav ]
    [ "$(cat "$tf2/Slot/States/GB/Zelda.state")" = state ]
    [ "$(cat "$tf2/Slot/Config/wifi.txt")" = "wifi=home" ]
    [ ! -e "$tf2/Slot/Config/sync.txt" ]
    [ ! -e "$tf2/Slot/Config/continue.txt" ]
    [ ! -e "$tf2/Slot/Config/brick_leds.ini" ]
    [ ! -e "$tf2/Slot/Config/brick_video_mode_v3.ini" ]
    [ "$(cat "$tf2/Slot/System/slot")" = sp-binary ]
    [ ! -e "$tf2/Slot/Overlays/GB/grid.png" ]
    [ ! -e "$tf2/Slot/Logs/slot.log" ]
    [ ! -e "$tf2/Apps" ]
    [ ! -e "$tf2/.slot-boot" ]
    [ "$(cat "$tf2/System/launch_frontend.sh")" = launcher ]
    # A second run copies nothing new and still deletes nothing.
    bash "$0" "$brick" "$tf2"
    [ "$(cat "$tf2/Slot/Games/GB/Zelda.gb")" = mine ]
    echo "copy-from-brick: self-test passed"
}

if [ "${1:-}" = --self-test ]; then
    self_test
    exit 0
fi

if [ "$#" -ne 2 ]; then
    echo "Usage: sh rg35xxsp/copy-from-brick.sh BRICK_CARD TF2_CARD" >&2
    echo "       sh rg35xxsp/copy-from-brick.sh --self-test" >&2
    exit 2
fi

if [ -d "$1/Slot" ]; then
    SRC="$1/Slot"
elif [ -d "$1/Games" ]; then
    SRC=$1
else
    echo "No Slot folder in $1 (looked for Slot/ and for Games/)." >&2
    exit 1
fi

case "$(basename "$2")" in
    Slot) DST=$2 ;;
    *) DST=$2/Slot ;;
esac
mkdir -p "$DST"

# Player folders only. System, Overlays and Logs stay on the device they belong to.
for top in Games BIOS Saves States Labels Fonts Wallpapers Config; do
    [ -d "$SRC/$top" ] || continue
    # -print0 so names with spaces survive. macOS and busybox find both have it.
    find "$SRC/$top" -type f -print0 | while IFS= read -r -d '' src; do
        rel=${src#"$SRC/"}
        base=$(basename "$rel")
        case "$base" in
            brick_*|brick_leds.ini|sync.txt|continue.txt) continue ;;
        esac
        case "$rel" in
            Config/brick_video_mode*) continue ;;
        esac
        dst="$DST/$rel"
        if [ -e "$dst" ]; then
            echo "skip $rel"
            continue
        fi
        mkdir -p "$(dirname "$dst")"
        cp "$src" "$dst"
        # Keep the Brick's file time so the copy does not look newer than the original.
        touch -r "$src" "$dst" 2>/dev/null || :
        echo "copy $rel"
    done
done

echo "Done. Existing files were left as they were. Nothing was deleted."
echo "Device files (System, Overlays, Logs, Brick LED and video settings) were not copied."
