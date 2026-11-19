#!/bin/bash
# Install the Slot download relay on this Mac and leave it running.
# Prints the three lines to put in Slot/Config/videos.txt on the Brick.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
if [ "$(uname -s)" != "Darwin" ]; then
    echo "This relay is installed on the Mac mini (macOS)." >&2
    exit 1
fi
if [ -x /opt/homebrew/bin/brew ]; then
    eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -x /usr/local/bin/brew ]; then
    eval "$(/usr/local/bin/brew shellenv)"
else
    echo "Install Homebrew first: https://brew.sh" >&2
    exit 1
fi

brew install ffmpeg yt-dlp

support="$HOME/Library/Application Support/SlotRelay"
logs="$HOME/Library/Logs"
agents="$HOME/Library/LaunchAgents"
mkdir -p "$support" "$logs" "$agents"
config="$support/config.json"
log="$logs/slot-relay.log"

if [ ! -f "$config" ]; then
    token=$(openssl rand -hex 24)
    python3 - "$config" "$token" "$(command -v yt-dlp)" "$(command -v ffmpeg)" <<'PY'
import json, sys
path, token, ytdlp, ffmpeg = sys.argv[1:]
open(path, "w").write(json.dumps({
    "token": token,
    "bind": "0.0.0.0",
    "port": 8787,
    "dir": path.rsplit("/", 1)[0],
    "keep_seconds": 86400,
    "ytdlp": ytdlp,
    "ffmpeg": ffmpeg,
}, indent=2) + "\n")
PY
fi

token=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["token"])' "$config")
python=$(command -v python3)
update="$support/update-ytdlp.sh"
cp "$here/update-ytdlp.sh" "$update"
chmod +x "$update"

write_plist() {
    src=$1
    dest=$2
    sed -e "s|__PYTHON__|$python|g" \
        -e "s|__RELAY__|$here/relay.py|g" \
        -e "s|__CONFIG__|$config|g" \
        -e "s|__UPDATE__|$update|g" \
        -e "s|__LOG__|$log|g" \
        "$src" > "$dest"
}

write_plist "$here/com.slot.relay.plist" "$agents/com.slot.relay.plist"
write_plist "$here/com.slot.relay.ytdlp.plist" "$agents/com.slot.relay.ytdlp.plist"

uid=$(id -u)
launchctl bootout "gui/$uid/com.slot.relay" 2>/dev/null || true
launchctl bootout "gui/$uid/com.slot.relay.ytdlp" 2>/dev/null || true
launchctl bootstrap "gui/$uid" "$agents/com.slot.relay.plist"
launchctl bootstrap "gui/$uid" "$agents/com.slot.relay.ytdlp.plist"
launchctl enable "gui/$uid/com.slot.relay"
launchctl kickstart -k "gui/$uid/com.slot.relay"

name=$(scutil --get LocalHostName 2>/dev/null || echo Mac-mini)
echo
echo "Relay is running. On the Brick, add these lines to Slot/Config/videos.txt:"
echo "relay_host=${name}.local"
echo "relay_port=8787"
echo "relay_token=${token}"
echo
echo "If the Brick cannot see ${name}.local, use the Mac's LAN address as relay_host."
echo "Log: $log"
