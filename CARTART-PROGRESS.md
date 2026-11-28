# Cart art engine — progress

Branch `cursor/cartart` from `cursor/android-phase3` (53a29549). Worktree `~/sbp-cartart`.
Draft PR only; merge on Jeff's yes. Jeff said (PR #39): on by default, 2× labels, remove the
old get-art path, log in to ScreenScraper on the device. Phase 3 below.
Design and settings: `docs/cart-art.md`.

## Done

- `crates/slot-cartart`: the engine (find the object, deskew, cut the label, fit the window,
  polish), per-system profiles for all 24 shelves, tier order and fallbacks. 12 tests.
- `crates/slot/src/cart_art_ss.rs`: ScreenScraper client (lookup, lazy per-tier downloads,
  1.2 s between calls, host allow-list, credential redaction, quota stop). 7 tests, including a
  fake server end to end. Wired into HOME → Cart Art ahead of the existing sources.
- Test tooling: `tools/cartart/` (games list, fetcher, contact sheets) and the
  `cartart_preview` / `cartart_bench` examples in slot-ui.

## Test run (2026-11-27)

74 games over 24 systems (3 each, 4 for NES and SNES; two well-known plus one less popular).
Raw pictures, fitted labels and carts in `~/Shared-Space/cartart-test/<SYS>/`, sheets in
`~/Shared-Space/cartart-test/sheets/` (also on the box at `/workspace/cartart-test/`).
Every game got a label: texture 49, photo 15, box 6, marquee 3, logo 1.

| System | Games | Tier used | Card path ms (Mac, median) | Label KB | Window |
|---|---|---|---|---|---|
| GBA | 3 | texture 3 | 5.9 | 289 | 266x138 |
| GB | 3 | texture 3 | 8.3 | 372 | 220x195 |
| GBC | 3 | texture 3 | 7.3 | 406 | 220x195 |
| NES | 4 | texture 4 (one Famicom) | 6.0 | 211 | 132x222 |
| SNES | 4 | texture 4 | 4.0 | 127 | 216x99 |
| PS1 | 3 | photo 3 | 10.6 | 346 | 300x300 |
| MD | 3 | texture 3 | 8.7 | 357 | 240x196 |
| SMS | 3 | box 3 | 10.0 | 234 | 181x198 |
| GG | 3 | texture 3 | 8.1 | 390 | 270x207 |
| ARCADE | 3 | marquee 3 | 5.9 | 103 | 330x90 |
| PCE | 3 | texture 3 | 9.3 | 178 | 181x237 |
| A2600 | 3 | texture 3 | 8.6 | 326 | 227x279 |
| INTV | 3 | texture 3 | 1.4 | 23 | 250x59 |
| LYNX | 3 | photo 3 | 12.5 | 326 | 286x263 |
| A7800 | 3 | texture 3 | 6.8 | 240 | 227x279 |
| GW | 3 | photo 3 (the handheld, standing) | 11.3 | 233 | 360x331 |
| COLECO | 3 | texture 3 | 8.1 | 210 | 254x268 |
| A5200 | 3 | texture 3 | 5.4 | 237 | 216x181 |
| A800 | 3 | texture 2, logo 1 | 3.1 | 125 | 261x86 |
| VECTREX | 3 | box 3 | 8.2 | 106 | 170x192 |
| 32X | 3 | photo 3 | 10.5 | 266 | 240x197 |
| N64 | 3 | texture 3 | 7.8 | 241 | 164x167 |
| NDS | 3 | photo 3 | 14.6 | 356 | 312x270 |
| PSP | 3 | texture 3 | 9.5 | 321 | 286x287 |

### Speed

`cartart_bench`, one thread on the Mac mini (M5 Pro), the card's exact path (decode only the
pictures needed, fit, encode at card size), median of 9 runs per game:

- per game: median 8.1 ms, p90 11.3 ms, max 14.6 ms (decode 1.9, fit 6.0, encode 0.6 median).
- PNG writing was 10 ms of that until it moved to fast deflate (23× faster, files 4.6% larger).

Estimates for the handhelds (not measured; no device was connected). Scalar integer code on an
in-order Cortex-A53 runs roughly 12–25× slower than one M5 P-core, and the JZ4770 roughly 50–90×:

| Device | CPU | Per game, median | Worst |
|---|---|---|---|
| Brick (tg5040) | A133P, Cortex-A53 @ 1.8 GHz | ~0.1–0.2 s | ~0.3 s |
| RG35XX SP | H700, Cortex-A53 @ 1.5 GHz | ~0.12–0.2 s | ~0.4 s |
| RG280V | JZ4770, MIPS32r2 @ 1.0 GHz, one core | ~0.4–0.75 s | ~1.3 s |

The network dominates either way: each game is one lookup plus usually one picture, with
1.2 s between calls and ScreenScraper's 128 KB/s developer cap (a 300 KB picture ≈ 2.3 s).
Peak memory is one ≤1000 px picture (4 MB RGBA) plus working copies, well under 30 MB.

### Cache size

Labels are kept at twice the window (at most 512 px across): mean 249 KB, max ~500 KB a game
(about 250 MB per 1,000 games). At the window's own size they would be mean 79 KB, max 162 KB.
Raw downloads are not kept on the card.

## Quality (judged from the sheets)

Good: GB, GBC, GBA, NES, SNES, MD, GG, PCE, A2600, A7800, COLECO, A5200, 32X, N64, NDS, PSP,
LYNX, PS1, GW (the handheld itself, standing on its outline), INTV (the end label, title only),
ARCADE (real marquees).

Weaker:
- SMS: ScreenScraper's support-texture is the end strip and support-2D is often the Mega
  Cartridge, so the label is the box cover cut to shape. Looks fine but is not the card label.
- VECTREX: the cart label is wide and the window is tall, so it uses the box (overlay art). OK,
  busy.
- ARCADE Metal Slug: the marquee ScreenScraper has is the generic NEO-GEO MVS one.
- A800 Star Raiders is a cartridge game; the floppy window gets its logo on a colour field.
- GB/GBA/SNES cart-photo fractions are uncalibrated (never needed here: every test game had a
  label scan).

## Status (2026-11-27 10:55 PST)

- Committed 71d47db6 on `cursor/cartart`, pushed. Draft PR #39 against `cursor/android-phase3`:
  https://github.com/RamblewoodLabs/slot-brick-plus/pull/39 (do not merge).
- CI on 6fdd0e30: changelog, unit and scenarios all pass (11:01 PST).
- All 25 PNG sheets copied to the box at `/workspace/cartart-test/`.

## Next

- Jeff: look at the sheets, then decide whether to turn ScreenScraper art on by default and
  whether labels should be 2× or 1× (cache size).
- Run on a real Brick / SP / RG280V to replace the estimates.
- Android: no cart UI yet (text labels), so not wired there; slot-cartart builds for it as is.

## Phase 3: on by default, ScreenScraper only, on-device login (2026-11-27)

- Done: `cart_art.rs` is ScreenScraper only (old art set, LaunchBox lists and
  `make-cart-*.py` removed); FIND A LABEL searches ScreenScraper (`jeuRecherche`). On by default
  (`cartart=off` turns it off); needs the built-in dev id plus the player's login.
- Done: HOME → Cart Art → SCREENSCRAPER row: username/password keyboard, TEST LOGIN AND SAVE
  (`ssuserInfos`), TEST LOGIN, SIGN OUT; GET CART ART without a login opens it.
- Done: `brick/ss-dev-env.sh` feeds `SLOT_SS_DEVID/DEVPASSWORD` to the Brick and SP docker builds
  from env or a gitignored `screenscraper.properties`; release.yml reads repo secrets of the
  same names (Jeff must add them).
- Done: `sh brick/sim.sh unit` 444 passed. Live test (`live_login_lookup_and_search`, ignored):
  dev account is refused as a user login (expected); dev-id lookups for GB/SNES/MD/NES fetched and
  fitted 2× labels; search works. Screens and live labels in `/workspace/cartart-test/`.
- Commits 55d4d533 and f2d78393 pushed. CI on f2d78393: changelog, unit, scenarios pass.
  Local: `sh brick/sim.sh test` 18 scenarios pass.
- Built with the dev id baked in (never printed): tg5040 card `dist-brick-cartart` (the arm64
  `slot-brick-build` image cannot run the x86 toolchain on the Mac, so built in `slot-sp-build`,
  the same Dockerfile on amd64), RG35XX SP/H700 card `dist-rg35xxsp-cartart`, Android
  `Slotted-debug.apk`. No old lists on either card; neither binary has the old hosts.
- Android: Settings already had username/password/Test login (ssuserInfos); added Sign out.
- RG280V (`cursor/rg280v-plan`): its own copy of crates/slot from an older base and no Wi-Fi;
  picks this up when rebased. Not changed.
- Open: Jeff to add repo secrets SLOT_SS_DEVID / SLOT_SS_DEVPASSWORD for release builds; a
  real user account to test the save-on-login path live (dev account is refused as a user).
