# Developer Notes

## "Reset to first use"

**Not found.** No "reset to first use", first-run or factory-reset feature exists on main or on any open branch
(searched: `first use`, `first_run`, `factory`, `onboard`, `welcome`). The nearest equivalents:

| Goal | How |
| :--- | :--- |
| Settings back to defaults | **HOME → System → Reset Settings**, then A again. Resets every setting, LEDs and font included. Games, saves, states, play time, favorites, the clock, brightness and volume stay. |
| A fresh simulator card | `FRESH=1 task sim` (or `FRESH=1 sh brick/sim.sh serve`) throws away `target-sim/live/card`. |
| Forget the RetroAchievements login | **LOG OUT**, or delete `Slot/Config/retroachievements.txt` |
| Library history (play time, favorites) | Remove `Slot/Config/library.txt` (back it up first) |
| See the first-start clock screen again | **Needs confirmation:** the README says the clock screen appears "at first start", but the trigger condition hasn't been traced in code. |

If a real "reset to first use" developer option is wanted, it would be a new feature. Nothing in the code today
matches that description.

## Logs worth knowing

`Slot/Logs/slot.log`, `cart-art.txt`, `multiplayer.log`, `sleep.log` (SP, PR #24), `startup-report.txt` (Apps → Check),
and `Slot/input-trace.log` (with `Slot/DEBUG_INPUT`).

## Repository map

| Path | What |
| :--- | :--- |
| `crates/slot` | The app: frontend, session, update, device glue |
| `crates/slot-store` | Platforms, library, `slot.state`, scanning, rankings (PR #29), arcade names (PR #28) |
| `crates/slot-ui` | Shelf, carts, menus, text, icons, 3D cart models (PR #32) |
| `crates/slot-gfx`, `slot-retro`, `slot-input`, `slot-power`, `slot-sfxcut` | Drawing, libretro host, input, power, sound effects |
| `crates/slot-cheevos` | rcheevos, libchdr and the C shim |
| `brick/` | The card (`brick/card`), build scripts, art tools (`build_sp_art.py`, `label_fit.py`, `gw-art`), simulator, changelog |
| `rg35xxsp/` | The SP's Docker build, card packaging and `copy-from-brick.sh` |
| `vendor/` | Prebuilt cores and their exact source tarballs |
| `licenses/` | Third-party licences copied onto the card |
| `assets/carts/<sys>/` | Cart shell PNGs and label windows |
