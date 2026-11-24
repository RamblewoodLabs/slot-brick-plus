#!/bin/sh
# Starts Slot on BaseOS. No TrimUI input daemon and no stock launcher to return to.
set -u
SYS="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
export SLOT_ROOT="$(dirname "$SYS")"
mkdir -p "$SLOT_ROOT/Logs" 2>/dev/null
[ ! -f "$SLOT_ROOT/Logs/slot.log" ] || mv -f "$SLOT_ROOT/Logs/slot.log" "$SLOT_ROOT/Logs/slot.previous.log" 2>/dev/null
if ( : >> "$SLOT_ROOT/Logs/slot.log" ) 2>/dev/null; then
    exec > "$SLOT_ROOT/Logs/slot.log" 2>&1
else
    exec > /tmp/slot.log 2>&1
    echo 'WARNING: SD card is not writable'
fi
export SLOT_BOOT_TIMES="${SLOT_BOOT_TIMES:-$SLOT_ROOT/Logs/boot-times.log}"
stamp() { echo "$(cut -d' ' -f1 /proc/uptime 2>/dev/null) $*" >> "$SLOT_BOOT_TIMES" 2>/dev/null; }
stamp launch.sh started
printf 'slot RG35XX SP\n'
date
uname -a
# BaseOS keeps glibc, Mali and ALSA on the default loader path.
export LD_LIBRARY_PATH="/usr/lib:/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export PATH="/usr/sbin:/usr/bin:/sbin:/bin"
if [ -f "$SLOT_ROOT/DEBUG_INPUT" ]; then
    export SLOT_TRACE_INPUT=1
else
    unset SLOT_TRACE_INPUT
fi
if [ "$(uname -m)" != aarch64 ]; then
    echo 'This build requires an aarch64 RG35XX SP running BaseOS.'
    exit 1
fi
chmod +x "$SYS/slot" 2>/dev/null || :
cd "$SLOT_ROOT"
stamp starting slot
"$SYS/slot"
status=$?
echo "slot exit code: $status"
if [ "$status" -ge 128 ]; then
    CRASH="$SLOT_ROOT/Logs/crash"
    mkdir -p "$CRASH" && cp "$SLOT_ROOT/Logs/slot.log" "$CRASH/slot-$(date '+%Y-%m-%d_%H-%M-%S').log" \
        && echo "Crash log kept in Logs/crash"
    ls -1t "$CRASH"/slot-*.log 2>/dev/null | tail -n +6 | while read -r old; do rm -f "$old"; done
fi
sync
# A replaced binary that never showed a frame is put back. The SP menu does not
# offer Online → Update (that download is the Brick binary). Copying a new
# slot over System/slot by hand still gets this safety net.
if [ "$status" != 66 ] && [ -f "$SYS/slot.pending" ] && [ -f "$SYS/slot.bak" ]; then
    echo 'The update did not start: putting the previous Slot back'
    mv -f "$SYS/slot.bak" "$SYS/slot"
    chmod +x "$SYS/slot"
    rm -f "$SYS/slot.pending" "$SYS/slot.new"
    sync
    status=66
fi
case "$status" in
    64) action=poweroff ;;
    65) action=reboot ;;
    66)
        echo 'Starting Slot again on the updated files'
        exec sh "$SYS/launch.sh"
        ;;
    *) exit "$status" ;;
esac
echo "Requesting system $action"
if ! "$action"; then
    echo "FAILED: $action command"
    exit 1
fi
while :; do sleep 1; done
