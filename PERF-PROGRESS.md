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

