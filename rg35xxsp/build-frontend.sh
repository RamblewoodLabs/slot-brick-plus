#!/bin/sh
# Cross-compile the RG35XX SP frontend. Called from docker-build.sh, which has
# already put the Brick toolchain on PATH. Same target triple and device profile
# as the Brick binary; the cargo feature is rg35xxsp instead of brick.
set -eu
: "${BRICK_TOOLCHAIN:?Set BRICK_TOOLCHAIN to the extracted aarch64-linux-gnu-7.5.0-linaro directory}"
cd "$(dirname "$0")/.."
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-gcc"
export CC_aarch64_unknown_linux_gnu="$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-gcc"
export AR_aarch64_unknown_linux_gnu="$BRICK_TOOLCHAIN/bin/aarch64-linux-gnu-ar"
cargo build --locked --profile device --target aarch64-unknown-linux-gnu -p slot --no-default-features --features rg35xxsp
