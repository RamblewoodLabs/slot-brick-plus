#!/bin/bash
# Remove the menu bar item. The relay itself is left as it is.
set -u
launchctl bootout "gui/$(id -u)/com.slot.relay.menubar" 2>/dev/null
pkill -x SlotRelayMenu 2>/dev/null
rm -f "$HOME/Library/LaunchAgents/com.slot.relay.menubar.plist"
rm -rf "$HOME/Applications/Slot Relay.app"
echo "Removed Slot Relay menu."
