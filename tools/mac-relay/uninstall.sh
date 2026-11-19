#!/bin/bash
# Stop the relay and remove its launchd jobs. The download folder is left in place.
set -euo pipefail
uid=$(id -u)
agents="$HOME/Library/LaunchAgents"
launchctl bootout "gui/$uid/com.slot.relay" 2>/dev/null || true
launchctl bootout "gui/$uid/com.slot.relay.ytdlp" 2>/dev/null || true
rm -f "$agents/com.slot.relay.plist" "$agents/com.slot.relay.ytdlp.plist"
echo "Relay stopped. Downloads remain in ~/Library/Application Support/SlotRelay"
