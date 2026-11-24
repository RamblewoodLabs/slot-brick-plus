#!/bin/bash
# Build "Slot Relay.app" (menu bar only, no Dock icon) next to this script.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${OUT:-$here/build}"
app="$out/Slot Relay.app"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
swiftc -O -o "$app/Contents/MacOS/SlotRelayMenu" "$here/SlotRelayMenu.swift"
cp "$here/Info.plist" "$app/Contents/Info.plist"
codesign --force --sign - "$app" >/dev/null 2>&1 || true
echo "$app"
