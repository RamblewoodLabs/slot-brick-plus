#!/bin/sh
# Cross-compile the Slotted hello OPK for the Anbernic RG280V.
#
# Run this on the Mac mini. It needs Docker Desktop, running. The OpenDingux
# gcw0 toolchain is an x86-64 binary, so the build is forced to linux/amd64.
#
# This does not flash a card, does not download BIOS, keys, or firmware, and
# does not touch the Brick or RG35XX SP Docker images.
#
# Usage: sh rg280v/docker-build.sh [OUT]
#   OUT  output directory, default dist-rg280v. Replaced if it exists.
set -eu
cd "$(dirname "$0")/.."

if ! command -v docker >/dev/null 2>&1; then
    echo "Docker is not installed. Install Docker Desktop on the Mac mini, start it, and run this again." >&2
    exit 1
fi
if ! docker info >/dev/null 2>&1; then
    echo "Docker is installed but not running. Start Docker Desktop and run this again." >&2
    exit 1
fi

OUT="${1:-dist-rg280v}"
IMAGE=slot-rg280v-build
TOOLCHAIN_TAR=".docker-cache/rg280v/opendingux-gcw0-toolchain.2022-09-22.tar.xz"
TOOLCHAIN_URL="https://github.com/OpenDingux/buildroot/releases/download/od-2022.09.22/opendingux-gcw0-toolchain.2022-09-22.tar.xz"
TOOLCHAIN_SHA256="25eb241e7088d93656d2aa42f57ba288849aa57415e0067428e8260283209c26"
export DOCKER_DEFAULT_PLATFORM=linux/amd64

mkdir -p .docker-cache/rg280v .docker-cache/cargo-rg280v
if [ ! -f "$TOOLCHAIN_TAR" ]; then
    echo "Downloading the OpenDingux gcw0 compiler toolchain (not BIOS or firmware)..."
    curl -fL --retry 3 -o "$TOOLCHAIN_TAR.partial" "$TOOLCHAIN_URL"
    mv "$TOOLCHAIN_TAR.partial" "$TOOLCHAIN_TAR"
fi
echo "$TOOLCHAIN_SHA256  $TOOLCHAIN_TAR" | shasum -a 256 -c -

# Context is a staging dir so the 153 MB toolchain is copied once, and the
# rest of the repo is not sent as a build context.
STAGE="$(mktemp -d "${TMPDIR:-/tmp}/rg280v-docker.XXXXXX")"
trap 'rm -rf "$STAGE"' EXIT
cp rg280v/Dockerfile "$STAGE/Dockerfile"
cp "$TOOLCHAIN_TAR" "$STAGE/opendingux-gcw0-toolchain.2022-09-22.tar.xz"

docker build --platform linux/amd64 -t "$IMAGE" "$STAGE"

rm -rf "$OUT"
mkdir -p "$OUT"
# Host user, so the OPK is not owned by root. Cargo's download cache stays
# in .docker-cache/cargo-rg280v, separate from the Brick 1.96 cache.
docker run --platform linux/amd64 --rm -u "$(id -u):$(id -g)" \
    -v "$PWD":/src -w /src \
    -e HOME=/tmp \
    -e CARGO_HOME=/src/.docker-cache/cargo-rg280v \
    -e RUST_TARGET_PATH=/src/rg280v \
    -e OUT="$OUT" \
    "$IMAGE" sh /src/rg280v/build-hello.sh

echo "OPK is in $OUT/."
SHARED="/Users/jeff/Shared-Space/rg280v"
if [ -d "$(dirname "$SHARED")" ]; then
    mkdir -p "$SHARED"
    cp -p "$OUT"/slotted-hello.opk "$SHARED/" 2>/dev/null || true
    if [ -f "$OUT/slotted-hello.opk" ]; then
        echo "Copied $OUT/slotted-hello.opk to $SHARED/"
    fi
fi
