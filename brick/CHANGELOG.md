# Changelog — Slot. for TrimUI Brick

## Unreleased
- Every shelf's cart now turns at the same unhurried pace before it goes in, about 2.6 seconds: the turn eases in and out, finishes, and the cart rests a moment before it drops straight into the slot. A Game & Watch opens or lights up over the same time. GB, GBC, GBA, NES and SNES carts now turn too: once round, showing their edge and back, then down into the slot; a PlayStation disc spins as it turns over.
- Every cart is now a real 3D model, on the shelf and as it turns: the GB, GBC, GBA, NES, Famicom, SNES (US and PAL) carts and the PlayStation disc have their moulded lettering, label wells, ridges, screws and stickers, and their real sides, back and connector end. Each still takes the shell colour you chose for that game, clear and glitter shells included. The other systems' carts gained their backs and bottoms too: screws, stickers, moulded maker's text, grips and the connector's opening with its contacts; the Vectrex has its vents and handle, the UMD its read window.
- New shelves: ColecoVision (`Games/COLECO`, gearcoleco), Atari 5200 (`Games/A5200`, a5200), Atari 800 (`Games/A800`, atari800), Vectrex (`Games/VECTREX`, vecx), 32X (`Games/32X`, PicoDrive), Nintendo DS (`Games/NDS`, melonDS), Nintendo 64 (`Games/N64`, mupen64plus-next) and PSP (`Games/PSP`, PPSSPP; `.iso`, `.cso`, `.chd`, `.pbp`). Library → Systems shows or hides each one, and a label picture in `Labels/<that folder>` sits on the cart.
- The Genesis cart has real depth: rounded edges, raised side bulges with ridged grips, and the label sunk in its well; it tips toward you as it turns. The N64 cart is the real rectangular Game Pak with straight sides and a ridged, rounded top.
- The Vectrex shelf shows the console itself, switched off, with the game's art dim on its screen. Picked, it turns round, its screen lights up, and it goes in lit.
- Each new shelf has its own cart: the ColecoVision, 5200 and 32X carts, a 5¼" floppy for the Atari 800, the DS card, the N64 cart and a UMD for PSP. Each turns until its contacts point down and goes into the slot contacts first, as the earlier new shelves do. A UMD instead spins its disc inside the still shell, behind the clear window, then the UMD turns over end to end and round to face out again, settles, and only then slides straight down into the slot curved end first with its disc art upright.
- ColecoVision will not start without `Slot/BIOS/colecovision.rom`. The 5200 uses `Slot/BIOS/5200.rom` when it is there. The Atari 800 has AltirraOS built in and uses `ATARIXL.ROM` and `ATARIBAS.ROM` when present. The DS starts without `bios7.bin`, `bios9.bin` and `firmware.bin` and uses them when present. PSP needs the PPSSPP assets in `Slot/BIOS/PPSSPP`, which the card package includes.
- N64 and PSP have no stick on these handhelds, so the D-pad drives the stick; on PSP, hold L2 to get the D-pad back. N64's C buttons are on the face and shoulder buttons. The DS shows both screens but has no touch input yet. Vectrex has no colour overlays: the core draws white vectors, and Slot's overlays cover only GB, GBC, GBA, NES and SNES.
- N64 and PSP draw with GLES on the device. Fixed for them: a core that asks for its framebuffer from its own thread gets it, and a core that switches threads (libco) can no longer corrupt Slot's floating-point registers.
- A Favorites shelf collects starred games from every system in one row, each in its own cart or handheld. Y still stars or unstars a game. SELECT on that shelf is Added or Last Played. Library → Systems has Favorites as the top row, and you can hide every other system and keep only that shelf.
- New shelf: Game & Watch (`Games/GW`, `.mgw`), run by gw-libretro. Each game sits on the shelf as its own handheld: Donkey Kong, Donkey Kong II, Mario Bros. and Lifeboat as closed clamshells that open when picked, every other one (Wide Screen, Panorama, Table Top, and the VTech, Mattel, Tomytronic, Gakken, Bandai, Epoch and Elektronika games) with its screen dark until it lights up. The art is `Labels/GW/<game>.png` and `Labels/GW/open/<game>.png`, made by `brick/gw-art`. The picture keeps its shape on screen whatever size the game draws it. No save states (the core has none).
- New shelves: Genesis / Mega Drive (`Games/MD`), Master System (`Games/SMS`), Game Gear (`Games/GG`), Arcade (`Games/ARCADE`), TurboGrafx-16 (`Games/PCE`), Atari 2600 (`Games/A2600`), Intellivision (`Games/INTV`), Atari Lynx (`Games/LYNX`) and Atari 7800 (`Games/A7800`). Library → Systems shows or hides each one. A label picture in `Labels/<that folder>` sits on the cartridge.
- Each new shelf's cartridge looks like the real one: the US Genesis cart, the US Sega Card for the Master System, the HuCard, the Intellivision cart with its label on the sloped end, the 2600, 7800, Game Gear and Lynx carts, and a lit marquee for arcade games. The label takes the cart's light and sits in its recess. Tall carts such as the Sega Card and the HuCard are drawn no taller than the NES cart, so they stay on screen. `brick/build_sp_art.py` cuts each label to its cart's window (a cart scan gives just its label; a box cover is cut from the top, where its title is) and uses marquees for arcade games, from ES-DE first and then arcadeitalia.
- On the new shelves, a cart turns to face the slot before it goes in: it spins on the shelf until its connector points down, then drops into the slot as every other cart does. It shows that game's own label all the way round, and an Intellivision cart or a HuCard, which turn over end to end, still go in with the label upright. Ejecting one turns it back. The other systems go in as before.
- Genesis is a 6-button pad: B is B, A is C, Y is A, X is Y, L is X, R is Z, Select is Mode. Atari 2600 and 7800 use L and R as the difficulty switches. Intellivision: the D-pad is the disc, A and B and Y are the action buttons, and holding L or R shows the keypad (D-pad moves, A B X or Y presses, X repeats the last key, L2 is Clear, R2 is Enter).
- Intellivision will not start without `Slot/BIOS/exec.bin` and `Slot/BIOS/grom.bin`. Lynx will not start without `Slot/BIOS/lynxboot.img`. Neo Geo games need `Slot/BIOS/neogeo.zip`; other arcade games start without it and Slot says so if it is missing.
- The Anbernic RG35XX SP can run this same Slot. TF1 is BaseOS, TF2 holds a `Slot` folder with the same Games, BIOS, Saves, States and Config as the Brick, and the screen is the Brick's layout at the SP's own 640×480. How to flash the cards is in `docs/rg35xxsp.md`.
- System → Continue is gone. Saves, the resume point, and last-played stay on this handheld.
- L1 and R1 no longer change the picture while a game is running. On NES, Game Boy and Game Boy Color they do nothing. Emulation → Aspect, and the same row on each system's page, chooses Native, 4:3, Full or Integer. A system can follow that, or use its own. The pause menu has Aspect too, and the choice is remembered.
- Videos and Music are gone. HOME no longer lists them, and Slot no longer downloads or plays them.
- HOME no longer lists Companion, and Pip does not suggest a game or show a mission. The companion itself is still in Slot, switched off.
- The system menu shows the same battery, Wi-Fi and clock as the shelf. The battery stays the usual colour above 20%, turns yellow below that, and red at 10% or less. The charging mark shows only while a cable is plugged in.
- RetroAchievements on Game Boy Color can read the console's work RAM again, including the extra banks, so a condition that looks there is no longer skipped.
- HOME → Online → Update checks this Slot's own releases and installs a newer one. It checks the download before replacing Slot, keeps the previous one, and puts it back if the new one does not start. The screen shows the release's title and what changed. With no Wi-Fi it says so and leaves Slot as it is.
- HOME → Companion: Pip, a small pixel friend, suggests a game you have not played much and keeps a few missions (try a new game, revisit one, play three systems, a weekend of four games). Playing earns XP and levels. Toasts show a mission or a level-up. Y on a mission marks it done if Slot could not see it happen. Settings turn Pip, the toasts, and each mission kind on or off, and can reset Pip without touching your games. X on the shelf opens Pip too.
- HOME → Companion: Pip is a chunky walking cartridge, orange with a shaded shell, a big cream face, and cream sneakers. It squishes while it idles, waves a whole arm, sulks in the rain, stomps when a session was tiny, jumps when a streak or an achievement lands, yawns, tilts its head when it is curious, and throws both arms up on a level-up. The line under it changes with the time of day, the streak, and the game. Animations and Sounds in its settings still turn those off.
- HOME → Online → RetroAchievements works away from Wi-Fi once a game's set has been saved. CACHE ACHIEVEMENT SETS, over Wi-Fi, saves every game in the library, including its badges. Unlocks still happen on the device, show a toast, and stay in VIEW ACHIEVEMENTS. They are sent when Wi-Fi returns, and a torn save is restored from its spare copy. This is softcore only: a hardcore unlock is not kept to send later.
- HOME → Companion uses the same large type as the rest of Slot's menus. Missions read as what to do, such as playing one game for a few minutes or any game on one system. Companion keeps three of them when it can: one from games you have played, one about a system, and one about a game you have not played. The next level costs 100 XP, then 150, then 200. Companion's save keeps a spare copy and comes back if the file is torn. It also keeps each game's time, a streak, and favorites.
- Gameplay → Swap A/B: trade A and B's places, in games and in Slot's own menus. Also reachable without leaving the game: HOME's own menu now has a Buttons entry with the same switch.
- Fixed: on Slot OS, the mute switch did nothing, on the Brick and the Brick Pro. Thanks to @jyav-19 for testing Slot OS on a Brick Pro.
- Tidier downloads: each release now holds only `slot-stock-…zip` for stock firmware and `slot-os-brick-…img.xz` / `slot-os-brickpro-…img.xz` for Slot OS. Online → Update gets its files from a separate Updates release. Coming from v0.60 or older, update to this version by hand once: on stock firmware, copy what is inside the `SD-CARD` folder of `slot-stock-…zip` onto your card; on Slot OS, do the same with `slot-os-brick-…-update.zip` (or the Brick Pro's) from the Updates release. Your games, saves and settings stay.

## v0.60
**New since v0.50:**

### Features
- Added a check for updates: on boot, then every 30 minutes and whenever you slot a game out, while Wi-Fi is on. A small arrow shows in the footer when a newer Slot is out.
- Added a pause menu: tap HOME in a game to resume, save or load a state, reset or quit. By @kirby-letsgo
- Added support for zipped games (`.zip` and `.7z`) for every system except PS1. By @kirby-letsgo
- Favorites got a makeover: press Y on a cartridge and a star pops onto it with a nice sound.
- Added various system sounds, and you can turn each one on or off in System → System Sounds.
- Speaking of menus, the settings have been reorganized so related things sit together. They're still rough in places, so expect more changes.

### Fixes
- Fixed PS1 games slowing down startup. By @TT-Hipster1941
- Fixed Wi-Fi not showing as connected after waking from sleep.
- Fixed the GBA cart colour during the label animation.

**Slot OS for the Brick Pro** hasn't been fully tested on a Brick Pro, so if anything acts weird, please open an issue.

## v0.50
**New since v0.44:**
- PlayStation: PS1 games from `Slot/Games/PS1` (`.cue`/`.bin`, `.chd`, `.pbp`, `.iso`, `.m3u`) on a shelf of discs that spin into the slot, games of several discs included.
- Rumble: games that rumble now drive the Brick's motor.
- Favorites (Y) and sorting (SELECT), with play time and a title card for the selected game.
- Brick Pro support: its left stick works as the D-pad, and its stick lights have their own LED row.
- Fonts: HOME → System → Font, with GohuFont included, and room for your own in `Slot/Fonts`.
- A reworked HOME menu: Emulation, Gameplay, Library, Online and System.
- Faster startup, and Wi-Fi fixes.

## v0.44
**New since v0.43:**
- HOME → Update: update Slot over Wi-Fi. It shows what's new (L1 / R1 scroll long notes), asks before installing, and leaves your games, saves, states, labels and settings alone. Updates are signed and checked before anything is replaced; if the new version doesn't start, the previous one is put back, and an update cut short by the Brick switching off is cleared away. Needs 20% battery or a charger.
- HOME → System: LEDs, Date & Time and About are now under System, leaving the menu shorter.
- System → Auto Clock: with Wi-Fi on, the clock now sets itself from an internet time server, on connecting and every six hours. On by default; the time zone is still set under Date & Time.

## v0.43
**New since v0.42:**
- Overlays for every system come with Slot: KrutzOtrem's *Vanilla* set for GB, GBC and GBA and *CRT* set for NES and SNES, made for the Brick. Thanks to @KrutzOtrem!
- Overlays with a grid or tint over the game (such as KrutzOtrem's LCD and DMG grids) now work: the game fills the part of the overlay that isn't fully opaque, at the size the overlay was drawn for.
- An overlay name too long for its row is shortened, and scrolls slowly while the row is selected.
- Gameplay → Auto Save: save the game every 1, 5 or 10 minutes, or never. It was every minute.
- Gameplay → Low Battery Shutdown: save and shut down at 3%, 5% or 10% battery, or never. It was 5%.

## v0.42
**New since v0.41:**
- Screen settings are sorted by system: Game Boy / GBC, Game Boy Advance, NES and SNES each have their own page. NES and SNES shaders can now be set there too.
- Colour correction is set separately for the Game Boys and the GBA. A setting saved before keeps working for both.
- Overlays: put your own 1024×768 PNG overlays in `Slot/Overlays/<system>` and pick one on the system's Screen page. The game fits itself into the overlay's window. In game, HOME + L1 / R1 switches between them.

## v0.41
**New since v0.40:**
- Choosing a label now ends with it peeling onto the cart like a sticker, on every cart. On GBA carts the front closes for it, then opens again.
- Low-battery warning on the F1/F2 lights: steady red at 15% or below, blinking at 10% or below, even with the lights turned off. Thanks to @wargenng!
- Fixed: the About screen showed the wrong version.

## v0.40
**New since v0.23:**
- Wifi status icon on the shelf footer. Icons may change.
- Label Picker. Press Y when in the edit screen on a game cart. It has a label editor built in, so you can resize the label and make it fit the cart cutout
- Finally fixed Wi-Fi
- Customize cartridges: color, finish and shape for all shelves.
- Add SNES support: SNES and PAL cartridges
- Add NES support: NES and FAMICOM Cartridges
- Add RetroAchievements support, softcore only
- Automatic label fetcher in main menu, it fetches labels for all carts at once, but it is really barebones, so refer to the label picker for more control

**FOR DEVELOPERS**
- The Brick simulator - for testing in the browser before real hardware testing. Refer to the Development part of the Readme.

## v0.23
- Settings > About credits the port to LordNordyx.
- Short app names in the stock Apps list: Slot, Logo, Check and Stock, each with its own icon.
  Folder names are unchanged, so existing Apps/BootLogo/Image pictures keep working.
- Card README rewritten with the Brick's actual button layout (HOME = left button, SELECT = middle).
- Removed leftovers: the old Tools/tg5040/Slot.pak launcher and the RetroAchievements research note.

## v0.22.2
- Fixed "works once, then stock menu": the startup hook now waits until the real SD card is
  mounted instead of reading a stray Slot folder on internal storage.
- Check lists files hidden under the card mount on internal storage.

## v0.22.1
- The startup hook waits up to 30 s (was 10 s) for slow-mounting cards and logs each decision
  to Slot/Logs/hook.log.

## v0.22
- A read-only or damaged card no longer drops silently to the stock menu; Slot shows
  "Can't write to SD card".
- Check writes startup-report.txt at the top of the card.

## v0.21
- Fixed autostart failing after a firmware update or reinstall.
- New Check app: read-only startup report.

## v0.17 – v0.20
- SELECT + X switches GBA between 3:2 and 4:3 full screen, remembered per game.
- About screen describes the Brick port.

## v0.16
- Waking no longer needs two Power presses; Wi-Fi reconnects in the background.

## v0.15
- Merged upstream Slot 1.6.0: GB cartridge clocks kept in saves, read-only card alarm, faster
  library loading, shuffled wallpapers, crash logs.
- Boot Logo app (now Logo) replaces the separate install/restore logo apps.

## v0.14
- Lower sleep drain: radios off during suspend, real suspend-to-RAM where available.

## v0.13
- Auto-sleep after 5 min idle in menus or 15 min in game (Slot/DISABLE_AUTO_SLEEP turns it off).

## v0.12
- Wi-Fi works on stock firmware; minimum CPU clock in games fixes crackling audio.

Earlier versions: Brick screen, input, LEDs, power menu and autostart (see SOURCES.md).
