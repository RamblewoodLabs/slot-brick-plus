#!/bin/sh
# Build the Brick `slot` binary and its sha256 checksum, the two files HOME → Online →
# Update downloads from a release of this repository.
# Usage: sh brick/publish-slot.sh [OUT]
#   OUT  directory to write slot and slot.sha256 into (default dist-release)
set -eu
cd "$(dirname "$0")/.."
out="${1:-dist-release}"
mkdir -p "$out"
sh brick/build-frontend.sh
cp target/aarch64-unknown-linux-gnu/device/slot "$out/slot"
chmod +x "$out/slot"
# sha256sum's own line: the hex, two spaces, the name. Update reads the hex.
( cd "$out" && sha256sum slot > slot.sha256 )
echo "Wrote $out/slot and $out/slot.sha256"
