# Building, Simulator and CI

## Building

| Target | Command | Output |
| :--- | :--- | :--- |
| TrimUI Brick card | `sh brick/docker-build.sh` | `dist-brick/` |
| RG35XX SP card | `sh rg35xxsp/docker-build.sh` (needs Docker Desktop on the Mac mini) | `dist-rg35xxsp/` |
| Rebuild the five SP cores from source | `sh rg35xxsp/docker-build.sh --cores` | |
| Classic cores | `brick/build-cores.sh` | |
| PR #31 cores | `brick/build-cores-systems2.sh`, plus `brick/build-gpu-cores.sh` for N64 and PSP | checked by `brick/cores-systems2.sha256` and `brick/gpu-cores/sources.sha256` |
| Slot OS image | `brick/os/build.sh` (see `brick/os/README.md`) | |

The toolchain is the TrimUI Linaro gcc 7.5 (glibc 2.25) with Rust for `aarch64-unknown-linux-gnu`, profile `device`.
The image is `linux/amd64` and runs under emulation on Apple silicon. Linaro gcc sometimes crashes under QEMU there;
retrying works (see `PERF-PROGRESS.md` on PR #27).

## Checks to run before a PR

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
sh brick/sim.sh unit      # unit tests in the simulator image
sh brick/sim.sh test      # screenshot scenarios vs brick/sim/golden
sh brick/sim.sh update    # rewrite goldens, only when the screen is meant to change
```

## Simulator

`task sim` (or `sh brick/sim.sh serve [PORT]`) runs the real Brick build (`--features sim`) in Docker, drawing with
Mesa into an offscreen 1024×768 buffer. Open <http://127.0.0.1:8765/>.

- Keys: arrows; X = A, Z = B, S = X, A = Y; Q/W = L1/R1; 1/2 = L2/R2; Enter = Start; Shift = Select; M = Menu;
  P = Power; `-` and `=` for volume.
- The card is kept in `target-sim/live/card`. `FRESH=1` starts over. `MODEL=pro` simulates a Brick Pro.
- Scenarios are in `brick/sim/scenarios/*.sim`, and their goldens are in `brick/sim/golden`.
- It can't tell you about H700 speed, clocks, audio underruns or Wi-Fi.

## CI (`.github/workflows`)

| Workflow | What it does |
| :--- | :--- |
| `changelog.yml` | `brick/changelog-check.sh`. A PR that touches `crates/*/src`, `crates/*/assets`, `brick/card/`, `vendor/` or `Cargo.lock` must add a line under `## Unreleased` in `brick/CHANGELOG.md`. Docs-only PRs (like this wiki) pass without one. |
| `sim.yml` | Simulator unit tests and screenshot scenarios |
| `brick-binary.yml` | On a `v*` tag, builds the Brick `slot` binary and `slot.sha256` for the release |
| `dev-build.yml` | Every merge to `main` republishes the `dev` pre-release (stock zip, Slot OS images, update files) |
| `release.yml` | On a `vX.Y` tag on `main`: checks, tests, builds, signs `update.txt` and publishes |

## Releasing

See `docs/releasing.md` and the root README's "Making a release": bump the version in the five places it lists, move
`## Unreleased` under `## vX.Y`, merge a `Release vX.Y` PR, then tag on `main`. `main` takes pull requests only.
Don't merge without the owner's explicit go-ahead.

## Working with `gh` in this fork

Always pass `--repo jfink22-design/slot-brick-plus`. Without it, `gh` resolves to the upstream repository.
