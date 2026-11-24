#!/bin/sh
# Stop the Slot sync hub and remove its launchd job. The save store is left in place.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
    echo "This hub is installed on the Mac mini (macOS)." >&2
    exit 1
fi
uid=$(id -u)
launchctl bootout "gui/$uid/com.slot.sync.menubar" 2>/dev/null || true
launchctl bootout "gui/$uid/com.slot.sync" 2>/dev/null || true
rm -f "$HOME/Library/LaunchAgents/com.slot.sync.menubar.plist"
rm -f "$HOME/Library/LaunchAgents/com.slot.sync.plist"
rm -rf "$HOME/Applications/Slot Sync.app"
echo "Stopped com.slot.sync and removed the menu bar app."
echo "Saves are still in ~/Library/Application Support/SlotSync"
