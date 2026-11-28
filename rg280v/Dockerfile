# Build box for Slotted on the Anbernic RG280V (OpenDingux / JZ4770).
# The gcw0 toolchain is an x86-64 Linux sysroot, so the image is pinned to
# amd64. On an Apple-silicon Mac, Docker runs it under emulation.
# rg280v/docker-build.sh also passes --platform linux/amd64.
#
# This image is only for rg280v/. It does not replace brick/Dockerfile or
# rg35xxsp/Dockerfile, and it does not change rust-toolchain.toml at the
# repo root (Brick and SP stay on 1.96).
FROM amd64/debian:bookworm-slim

# python3 is omitted on purpose: Debian bookworm python3.11 segfaults in
# py3compile under qemu-user on Apple silicon, and the hello build does not
# need it (the target JSON and icon are generated on the Mac and committed).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl file make xz-utils \
 && rm -rf /var/lib/apt/lists/*

# Official OpenDingux *compiler* toolchain (gcc, uClibc, SDL2, GLES).
# Not BIOS, keys, or device firmware. Checksum pinned.
ARG TOOLCHAIN_SHA256=25eb241e7088d93656d2aa42f57ba288849aa57415e0067428e8260283209c26
COPY opendingux-gcw0-toolchain.2022-09-22.tar.xz /tmp/tc.tar.xz
RUN echo "$TOOLCHAIN_SHA256  /tmp/tc.tar.xz" | sha256sum -c - \
 && tar -xJf /tmp/tc.tar.xz -C /opt \
 && rm /tmp/tc.tar.xz \
 && /opt/gcw0-toolchain/relocate-sdk.sh

# Nightly is required for -Z build-std on this Tier 3 target. Pinned so the
# image does not float. rust-src is the stdlib source build-std compiles.
# The workspace rust-toolchain.toml (1.96) is not used in this image.
ARG RUST_NIGHTLY=nightly-2026-11-25
ENV RUSTUP_TOOLCHAIN=$RUST_NIGHTLY \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/opt/gcw0-toolchain/bin:$PATH \
    GCW0_TOOLCHAIN=/opt/gcw0-toolchain \
    GCW0_SYSROOT=/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --no-modify-path \
      --profile minimal --default-toolchain "$RUST_NIGHTLY" -c rust-src \
 && chmod -R a+rX /usr/local/rustup /usr/local/cargo

ENV PATH=/opt/gcw0-toolchain/bin:/usr/local/cargo/bin:$PATH \
    PKG_CONFIG_SYSROOT_DIR=/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot \
    PKG_CONFIG_LIBDIR=/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot/usr/lib/pkgconfig \
    PKG_CONFIG_PATH=/opt/gcw0-toolchain/mipsel-gcw0-linux-uclibc/sysroot/usr/lib/pkgconfig \
    CC=mipsel-linux-gcc \
    AR=mipsel-linux-ar \
    CARGO_TARGET_MIPSEL_GCW0_LINUX_UCLIBC_LINKER=mipsel-linux-gcc \
    CARGO_TARGET_MIPSEL_GCW0_LINUX_UCLIBC_AR=mipsel-linux-ar

# Host gcc is only for cargo build scripts (compiler_builtins, libc, this
# crate's build.rs). The mipsel compiler is the gcw0 toolchain, not this one.
# Kept as a later layer so the rustup layer stays cached.
RUN apt-get update \
 && apt-get install -y --no-install-recommends gcc libc6-dev \
 && rm -rf /var/lib/apt/lists/*
