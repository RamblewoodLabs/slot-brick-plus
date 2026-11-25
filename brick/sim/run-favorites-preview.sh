#!/bin/sh
# Render Favorites-shelf previews for the Brick (1024x768) and the SP (640x480).
# Writes PNGs under preview/favorites/{brick,sp}. Does not update brick/sim/golden.
set -eu
cd "$(dirname "$0")/../.."
export PATH=/opt/homebrew/bin:/usr/local/bin:$PATH

IMAGE=slot-brick-sim
BIN=target-sim/release/slot
SIM_TIME=1767225600

docker_run() {
    docker run --rm -u "$(id -u):$(id -g)" -v "$PWD":/src -w /src \
        -e HOME=/tmp -e CARGO_HOME=/src/.docker-cache/cargo "$@"
}

echo "building sim binary..."
docker build -q -t "$IMAGE" -f brick/sim/Dockerfile brick/sim >/dev/null
mkdir -p .docker-cache/cargo
docker_run "$IMAGE" sh brick/sim/build-cores.sh
docker_run "$IMAGE" cargo build --locked --release --target-dir target-sim \
    -p slot --no-default-features --features sim

run_preview() {
    panel="$1"
    rel="$2"
    run="target-sim/run/favorites-$panel"
    rm -rf "$run"
    mkdir -p "$run/hw" "$rel"
    docker_run "$IMAGE" "$BIN" --init-root "$run/card/Slot" >/dev/null
    cp brick/card/Slot/System/*.txt "$run/card/Slot/System/" 2>/dev/null || true
    if [ -d target-sim/cores ]; then
        cp target-sim/cores/*_libretro.so "$run/card/Slot/System/" 2>/dev/null || true
    fi
    cp -r brick/card/Slot/System/Fonts "$run/card/Slot/System/"
    for cart in "GBA/Sim Quest.gba" "GB/Pocket Test.gb" "SNES/Super Sample.sfc"; do
        mkdir -p "$run/card/Slot/Games/$(dirname "$cart")"
        printf 'slot simulator stand-in cartridge\n' > "$run/card/Slot/Games/$cart"
    done
    python3 -I brick/sim/make-rom.py "$run/card/Slot/Games/GB/Pocket Test.gb" 2>/dev/null || true
    sh brick/sim/scenarios/favorites.card.sh "$run/card/Slot"
    echo "running $panel into $rel ..."
    if docker_run \
        -e SLOT_ROOT="/src/$run/card/Slot" \
        -e SLOT_HW_ROOT="/src/$run/hw" \
        -e SLOT_SIM_TIME="$SIM_TIME" \
        -e SLOT_SIM_SCRIPT="/src/brick/sim/scenarios/favorites.sim" \
        -e SLOT_NTP_SERVER= \
        -e SLOT_SIM_VERSION=v0.1 \
        -e SLOT_SIM_CARD="12.3/63.9 GB free" \
        -e SLOT_SIM_OUT="/src/$rel" \
        -e SLOT_SIM_GOLDEN="/src/$rel" \
        -e SLOT_SIM_UPDATE=1 \
        -e SLOT_SIM_PANEL="$panel" \
        "$IMAGE" timeout 300 "$BIN" > "$rel/run.log" 2>&1
    then
        if [ -d "$rel/favorites" ]; then
            mv "$rel/favorites/"*.png "$rel/" 2>/dev/null || true
            rmdir "$rel/favorites" 2>/dev/null || true
        fi
        echo "OK $panel -> $rel ($(ls "$rel"/*.png 2>/dev/null | wc -l | tr -d ' ') pngs)"
    else
        echo "FAIL $panel; log: $rel/run.log"
        tail -40 "$rel/run.log" || true
        return 1
    fi
}

run_preview 1024x768 preview/favorites/brick
run_preview 640x480 preview/favorites/sp
echo PREVIEW_DONE
