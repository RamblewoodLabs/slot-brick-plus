#!/bin/sh
# Opens an app the player installed (HOME → Apps → GrainBrick) in Slot's place, then starts Slot again.
# Usage (by Slot only): run-app.sh <app folder> <python3>
# Slot has replaced itself with this script (same process, which launch.sh or Slot OS's init is
# waiting on), so the screen, the sound card and the buttons are free; its saves are settled.
# Whatever happens to the app, Slot comes back: the last line always starts it again.
APP="$1"
PY="$2"
ROOT="${SLOT_ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}"
LOG="$ROOT/Logs/apps.log"
mkdir -p "$ROOT/Logs" 2>/dev/null
[ ! -f "$LOG" ] || [ "$(wc -c < "$LOG")" -lt 1048576 ] || mv -f "$LOG" "$ROOT/Logs/apps.previous.log"
echo "--- $(date '+%Y-%m-%d %H:%M:%S') opening $APP with $PY" >> "$LOG" 2>/dev/null
if [ -f "$APP/main.py" ] && [ -f "$PY" ]; then
    # Buttons: on Slot OS they reach Slot through GPIO, which apps cannot read; the bridge shows
    # them to apps as a joystick laid out like stock's (on stock it exits: SDL has the pad there).
    ./System/slot --pad-bridge >> "$LOG" 2>&1 &
    BRIDGE=$!
    sleep 0.3
    # GrainBrick's own launcher does the same: its folder first on the module path (its NumPy
    # is in _vendor), run from its folder, which it finds its files by.
    (
        cd "$APP" || exit 1
        export PYTHONPATH="$APP/_vendor:$APP${PYTHONPATH:+:$PYTHONPATH}"
        export SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1 PYGAME_HIDE_SUPPORT_PROMPT=1
        RT="$(dirname -- "$PY")/.."
        if [ -f "$RT/lib/python3.12/site-packages/slot_fb.py" ]; then
            # Slot's runtime (Slot/Apps/python): SDL draws off-screen and its slot_fb copies each
            # frame to the framebuffer; SDL leaves the sound card alone (GrainBrick plays through
            # aplay). lib/compat holds a SoundTouch built for the Brick's glibc, ahead of the app's own.
            export SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SLOT_FB=/dev/fb0
            export LD_LIBRARY_PATH="$RT/lib/compat${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
        fi
        exec "$PY" "$APP/main.py"
    ) >> "$LOG" 2>&1
    status=$?
    kill "$BRIDGE" 2>/dev/null
    wait "$BRIDGE" 2>/dev/null
    echo "--- $(date '+%Y-%m-%d %H:%M:%S') app ended ($status)" >> "$LOG" 2>/dev/null
else
    echo "--- not opened: $APP/main.py or $PY is missing" >> "$LOG" 2>/dev/null
fi
sync
cd "$ROOT" || exit 1
exec ./System/slot
