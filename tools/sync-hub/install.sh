#!/bin/sh
# Install the Slot continue-anywhere hub on this Mac and leave it running.
# Saves go in ~/Library/Application Support/SlotSync. This is not the old relay.
set -eu
if [ "$(uname -s)" != "Darwin" ]; then
    echo "This hub is installed on the Mac mini (macOS)." >&2
    exit 1
fi
here=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
root="$HOME/Library/Application Support/SlotSync"
config="$root/config.json"
uid=$(id -u)
agents="$HOME/Library/LaunchAgents"
mkdir -p "$root/files" "$agents"
python=$(command -v python3)
if [ ! -f "$config" ]; then
    token=$(python3 -c 'import secrets; print(secrets.token_hex(16))')
    python3 - "$config" "$token" <<'PY'
import json, sys
path, token = sys.argv[1:]
with open(path, "w") as f:
    json.dump({"token": token, "port": 8791, "bind": "0.0.0.0"}, f, indent=2)
    f.write("\n")
os_chmod = __import__("os").chmod
os_chmod(path, 0o600)
PY
fi
token=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["token"])' "$config")
port=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("port", 8791))' "$config")
name=$(scutil --get LocalHostName 2>/dev/null || echo Jeffs-Mac-Mini-2)
plist="$agents/com.slot.sync.plist"
sed -e "s|__PYTHON__|$python|g" \
    -e "s|__HUB__|$here/hub.py|g" \
    -e "s|__ROOT__|$root|g" \
    "$here/com.slot.sync.plist" > "$plist"
launchctl bootout "gui/$uid/com.slot.sync" 2>/dev/null || true
launchctl bootstrap "gui/$uid" "$plist"
launchctl enable "gui/$uid/com.slot.sync"
launchctl kickstart -k "gui/$uid/com.slot.sync"
echo "Hub files: $root"
echo "On each handheld, Slot/Config/sync.txt (leave device blank):"
echo "host=${name}.local"
echo "port=${port}"
echo "token=${token}"
echo "device="
echo "If ${name}.local does not resolve, use the Mac's LAN address as host (this mini is 192.168.7.99)."
