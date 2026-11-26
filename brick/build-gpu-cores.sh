#!/bin/sh
# Builds vendor/mupen64plus_next_libretro.so and vendor/ppsspp_libretro.so from their pinned
# sources (vendor/*-src.tar.gz) in an arm64 container; see brick/gpu-cores/build.sh.
set -eu
cd "$(dirname "$0")/.."
docker build --platform linux/arm64 -q -t slot-gpu-cores brick/gpu-cores > /dev/null
docker run --platform linux/arm64 --rm -v "$PWD":/src -w /src -e JOBS="${JOBS:-8}" slot-gpu-cores sh brick/gpu-cores/build.sh
