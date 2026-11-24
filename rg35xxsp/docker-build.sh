#!/bin/sh
# Cross-compile Slot for the Anbernic RG35XX SP and pack a TF2 card.
#
# Run this on the Mac mini. It needs Docker Desktop, running, and nothing else:
# no Rust toolchain and no aarch64 compiler on the Mac. The image is the same
# one the Brick build uses (Linaro gcc 7.5, glibc 2.25), so the binary and the
# cores match the Brick card and save states load on both.
#
# The TrimUI toolchain is an x86-64 binary. On an Apple-silicon Mac the build
# is forced to linux/amd64 (the same platform Brick builds use via
# DOCKER_DEFAULT_PLATFORM=linux/amd64) and the base image is amd64/debian.
#
# Docker is not available in the cloud agent that wrote this script. The script
# only talks to a local Docker. If `docker info` fails, it stops and says why.
#
# Usage: sh rg35xxsp/docker-build.sh [--cores] [OUT]
#   --cores  rebuild the libretro cores from the same upstream commits the
#            Brick uses (brick/build-cores.sh). Without this flag the card
#            gets vendor/*.so, the builds already on the Brick card.
#   OUT      output directory, default dist-rg35xxsp. Replaced if it exists.
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

CORES=0
if [ "${1:-}" = --cores ]; then
    CORES=1
    shift
fi
OUT="${1:-dist-rg35xxsp}"
IMAGE=slot-sp-build
# Apple silicon otherwise builds an arm64 image, and the x86-64 toolchain will not run.
export DOCKER_DEFAULT_PLATFORM=linux/amd64

# Context is rg35xxsp/ so the Dockerfile can see nothing but itself. The
# toolchain URL and checksum match brick/Dockerfile.
docker build --platform linux/amd64 -t "$IMAGE" -f rg35xxsp/Dockerfile rg35xxsp
mkdir -p .docker-cache/cargo
rm -rf "$OUT"
# Host user, so the card is not owned by root. Cargo's download cache stays
# in .docker-cache between runs. HOME is inside the container: the Mac's
# home directory is not mounted. --platform keeps the run on amd64 too.
docker run --platform linux/amd64 --rm -u "$(id -u):$(id -g)" -v "$PWD":/src -w /src \
    -e HOME=/tmp -e CARGO_HOME=/src/.docker-cache/cargo -e CORES="$CORES" -e OUT="$OUT" \
    "$IMAGE" sh -euc '
        [ "$CORES" = 0 ] || sh brick/build-cores.sh
        sh rg35xxsp/build-frontend.sh
        sh rg35xxsp/package-card.sh "$OUT"
        file "$OUT/Slot/System/slot"
    '
echo "TF2 card is in $OUT/."
echo "Copy everything inside $OUT/ to the root of the TF2 card. System/launch_frontend.sh must sit at that root, and the games live in Slot/."
