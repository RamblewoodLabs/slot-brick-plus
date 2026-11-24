#!/bin/sh
# Stop the Slot sync hub and remove its launchd job. The save store is left in place.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
    echo "This hub is installed on the Mac mini (macOS)." >&2
    exit 1
fi
uid=$(id -u)
plist="$HOME/Library/LaunchAgents/com.slot.sync.plist"
launchctl bootout "gui/$uid/com.slot.sync" 2>/dev/null || true
rm -f "$plist"
echo "Stopped com.slot.sync."
echo "Saves are still in ~/Library/Application Support/SlotSync"
