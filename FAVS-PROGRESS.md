# Favorites shelf — progress

Worktree: `~/sbp-favs` (branch `favs-shelf` from `1c4f42c`).
Do not touch `~/sbp-pr25`, the EVO, any card, or E:.
Do not push or open a PR until Jeff approves the look.

## Design

### How to favorite / unfavorite
Y on the shelf already stars or unstars the selected game (gold star on the cart, pop sound). That stays the control, and it works on the Favorites shelf too: unstarring a game removes it from that shelf.

### Where it is saved
`Config/library.txt` on the card (already): one line a game, `PLATFORM<tab>STEM<tab>secs<tab>last_played<tab>favorite`. This lives in Config, so it survives a reinstall that leaves Config alone.

Recently-added time is stored as extra lines the old parser skips: `FAVADD<tab>PLATFORM<tab>STEM<tab>unix_seconds`. Older builds keep reading the five-field lines and ignore FAVADD, so play time and stars are not lost if Jeff boots an older build.

Show/hide of the Favorites shelf and its sort are in `Config/slot.state`: `show_favorites=1` (default on) and `favorites_sort=added|recent`.

### How the shelf appears
- A combined Favorites shelf is the first shelf, when it is shown and at least one game is starred.
- It holds favorites from every system, including systems hidden in Library → Systems, each in its own cart or handheld shell with its own label.
- Launch uses the selected cart's platform and core, not a shelf-wide platform.
- HOME → Library → Systems has **Favorites** as the top row (On/Off), then the systems in shelf order.
- You can hide every system and keep only Favorites, if there are starred games. You can always hide Favorites; if every system was also hidden, the usual fallback shows all systems again.
- Mixed shells share one pitch (the widest cart on the shelf, usually 360 px) so they do not gap or overlap. Feet still rest on the same line; shells stay at most 331 px tall as in PR #25.
- Boot still lands on the first system shelf (not Favorites), so existing library-sort tests stay valid. L1 from the first system shelf goes to Favorites when it is shown.

### Sorting
- On the Favorites shelf, SELECT cycles **ADDED** (default, newest star first) and **LAST PLAYED**.
- On every other shelf, SELECT is unchanged (A–Z, last played, most played).
- The existing Library → Show: Favorites filter (per-system shelves) is unchanged.
- Games starred before FAVADD existed (favorited_at = 0) sort after dated stars in ADDED order, then by last played among themselves.

## Status

Implementation is complete and committed locally on `favs-shelf` (not pushed).

### Tests (all passed, Docker image `slot-brick-sim`, `CARGO_HOME` under `~/sbp-favs/.docker-cache`, target `target-favs`)
- `cargo test -p slot-store --offline`: 22 lib tests plus archive/durability, including FAVADD round-trip and arrange_favorites sort.
- `cargo test -p slot --no-default-features --features sim --lib --test library`: 202 lib tests + 14 library tests, including mixed-system Favorites + SELECT sort, Y removes from Favorites, hide Favorites in Systems and persist, hide all systems when Favorites remains, launch/star uses the cart's platform.
- `cargo test -p slot-ui --offline --lib`: 29 tests, plus `layout_tests::the_systems_list_scrolls_so_rows_stay_clear_of_the_legend`.

### Previews
Regenerated after the Systems-menu scroll fix (2026-11-25, Pacific).

On the Mac mini: `~/sbp-favs/preview/favorites/brick` and `.../sp` (9 PNGs each, 1024×768 and 640×480).

On the box: `/workspace/cart-renders/favorites-preview/{brick,sp}/`
- `shelf.png`, `next1.png`–`next5.png`: mixed Favorites row (Intellivision Astrosmash, US Genesis Gunstar Heroes, SNES Super Metroid, arcade Metal Slug, GBA Metroid Fusion, Game & Watch Donkey Kong II), each in its own shell with a gold star.
- `sort-added.png` / `sort-last-played.png`: SELECT legend reads ADDED or LAST PLAYED.
- `menu-systems.png`: Favorites is the top Systems row, On, readable pitch, legend clear. The list scrolls at the standard quick-menu pitch so Game & Watch no longer runs into the footer.

### Preview notes
- GBA and SNES games in the sim card use Slot title-card labels because fitted label PNGs for those systems were not on the Mac mini (dist Labels were empty). New-system labels came from `~/Projects/slot-cart-renders/fitted` and Game & Watch art was read-only from `~/sbp-pr25/brick/labels-local/GW`. On the device, real cart labels will show when they are already on the card.
- SP previews still log “running on a TrimUI Brick”; only the panel size is overridden (`SLOT_SIM_PANEL`), not the `rg35xxsp` feature. Same method as earlier PR #25 previews.

### CI (PR #26)
Pushed as `cursor/favorites-shelf`, stacked on PR #25. First CI: unit passed; changelog missed an Unreleased line; scenarios failed on missing Favorites goldens, the Systems menu (now scrolling), and a few goldens that assumed the old cramped menu or a single shelf.

Fixes in the follow-up commit:
- Changelog line under Unreleased.
- Systems list still scrolls when rows would overlap; short pages such as System keep every row on screen.
- Slot name is rebuilt when arrange clears its face but the words stay the same (Y to star).
- `library.sim` hides GBA (under Favorites) and R1-skips it; goldens updated for the Systems menu, the Favorites scenario, and a slot that now says A–Z when two shelves are in reach.
- `favorites.card.sh` no longer copies machine-local labels, so CI and a local `sim.sh test` match. The preview script still adds those labels when they are on the Mac mini.

### Not done
- No push, no PR, no merge. Wait for Jeff to approve the look.
- Did not touch `~/sbp-pr25`, the EVO, any card, or E:.
