#!/bin/sh
# BaseOS starts the first of System/frontend, System/launch_frontend.sh, or
# System/slot at the card root. This file is that entry. Games stay in Slot/,
# the same folders as a TrimUI Brick card, so BaseOS's own Slot layout (games
# at the card root) is not used.
set -u
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
ROOT="$(dirname "$HERE")/Slot"
if [ ! -x "$ROOT/System/slot" ] && [ ! -f "$ROOT/System/slot" ]; then
    echo "rg35xxsp: no Slot frontend at $ROOT/System/slot" >&2
    exit 1
fi
export SLOT_ROOT="$ROOT"
cd "$SLOT_ROOT" || exit 1
exec /bin/sh "$SLOT_ROOT/System/launch.sh"
