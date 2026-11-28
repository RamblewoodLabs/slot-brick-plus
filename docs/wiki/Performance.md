# Performance

## On main

- **Adaptive CPU clocks** keep audio smooth. The minimum CPU clock in games fixes crackling audio.
- Frames that haven't changed aren't redrawn.
- Labels and cart faces load lazily, and `library.txt` is written on a background thread.
- Startup waits up to 30 s for slow-mounting cards.

## In PR #27: faster menus and All/Favorites switching

Jeff reported that switching All ↔ Favorites was slow on the RG35XX SP with about 6,189 games, and that menus felt
sluggish. Profiling (notes in `PERF-PROGRESS.md` on that branch) found three causes:

1. Every setting change, SELECT sort, Systems toggle and All/Favorites switch wrote `slot.state` **synchronously**
   (two fsyncs) on the UI thread. On an SD card that can take tens to hundreds of milliseconds.
2. `App::arrange()` scanned every cart against every other (O(n²)) and cloned whole lists.
3. Every library lookup allocated a new string key.

Fixes: `slot.state` is written in the background (and still flushed on sleep or power-off), shelves are rearranged in
memory from an index, and the library is a two-level map. Measured on the Mac mini simulator, not the SP:

| Operation | Before | After |
| :--- | ---: | ---: |
| arrange (all games + Favorites) | 12.6 ms | 2.1 ms |
| toggle Favorites-only + arrange | 7.3 ms | 1.7 ms |
| toggle show-Favorites + arrange | 1.8 ms | 1.2 ms |

On the SP, the bigger win is that the UI no longer waits on SD-card fsyncs.

## N64, DS and PSP (in PR #31)

These run near the H700's limit. Stick to the curated lists in [Systems and Cores](Systems-and-Cores.md#curated-n64-ds-and-psp-lists).
N64 and PSP draw with GLES. Some PSP titles are listed as "100% at 2x". The speed column in each list comes from
community reports and is **needs confirmation** on this build.

## What the simulator can't tell you

The Docker simulator runs the real app on a PC CPU. It can't measure H700 speed, CPU clocks, audio underruns or Wi-Fi
(`brick/sim/README.md`).
