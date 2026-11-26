# PERF-PROGRESS (MGT-14)

Started: 2026-11-25 16:43 Pacific (America/Los_Angeles)
Worktree: ~/sbp-perf
Branch: cursor/perf-menu-nav
Base: main f753740 (merge of PR #26 Favorites shelf)

Goal: Switching All games <-> Favorites is really slow on the RG35XX SP;
all menus feel a bit slow. Make navigation fast. Do not merge. Do not
write to any card.

## Status

- [x] Worktree created off main
- [ ] Profile code paths (arrange, favorites toggle, menus, textures, IO)
- [ ] Measure before timings
- [ ] Fix biggest offenders
- [ ] CI-equivalent checks
- [ ] Push + PR against main (do not merge)
- [ ] Build dist-rg35xxsp and dist-brick
- [ ] Stage \\192.168.7.99\Shared-Space\sp-update-<sha> with SHA-256 manifest

## Notes

Jeff tested main f753740 on the RG35XX SP with ~6,189 games. SD card
random IO is slow. Hypotheses to verify (not assumed):

- full library rescans or label/cover reloads on each shelf switch
- rebuilding the Favorites list from disk
- synchronous writes of slot.state or library.txt on each toggle
- loading or scaling all PNG/label textures up front instead of lazily
- redrawing or re-rendering 3D carts every frame or on each move
- per-frame sleeps or fixed frame delays and input repeat timing
- fs stats across 6,189 games


## Before timings (2026-11-25, slot-brick-sim arm64 release on the Mac mini)

Host/desktop CPU, not the RG35XX SP. `write_slot_state` here is a fast disk;
the SP's SD card is much slower at random IO and fsync.

- Library::get + any_favorite over 6,189 carts: mean 115–121 µs
- write_slot_state (atomic_write + 2 fsyncs) on this disk: mean 1.10 ms (min 0.69, max 2.40)
- atomic_write 32 bytes on this disk: mean 0.996 ms
- App::arrange (all games + Favorites shelf): mean 12.61 ms (min 12.30, max 13.33)
- toggle favorites_only + arrange: mean 7.29 ms (median 12.01; min 2.24 when already filtered)
- toggle show_favorites + arrange: mean 1.78 ms

Root causes (confirmed in code, not only in these numbers):

1. `persist()` did a synchronous `write_slot_state` (create temp, write, fsync
   file, rename, fsync directory) on the UI thread for every menu setting
   change, SELECT sort, Systems toggle, and All/Favorites switch. Volume
   already used the background `persist_soon` path so held keys would not stall.
   On a class-10 SD card those two fsyncs are often tens to hundreds of ms,
   which matches "all menus feel a bit slow" and a hitch on the shelf switch.
2. `App::arrange()` ran on each of those toggles. For every cart on every
   system shelf it did a linear `all_carts` scan (`O(n²)` stem compares) and
   cloned a full `shown` list of every game, then cloned again per shelf.
   ~12.6 ms on this host is a one-frame-plus hitch; on the SP's slower CPU
   that can be well over a 60 Hz frame, stacked on the SD write.
3. `Library::get` allocated a new `String` key on every lookup. Cheap on this
   host (~120 µs for 6,189 gets) but paid again in `any_favorite`, sort, and
   arrange. Fixed by nesting `HashMap<Platform, HashMap<String, GameRecord>>`.

Labels and textures were already lazy (`CartFaces`, `shed_faces`); arrange
reuses faces by (platform, stem). library.txt was already written on a
background thread. No full library rescan or PNG reload on a shelf switch.

## After timings (2026-11-25, same host/image/profile as before)

- Library::get + any_favorite over 6,189 carts: mean 168–176 µs (same order; two-level map, no String alloc)
- write_slot_state still ~0.9–1.5 ms on this disk (unchanged; the fix is that the UI no longer waits for it)
- App::arrange (all games + Favorites shelf): mean 2.10 ms (was 12.61 ms, ~6×)
- toggle favorites_only + arrange: mean 1.67 ms (was 7.29 ms, ~4×)
- toggle show_favorites + arrange: mean 1.18 ms (was 1.78 ms, ~1.5×)

On the SP, the bigger win is `persist()` no longer blocking the frame on two SD fsyncs
(often tens to hundreds of ms). arrange() is also cheaper on that slower CPU.


