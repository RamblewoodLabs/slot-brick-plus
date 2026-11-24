#!/bin/sh
# RG35XX SP sleep. Not the Brick script.
#
# BaseOS 1.4.0 writes 16 to axp2202-battery/os_sleep at boot. That is stock
# Super Standby: Power can wake the machine, and the hall sensor cannot, so
# opening the lid leaves the screen black. This script clears that for the
# sleep, arms every hall, lid, and power wakeup file it can find, and asks
# the kernel for `mem` only when one of those wakeups was armed.
#
# If `mem` is missing, no lid wakeup could be armed, or `mem` returns at once,
# the script does not stay in the kernel. It exits 2. Slot keeps the game
# paused, the backlight off, and the lid poll running, and wakes on lid-open
# or Power.
#
# Exit 0: the kernel slept and something woke it.
# Exit 2: screen-off standby; Slot is still running.
# Exit 1: cancelled while preparing, or the card could not be logged.
#
# sh rg35xxsp/card/Slot/System/sleep.sh --self-test
set -u

ROOT=${SLOT_ROOT:-/mnt/sdcard/Slot}
SYS=${SLOT_SLEEP_SYS:-/sys}
SUSPENDING=${SLOT_SLEEP_MARKER:-/tmp/slot-suspending}
WAKE=${SLOT_SLEEP_WAKE:-/tmp/slot-wake}
DRY=${SLOT_SLEEP_DRY:-0}
PRETEND=${SLOT_SLEEP_PRETEND_SEC:-}

LOG="$ROOT/Logs/sleep.log"
mkdir -p "$ROOT/Logs" 2>/dev/null || true
if ! ( : >> "$LOG" ) 2>/dev/null; then
    LOG=/tmp/slot-sleep.log
fi

log() {
    printf '%s %s\n' "$(date '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo '?')" "$*" >> "$LOG"
}

now_s() {
    date +%s 2>/dev/null || echo 0
}

hall_file() {
    for f in "$SYS"/class/power_supply/*/hallkey; do
        [ -r "$f" ] && { printf '%s\n' "$f"; return 0; }
    done
    return 1
}

# Arm wakeup files whose path or device name looks like the lid, the hall
# sensor, a gpio key (the lid switch and Power live there), or the PMIC.
arm_wakeups() {
    list=$1
    : > "$list"
    # find is on BaseOS (busybox). A missing tree is not a failure.
    find "$SYS" -name wakeup -type f 2>/dev/null | while IFS= read -r f; do
        dir=$(dirname "$f")
        name=""
        [ ! -r "$dir/name" ] || name=$(cat "$dir/name" 2>/dev/null)
        [ -n "$name" ] || [ ! -r "$dir/../name" ] || name=$(cat "$dir/../name" 2>/dev/null)
        case "$f $name" in
            *[Hh]all*|*[Ll]id*|*[Gg]pio*|*[Aa]xp*|*[Pp]ower*)
                if printf 'enabled\n' > "$f" 2>/dev/null; then
                    printf '%s\n' "$f" >> "$list"
                    log "wakeup armed: $f ${name:+($name)}"
                else
                    log "wakeup not writable: $f"
                fi
                ;;
        esac
    done
    wc -l < "$list" | tr -d ' '
}

clear_super_standby() {
    f="$SYS/class/power_supply/axp2202-battery/os_sleep"
    [ -w "$f" ] || return 0
    old=$(cat "$f" 2>/dev/null || echo '?')
    printf '%s\n' "$old" > "$ROOT/Logs/os_sleep.was" 2>/dev/null || printf '%s\n' "$old" > /tmp/slot-os-sleep.was
    if printf '0\n' > "$f" 2>/dev/null; then
        # 16 is Super Standby. BaseOS sets it at boot. It drops the hall sensor.
        log "os_sleep $old -> 0"
    else
        log "os_sleep $old could not be cleared"
    fi
}

restore_super_standby() {
    was="$ROOT/Logs/os_sleep.was"
    [ -r "$was" ] || was=/tmp/slot-os-sleep.was
    f="$SYS/class/power_supply/axp2202-battery/os_sleep"
    [ -r "$was" ] && [ -w "$f" ] || return 0
    old=$(cat "$was" 2>/dev/null || true)
    [ -n "$old" ] && printf '%s\n' "$old" > "$f" 2>/dev/null && log "os_sleep restored to $old"
    rm -f "$ROOT/Logs/os_sleep.was" /tmp/slot-os-sleep.was
}

pick_mem_sleep() {
    f="$SYS/power/mem_sleep"
    [ -r "$f" ] || { log "mem_sleep: no file"; return 0; }
    opts=$(cat "$f" 2>/dev/null || echo '?')
    log "mem_sleep options: $opts"
    echo "$opts" | grep -qw deep || return 0
    if [ -w "$f" ] && printf 'deep\n' > "$f" 2>/dev/null; then
        log "mem_sleep selected deep"
    fi
}

panel() { # blank|unblank
    blank=$SYS/class/graphics/fb0/blank
    if [ -w "$blank" ]; then
        if [ "$1" = blank ]; then
            printf '1\n' > "$blank" 2>/dev/null || log "fb blank failed"
        else
            printf '0\n' > "$blank" 2>/dev/null || log "fb unblank failed"
        fi
    fi
    for dir in "$SYS"/class/backlight/*; do
        [ -d "$dir" ] || continue
        if [ "$1" = blank ]; then
            [ -w "$dir/bl_power" ] && printf '1\n' > "$dir/bl_power" 2>/dev/null
            [ -w "$dir/brightness" ] && printf '0\n' > "$dir/brightness" 2>/dev/null
        else
            [ -w "$dir/bl_power" ] && printf '0\n' > "$dir/bl_power" 2>/dev/null
        fi
    done
}

state_has_mem() {
    [ -r "$SYS/power/state" ] && grep -qw mem "$SYS/power/state"
}

try_mem() {
    start=$(now_s)
    printf 'mem\n' > "$SUSPENDING"
    if [ "$DRY" = 1 ]; then
        elapsed=${PRETEND:-2}
        log "dry run, pretending mem lasted ${elapsed}s"
    else
        log "requesting mem"
        printf 'mem\n' > "$SYS/power/state" 2>> "$LOG" || log "mem write failed"
        end=$(now_s)
        elapsed=$((end - start))
    fi
    log "mem returned after ${elapsed}s"
    # A write that comes straight back did not suspend.
    [ "$elapsed" -ge 1 ]
}

main() {
    rm -f "$WAKE" "$SUSPENDING"
    log "---- sleep start ----"
    hall=$(hall_file || true)
    log "hallkey: ${hall:-none}"
    clear_super_standby
    pick_mem_sleep
    list=${SLOT_SLEEP_LIST:-/tmp/slot-wakeups}
    armed=$(arm_wakeups "$list")
    log "wakeup files armed: $armed"
    if command -v alsactl >/dev/null 2>&1; then
        alsactl store >/dev/null 2>&1 || log "alsactl store failed"
    fi
    mode=standby
    if state_has_mem && [ "${armed:-0}" -gt 0 ]; then
        if try_mem; then
            mode=mem
        else
            log "mem did not sleep; using screen-off standby"
        fi
    else
        log "mem not wakeable by the lid (state mem=$(state_has_mem && echo yes || echo no), armed=$armed); using screen-off standby"
    fi
    panel unblank
    if command -v alsactl >/dev/null 2>&1; then
        alsactl restore >/dev/null 2>&1 || log "alsactl restore failed"
    fi
    restore_super_standby
    rm -f "$SUSPENDING"
    log "mode=$mode"
    if [ "$mode" = mem ]; then
        exit 0
    fi
    exit 2
}

self_test() {
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT INT TERM
    sys=$tmp/sys
    root=$tmp/card
    mkdir -p "$sys/power" "$sys/class/power_supply/axp2202-battery" \
        "$sys/class/graphics/fb0" "$sys/class/backlight/backlight" \
        "$sys/devices/hall/power" "$root/Logs"
    printf 'freeze mem disk\n' > "$sys/power/state"
    printf 's2idle [deep]\n' > "$sys/power/mem_sleep"
    printf '16\n' > "$sys/class/power_supply/axp2202-battery/os_sleep"
    printf '0\n' > "$sys/class/power_supply/axp2202-battery/hallkey"
    printf 'disabled\n' > "$sys/devices/hall/power/wakeup"
    printf '0\n' > "$sys/class/graphics/fb0/blank"
    printf '0\n' > "$sys/class/backlight/backlight/bl_power"
    printf '200\n' > "$sys/class/backlight/backlight/brightness"
    # A lid wakeup is armed and mem would have lasted: exit 0, os_sleep restored.
    SLOT_ROOT=$root SLOT_SLEEP_SYS=$sys SLOT_SLEEP_DRY=1 SLOT_SLEEP_PRETEND_SEC=2 \
        SLOT_SLEEP_MARKER=$tmp/suspending SLOT_SLEEP_WAKE=$tmp/wake \
        sh "$0" >/dev/null 2>&1
    code=$?
    [ "$code" = 0 ]
    grep -q 'mode=mem' "$root/Logs/sleep.log"
    grep -q 'os_sleep 16 -> 0' "$root/Logs/sleep.log"
    grep -q 'wakeup armed:' "$root/Logs/sleep.log"
    [ "$(cat "$sys/devices/hall/power/wakeup")" = enabled ]
    [ "$(cat "$sys/class/power_supply/axp2202-battery/os_sleep")" = 16 ]
    [ "$(cat "$sys/power/mem_sleep")" = deep ]
    [ "$(cat "$sys/class/graphics/fb0/blank")" = 0 ]
    # No wakeup file that matches: standby, exit 2. mem must not be required.
    rm -f "$sys/devices/hall/power/wakeup"
    : > "$root/Logs/sleep.log"
    SLOT_ROOT=$root SLOT_SLEEP_SYS=$sys SLOT_SLEEP_DRY=1 \
        SLOT_SLEEP_MARKER=$tmp/suspending SLOT_SLEEP_WAKE=$tmp/wake \
        sh "$0" >/dev/null 2>&1
    code=$?
    [ "$code" = 2 ]
    grep -q 'mode=standby' "$root/Logs/sleep.log"
    # mem returns at once: also standby.
    printf 'disabled\n' > "$sys/devices/hall/power/wakeup"
    : > "$root/Logs/sleep.log"
    SLOT_ROOT=$root SLOT_SLEEP_SYS=$sys SLOT_SLEEP_DRY=1 SLOT_SLEEP_PRETEND_SEC=0 \
        SLOT_SLEEP_MARKER=$tmp/suspending SLOT_SLEEP_WAKE=$tmp/wake \
        sh "$0" >/dev/null 2>&1
    code=$?
    [ "$code" = 2 ]
    grep -q 'mem did not sleep' "$root/Logs/sleep.log"
    echo "sleep.sh: self-test passed"
}

case "${1:-}" in
    --self-test) self_test ;;
    *) main ;;
esac
