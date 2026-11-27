#!/bin/sh
# Builds the Brick's Python runtime for HOME → Apps (Slot/Apps/python): CPython 3.12 from
# python-build-standalone (glibc 2.17 and up, so it runs on stock's glibc 2.33 and Slot OS's
# 2.36), pygame 2.6.1 with its own SDL2, Slot's framebuffer shim (slot_fb.py, sitecustomize.py),
# and SoundTouch 2.3.2 built here against glibc 2.31 (lib/compat/libSoundTouch.so.1): GrainBrick's
# own copy was built on Ubuntu 24.04 and needs glibc 2.38, and its LGPL notice invites swapping in
# an ABI-compatible build. run-app.sh puts lib/compat on LD_LIBRARY_PATH, ahead of its $ORIGIN.
# Needs Docker on an arm64 machine (or with arm64 emulation). Output: brick/python-runtime/runtime/python
set -eu
cd "$(dirname "$0")/../.."
CACHE=.docker-cache/python-runtime
mkdir -p "$CACHE"
grep -v '^#' brick/python-runtime/sources.txt | while read -r sum name url; do
    [ -n "$sum" ] || continue
    if ! echo "$sum  $CACHE/$name" | shasum -a 256 -c - >/dev/null 2>&1; then
        echo "fetching $name"
        curl -fsSL -o "$CACHE/$name.part" "$url"
        echo "$sum  $CACHE/$name.part" | shasum -a 256 -c - >/dev/null || { echo "$name: sha256 differs" >&2; rm -f "$CACHE/$name.part"; exit 1; }
        mv "$CACHE/$name.part" "$CACHE/$name"
    fi
done
OUT=brick/python-runtime/runtime
rm -rf "$OUT" && mkdir -p "$OUT"
docker run --rm --platform linux/arm64 -v "$PWD":/src -w /src debian:bullseye sh -euc '
    # The bullseye mirrors are being retired: install from the snapshot the image was made from.
    printf "deb http://snapshot.debian.org/archive/debian/20260824T000000Z bullseye main\ndeb http://snapshot.debian.org/archive/debian-security/20260824T000000Z bullseye-security main\n" > /etc/apt/sources.list
    rm -f /etc/apt/sources.list.d/*
    apt-get -qq -o Acquire::Check-Valid-Until=false update >/dev/null
    apt-get -qq install -y --no-install-recommends ca-certificates g++ make autoconf automake libtool pkg-config file binutils >/dev/null
    C=/src/.docker-cache/python-runtime; O=/tmp/rt; mkdir -p $O
    tar -xzf "$C"/cpython-3.12.16+*-install_only_stripped.tar.gz -C "$O"
    P="$O/python"
    "$P/bin/python3" -m pip install -q --no-index --no-compile --disable-pip-version-check "$C"/pygame-2.6.1-*.whl
    SITE="$P/lib/python3.12/site-packages"
    cp /src/brick/python-runtime/slot_fb.py /src/brick/python-runtime/sitecustomize.py "$SITE/"
    # SoundTouch, as Ubuntu builds it (default configure), for glibc 2.31.
    mkdir -p /tmp/st && tar -xzf "$C/soundtouch-2.3.2.tar.gz" -C /tmp/st
    cd /tmp/st/soundtouch && ./bootstrap >/dev/null 2>&1 && ./configure -q --disable-static >/dev/null && make -s -j"$(nproc)" >/dev/null 2>&1
    mkdir -p "$P/lib/compat"
    cp -L source/SoundTouch/.libs/libSoundTouch.so.1 "$P/lib/compat/"
    strip --strip-unneeded "$P/lib/compat/libSoundTouch.so.1"
    # Not needed to run apps on the Brick: the test suite, IDLE, Tk, ensurepip, pip, pygame examples, docs and tests;
    # and dbm, which links Berkeley DB (Sleepycat licence).
    rm -rf "$P/lib/python3.12/test" "$P/lib/python3.12/idlelib" "$P/lib/python3.12/tkinter" "$P/lib/python3.12/turtledemo" \
           "$P/lib/python3.12/ensurepip" "$P/lib/python3.12/lib2to3" "$P/lib/tcl"* "$P/lib/tk"* "$P/lib/itcl"* "$P/lib/thread"* \
           "$SITE/pip" "$SITE"/pip-*.dist-info "$P/bin/pip"* "$P/bin/idle"* "$P/bin/pydoc"* \
           "$SITE/pygame/examples" "$SITE/pygame/tests" "$SITE/pygame/docs" "$P/share" "$P/include" \
           "$P/lib/python3.12/pydoc_data" "$P/lib/python3.12/lib-dynload"/_tkinter* "$P/lib/python3.12/lib-dynload"/_dbm* "$P/lib/python3.12/config-3.12"* \
           "$P/lib/libtcl"* "$P/lib/libtk"* "$P/lib/pkgconfig" "$P/lib/libpython"* \
           "$P/bin/2to3"* "$P/bin/python3"*-config
    # bin/python3 has libpython built in; the card gets one real file, no links.
    cp -L "$P/bin/python3" "$P/bin/python3.real" && rm -f "$P/bin/python" "$P/bin/python3" "$P/bin/python3.12" && mv "$P/bin/python3.real" "$P/bin/python3"
    find "$P" -name "__pycache__" -type d -prune -exec rm -rf {} +
    # Compiled once here so the Brick does not write .pyc files to the card on first run.
    "$P/bin/python3" -m compileall -q -j0 "$P/lib/python3.12" >/dev/null 2>&1 || true
    SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy SLOT_FB=/dev/null LD_LIBRARY_PATH="$P/lib/compat" "$P/bin/python3" -c "
import ctypes, ssl, sqlite3, zlib, json, wave, audioop, pygame
ctypes.CDLL(\"libSoundTouch.so.1\")
pygame.display.init(); pygame.joystick.init()
pygame.display.set_mode((1024, 768), pygame.FULLSCREEN); pygame.display.flip()
print(\"smoke ok: python\", __import__(\"sys\").version.split()[0], \"pygame\", pygame.version.ver, \"SDL\", pygame.get_sdl_version())"
    echo "newest glibc any runtime file needs:"
    find "$P" -type f \( -name "*.so*" -o -path "*/bin/*" \) -exec sh -c "objdump -T \"\$1\" 2>/dev/null | grep -o \"GLIBC_[0-9.]*\"" _ {} \; | sort -uV | tail -1
    # The card is FAT32: no symlinks, and names differing only in case collide.
    find "$P" | tr A-Z a-z | sort | uniq -d | grep . && { echo "names collide on FAT32" >&2; exit 1; }
    cp -RL "$P" /src/brick/python-runtime/runtime/
    chown -R "$(stat -c %u /src)":"$(stat -c %g /src)" /src/brick/python-runtime/runtime
'
du -sh "$OUT/python"
