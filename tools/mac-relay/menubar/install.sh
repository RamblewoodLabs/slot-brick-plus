#!/bin/bash
# Build, install to ~/Applications, and start at login with a LaunchAgent.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
app=$(OUT="$(mktemp -d)" bash "$here/build.sh")
dest="$HOME/Applications/Slot Relay.app"
mkdir -p "$HOME/Applications"
launchctl bootout "gui/$(id -u)/com.slot.relay.menubar" 2>/dev/null || true
pkill -x SlotRelayMenu 2>/dev/null || true
rm -rf "$dest"
cp -R "$app" "$dest"
plist="$HOME/Library/LaunchAgents/com.slot.relay.menubar.plist"
sed "s#__APP__#$dest/Contents/MacOS/SlotRelayMenu#" "$here/com.slot.relay.menubar.plist" > "$plist"
launchctl bootstrap "gui/$(id -u)" "$plist"
echo "Installed $dest and started it. It also starts at login."
