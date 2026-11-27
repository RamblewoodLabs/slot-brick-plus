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
    # GrainBrick's own launcher does the same: its folder first on the module path (its NumPy
    # is in _vendor), run from its folder, which it finds its files by.
    (
        cd "$APP" || exit 1
        export PYTHONPATH="$APP/_vendor:$APP${PYTHONPATH:+:$PYTHONPATH}"
        export SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1
        exec "$PY" "$APP/main.py"
    ) >> "$LOG" 2>&1
    status=$?
    echo "--- $(date '+%Y-%m-%d %H:%M:%S') app ended ($status)" >> "$LOG" 2>/dev/null
else
    echo "--- not opened: $APP/main.py or $PY is missing" >> "$LOG" 2>/dev/null
fi
sync
cd "$ROOT" || exit 1
exec ./System/slot
