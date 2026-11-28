# Settings Reference

Press **HOME** on the shelf. Rows are taken from `crates/slot-ui/src/quick_menu.rs` on main, and descriptions from the
READMEs. Settings are saved in `Slot/Config` (`slot.state` and friends).

## Emulation

One page per system (Game Boy / GBC, Game Boy Advance, NES, SNES, PlayStation):

- **Shader:** the bundled shaders (ports of libretro's LCD3x, Dot, zfast_lcd_standard and simpletex_lcd). In game, HOME + A or SELECT + A cycles them.
- **Color Correction** (GB/GBC, GBA)
- **GB Palettes** (GB)
- **Overlay:** for GB, GBC, GBA, NES and SNES, KrutzOtrem's *Vanilla* and *CRT* sets or your own. For PS1, your own.
  In game, HOME + L1 / R1 steps through them.
- **Aspect:** Native, 4:3, Full or Integer
- **Dithering** (PlayStation)

## Gameplay

- **Saving:** **Save on Eject**, and **Auto Save** every 1, 5 or 10 minutes, or never
- **Turbo Buttons**, **Rewind** (hold L2), **Fast Forward** (hold R2; double-tap to lock) and **Fast Forward Sound**
- **Swap A/B:** swaps A and B in games and in Slot's menus. It's also on the in-game HOME menu under Buttons.

## Library

- **Sort By:** A–Z, Last Played or Most Played, for every shelf at once
- **Show:** all games, or Favorites only
- **Play Time** and **Title Card**
- **Systems:** shows or hides each shelf, with the Favorites shelf as the top row

## Online (needs Wi-Fi, except reading a saved achievement set)

- **Wi-Fi**
- **Cart Art:** GET CART ART, or delete a label
- **RetroAchievements:** log in or out, test the connection, CACHE ACHIEVEMENT SETS, VIEW ACHIEVEMENTS
- **Encore Mode** (in PR #27, on by default)
- **Update** / **Check for Updates** (Brick only)

## System

- **LEDs** (Brick): All LEDs, Top Bar, Buttons, Triggers, Sticks (Brick Pro), Color, Effect
- **Clock:** Auto Clock (NTP on Wi-Fi connect and every 6 h), Date & Time and time zone
- **Rumble:** games that rumble drive the motor
- **Font:** Slot's own font, GohuFont, or any `.ttf`/`.otf` in `Slot/Fonts`. Slot restarts to use it.
- **System Sounds:** Menu, Favorite Star, Cart In & Out, Cart Opening, Label On, Volume
- **Low Battery Shutdown:** 3%, 5% (the default), 10% or off
- **Reset Settings:** see [Developer Notes](Developer-Notes.md)
- **About:** Version, Build (Dev or release), Device, Firmware, SD Card free space, Label

## Files that change behaviour

| File | Effect |
| :--- | :--- |
| `Slot/DISABLE_AUTOBOOT` | Don't start Slot at boot (stock firmware) |
| `Slot/DISABLE_AUTO_SLEEP` | Turn off auto-sleep |
| `Slot/DEBUG_INPUT` | Log raw input to `Slot/input-trace.log` (SP bring-up) |

**Companion** (Pip) is switched off on main. Its missions are in PR #11.
