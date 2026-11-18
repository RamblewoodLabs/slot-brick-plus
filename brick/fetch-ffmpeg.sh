#!/bin/sh
# Fetch a static aarch64 ffmpeg for HOME → Videos. Not stored in git: it is GPL-3.0
# and large. Playback also works with ffmpeg already on PATH, or a binary copied to
# Slot/System/ffmpeg by hand.
set -eu
cd "$(dirname "$0")/.."
OUT=brick/vendor/ffmpeg
if [ -x "$OUT" ]; then
    echo "ffmpeg already at $OUT"
    exit 0
fi
mkdir -p brick/vendor
URL="${FFMPEG_URL:-https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-arm64-static.tar.xz}"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
echo "Fetching ffmpeg from $URL"
curl -fL --retry 3 -o "$tmp/ffmpeg.tar.xz" "$URL"
tar -xJf "$tmp/ffmpeg.tar.xz" -C "$tmp"
bin=$(find "$tmp" -type f -name ffmpeg -perm -111 | head -n 1)
if [ -z "$bin" ]; then
    echo "The archive had no ffmpeg binary" >&2
    exit 1
fi
cp "$bin" "$OUT"
chmod +x "$OUT"
echo "Wrote $OUT ($("$OUT" -version | head -n 1))"
