# Installation and Updating

> **Always back up the card first.** After installing or updating, **power the handheld fully off and cold-boot it.**
> A sleep and wake is not enough: Slot and its cores are loaded at startup.

## Card layout

Every device keeps the library in one `Slot/` folder.

```
Slot/
  Games/<SYSTEM>/     ROMs, one folder per shelf (see Systems and Cores)
  BIOS/               BIOS files; Slot/BIOS/PPSSPP for PSP (in PR #31)
  Labels/<SYSTEM>/    cart labels, <rom name>.png; Labels/GW/open/ for Game & Watch frames
  Saves/  States/     battery saves and save states, per system
  Config/             settings: library.txt (play time, favorites), retroachievements.txt,
                      Achievements/ (cached sets, offline unlocks), wpa_supplicant.conf (SP)
  Overlays/<SYSTEM>/  GB, GBC, GBA, NES, SNES (and PS1 on the SP)
  Fonts/              extra .ttf / .otf fonts
  Wallpapers/
  System/             Slot itself, its cores and licences. Never restore an old copy of this folder.
  Logs/               slot.log, cart-art.txt, multiplayer.log
```

The Brick card also has `.slot-boot` (a hidden folder), `Apps/` (Slot, Logo, Check, Stock) and `trimui/`.
The SP's TF2 also has `System/launch_frontend.sh`, which is the file BaseOS looks for.

## TrimUI Brick: stock firmware (recommended)

1. Back up the whole card to your computer.
2. Download `slot-stock-<version>.zip` from Releases and extract it.
3. Format the card FAT32, labelled `SLOT`.
4. Copy the **contents** of `SD-CARD` to the card root, hidden `.slot-boot` included.
5. Restore `Slot/Games`, `Saves`, `States`, `BIOS`, `Config`, `Labels`, `Wallpapers`, and any `Slot/BootLogo/original*`
   files. Don't restore old `Slot/System`, `trimui`, `Tools` or `Apps` folders.
6. Power on. If the stock menu appears, open **Apps → Slot** once to turn on autostart. **Apps → Check**
   writes `startup-report.txt`.

## TrimUI Brick: Slot OS (experimental)

Flash `slot-os-brick-<version>.img.xz` (or `slot-os-brickpro-…` on a Brick Pro) with Raspberry Pi Imager or
balenaEtcher. Flashing **erases the whole card**. If Windows then asks to format a drive, **always click Cancel**:
those drives are Slot OS's boot partitions. On its first start Slot OS grows the card to its full size. After that,
copy the library into `Slot/`. To go back to stock firmware, save `Slot/Saves` and `Slot/States`, then format with
SD Card Formatter.

## RG35XX SP

Two cards. Full steps are in `docs/rg35xxsp.md`.

- **TF1:** BaseOS 1.4.0, flashed once. Don't format or reflash it after that. Cancel any format prompt.
- **TF2:** FAT32 with an MBR partition table. Copy the **contents** of `dist-rg35xxsp/` (built on the Mac mini with
  `sh rg35xxsp/docker-build.sh`) to the root.
  `bash rg35xxsp/copy-from-brick.sh /Volumes/BRICK /Volumes/SLOT` brings a Brick library across. It never deletes
  anything, and it skips files already there.
- FAT32 can't store a file of 4 GiB or more. Split a disc image that big, or convert it to `.chd` or `.pbp`.

## Updating

- **Brick:** **HOME → Online → Update**, with Wi-Fi on and at least 20% battery or a charger. Updates are signed and
  checksummed. Only Slot's own files are replaced (`Slot/System`, the bundled `Apps`, `.slot-boot`, the bundled
  overlays and the README); games, saves, states, labels, BIOS, settings and wallpapers are left alone. If the new
  version doesn't start, the previous one is put back. This fork's Update reads this repository's releases
  (`crates/slot/src/update.rs`). On Slot OS it updates Slot OS too.
- **RG35XX SP:** no Update row. With the SP off, copy a new `Slot/System` (and `System/launch_frontend.sh`) onto TF2,
  then cold-boot.
- **Dev build:** every merge to `main` republishes the `dev` pre-release.

## Backups

- Everything personal is under `Slot/`: `Games`, `Saves`, `States`, `BIOS`, `Labels`, `Config` and `Wallpapers`.
- Slot OS and BaseOS cards have hidden folders and extra partitions. Show hidden items when copying.
- The **Logo** app saves the original boot logo to `Slot/BootLogo/original.bmp` the first time it runs. Keep a copy:
  it's the only way to get the original back.
