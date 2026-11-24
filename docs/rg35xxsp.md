# Slot on the Anbernic RG35XX SP

This is the same Slot as the TrimUI Brick build in this repository: GB, GBC, GBA, NES (FCEUmm), SNES (Snes9x) and PS1 (PCSX ReARMed), and RetroAchievements through `slot-cheevos` (rcheevos 12.5.0). Videos and Music stay off, as they are on main. Emulation → Aspect is the same as on the Brick: Native, 4:3, Full or Integer, and L1 and R1 do not change the picture.

The SP is a different machine. It is an Allwinner H700 with a 640×480 panel, a lid sensor, TF1 for the operating system and TF2 for games. The Brick is a tg5040 with a 1024×768 panel. The picture you see is the same layout. The Brick scales that 640×480 layout up to fill 1024×768 (exactly 8/5). On the SP it is drawn 1:1.

## Why this is not the pvaibhav 1.16 build

[pvaibhav/slot v1.16.0-pvaibhav.2](https://github.com/pvaibhav/slot/releases/tag/v1.16.0-pvaibhav.2) already runs on BaseOS 1.4.0 and already handles Wi-Fi, the screen, the buttons, the lid and TF1/TF2. Basing the product on that tree would be a shorter path to a binary that boots.

It would not be this Slot.

- Its default GBA core is armGBA. Save states written by the Brick's mGBA and gpSP do not load there. Battery saves do. The point of sharing a card is that states load too.
- Its card layout is upstream Slot's: `Games/`, `Saves/` and `System/slot` at the card root, and only GB, GBC and GBA in the stock release. This repository's library lives in `Slot/Games/{GB,GBC,GBA,NES,SNES,PS1}` and the matching Saves, States, BIOS and Config folders.
- RetroAchievements here go through `slot-cheevos` and rcheevos 12.5.0, the same client the Brick uses.

So the SP target stays in this repository. The hardware path is the one upstream Slot already uses on H700: Mali's fbdev window (width and height), evdev for the pad, the hall sensor for the lid, and the sysfs battery, backlight and power keys. The Brick's PowerVR null-window and SDL joystick are not used. The cargo feature is `rg35xxsp`. It turns on the Brick software (shelves, menus, achievements) and the H700 device path.

What that costs: this binary has not been booted on an SP in the environment that wrote it. Docker is not available there, so the cross-build has not been run either. The first boot on a real SP is still required. See [What is left](#what-is-left).

## Cards

You need two cards. TF1 is the operating system. TF2 is Slot and your games. A usable TF2 card is the one BaseOS boots. Leave TF2 out only when you want the handheld to show BaseOS's "add a frontend" screen.

Power the handheld off before you insert or remove a card.

### TF1 — BaseOS 1.4.0

| | |
| :--- | :--- |
| Image | [baseos-rg35xxsp-1.4.0.img.zip](https://github.com/pvaibhav/BaseOS/releases/download/v1.4.0/baseos-rg35xxsp-1.4.0.img.zip) |
| Release | [BaseOS v1.4.0](https://github.com/pvaibhav/BaseOS/releases/tag/v1.4.0) (14 October 2026), file `baseos-rg35xxsp-1.4.0.img.zip` |
| Zip size | 80,762,937 bytes (77.0 MiB) |
| Zip sha256 | `0d93edff938c1ca291bf0b0c5a281fa080a660e4d0fb7dc3ef788c2724b7626d` |
| Image inside | `baseos-rg35xxsp.img`, 1,498,415,104 bytes (1.40 GiB) |
| Card | 4 GB is the smallest BaseOS asks for. 8 GB or 16 GB is the comfortable size. Games do not go here, so a larger TF1 does not help. |
| Slot | The internal socket, marked **TF1**. |

This image is for the RG35XX SP only. A BaseOS card from another Anbernic model will not boot in it. The kernel and the panel drivers differ.

Flashing erases the whole TF1 card. Copy anything you need off it first.

1. On the Mac, check the zip before you unzip it:

   ```sh
   shasum -a 256 baseos-rg35xxsp-1.4.0.img.zip
   ```

   The line must end in `0d93edff938c1ca291bf0b0c5a281fa080a660e4d0fb7dc3ef788c2724b7626d`.

2. Unzip it. You get `baseos-rg35xxsp.img`.
3. Install [Raspberry Pi Imager](https://www.raspberrypi.com/software/).
4. **Choose device**: no filtering. **Choose OS**: Use custom, and pick `baseos-rg35xxsp.img`. **Choose storage**: the TF1 card. Check the size. The drive you pick is erased.
5. Next. No to customisation. Yes to erase. Wait until the verify step finishes.
6. If macOS or Windows then offers to format a partition, choose **Cancel** every time. Those partitions are BaseOS. Formatting one breaks the card.
7. Eject the card, put it in TF1, and turn the SP on with TF2 empty. The first start grows the card to its full size. When the screen says to add a frontend, hold Power to turn it off. Do not format TF1 after this. Do not reflash it to "make room".

A later BaseOS update, when you want one, is that model's `.bosupd` copied to the TF2 root, then a restart. v1.4.0 is the version this port is written against. Do not take a newer BaseOS until someone has booted this Slot on it.

### TF2 — Slot, FAT32, MBR

| | |
| :--- | :--- |
| Filesystem | FAT32 |
| Partition table | MBR (one partition) |
| Label | `SLOT` is a good name. Anything readable is fine. |
| Size | Whatever the library needs. 32 GB holds a cartridge collection. PS1 discs want 128 GB or more. |
| File size | FAT32 cannot store a single file of 4 GiB or larger. A `.chd` or `.bin` over that limit needs to be split, or stored as a `.pbp` that fits. BaseOS can also mount exFAT, but this card is FAT32 so it matches the Brick card and every frontend BaseOS documents. |
| Slot | The external socket, marked **TF2**. |

On the Mac, find the disk number with `diskutil list` (for example `disk4`, not `disk4s1`). Unmount it, then erase it. The command below destroys that disk.

```sh
diskutil unmountDisk /dev/diskN
diskutil eraseDisk FAT32 SLOT MBRFormat /dev/diskN
```

`MBRFormat` is the partition table. `FAT32` is the filesystem. macOS will format a card larger than 32 GB this way. Windows' own formatter will not; use the Mac, or [SD Card Formatter](https://www.sdcard.org/downloads/formatter/) set to overwrite format, then confirm the result is still MBR.

Build the card files on the Mac mini ([below](#build-on-the-mac-mini)), then copy **the contents** of `dist-rg35xxsp/` onto the TF2 root. The root must look like this:

```
README.txt
System/launch_frontend.sh
Slot/System/slot
Slot/System/launch.sh
Slot/System/mgba_libretro.so
Slot/System/gpsp_libretro.so
Slot/System/fceumm_libretro.so
Slot/System/snes9x_libretro.so
Slot/System/pcsx_rearmed_libretro.so
Slot/Games/GB/          .gb, and .zip or .7z with one ROM inside
Slot/Games/GBC/
Slot/Games/GBA/
Slot/Games/NES/
Slot/Games/SNES/
Slot/Games/PS1/         .cue/.bin, .chd, .pbp, .iso, .m3u
Slot/BIOS/              gba_bios.bin when you want the real GBA boot
Slot/Saves/...
Slot/States/...
Slot/Labels/...
Slot/Config/
```

BaseOS looks at the TF2 root for `System/launch_frontend.sh` and runs it. That script sets `SLOT_ROOT` to `Slot/` and starts Slot. Games are not placed at the TF2 root. That is deliberate: a Brick card's `Slot` folder drops onto this card without being rearranged.

`Slot/System/` on TF2 is the SP build (this binary, these cores, the BaseOS launch script). Do not replace it with the Brick's `Slot/System`. The copy script below will not do that.

Overlays are not on the card. The Brick's frames are 1024×768. An overlay for the SP is a 640×480 PNG in `Slot/Overlays/GB`, `GBC`, `GBA`, `NES`, `SNES` or `PS1`. The game is fitted into the part of the picture that is not fully opaque.

Eject TF2, insert it, and turn the SP on. Slot should open. If BaseOS still says to add a frontend, `System/launch_frontend.sh` is not at the TF2 root (it is easy to copy the `dist-rg35xxsp` folder itself instead of what is inside it). The log, once Slot has started, is `Slot/Logs/slot.log`. BaseOS's own log is `baseos-boot.log` at the TF2 root.

### Wi-Fi, the lid, and power

Wi-Fi is HOME → Online → Wi-Fi, the same menu as the Brick. The helper is `Slot/System/brick_wifi.sh`. On BaseOS it uses `wpa_supplicant` and `udhcpc`. Saved networks go to `Slot/Config/wpa_supplicant.conf`.

Closing the lid asks the kernel for suspend-to-RAM, the same `Slot/System/sleep.sh` the Brick uses. Opening the lid wakes it. Holding Power opens the menu: shut down, or restart. There is no stock launcher to exit to, and there is no Brick light-bar menu.

HOME → Online has Wi-Fi, cart art and RetroAchievements. It does not have Update. That row on the Brick downloads the Brick binary, which will not run this hardware. To replace Slot, copy a new `Slot/System/slot` (and the `.so` cores, if those changed) onto TF2 with the SP off.

## Copy a Brick library onto TF2

With both cards in the Mac:

```sh
bash rg35xxsp/copy-from-brick.sh /Volumes/BRICK /Volumes/SLOT
```

The first path is the Brick card (the volume that contains `Slot/`, or the `Slot` folder itself). The second is the TF2 volume. Files land in `TF2/Slot/`.

It copies `Games`, `BIOS`, `Saves`, `States`, `Labels`, `Fonts`, `Wallpapers` and `Config`. A file that is already on TF2 is skipped. Nothing is deleted. File times are kept, so the copy does not look newer than the file it came from. `Config/sync.txt` and `Config/continue.txt` are left behind.

It does not copy:

- `System/` (the Brick binary and the SP binary are different)
- `Overlays/` (1024×768)
- `Logs/`
- `Apps/`, `trimui/`, `.slot-boot/`
- `Config/brick_leds.ini`, `Config/brick_video_mode*.ini`, and any file whose name starts with `brick_`

`bash rg35xxsp/copy-from-brick.sh --self-test` checks those rules in a temporary directory.

## Build on the Mac mini

From a checkout of this branch, with Docker Desktop running:

```sh
sh rg35xxsp/docker-build.sh
```

The result is `dist-rg35xxsp/`. Copy its contents to the TF2 root, as above.

The script refuses to start when `docker` is missing or Docker is not running, and it says which of those it is. It does not install Docker and it does not need Rust on the Mac. The container is `brick/Dockerfile`: Linaro gcc 7.5, glibc 2.25, Rust 1.96, target `aarch64-unknown-linux-gnu`, profile `device`, feature `rg35xxsp`.

```sh
sh rg35xxsp/docker-build.sh --cores
```

rebuilds the five cores from the commits in `brick/build-cores.sh` and then packs them. You do not need that for a card that should match the Brick. The default card uses `vendor/*.so`, which are those builds already.

| Core | Role | Commit |
| :--- | :--- | :--- |
| mGBA | GB, GBC, GBA | `e31759b24e7a4e3899285ff720d7b573ac328ae7` |
| gpSP | GBA (the fast core; pick it per cart with START) | `7b2dd37394e2822110cacef321e9bb291a305b77` |
| FCEUmm | NES | `7a542dab1e87679921962a9f056186eca425c0c2` |
| Snes9x | SNES | `fae2fea08f74180759ef540ee94259213f503480` |
| PCSX ReARMed | PS1 | `c8816799b50388e61cfe237fe2cdbb7d8175f20a` |

rcheevos is 12.5.0, compiled into `slot` from `crates/slot-cheevos`. A save state from one device loads on the other when both are running these cores. A state made by a different core build, including armGBA, will not.

## What is left

- Boot it on an RG35XX SP. Confirm the picture fills the 640×480 panel, that every button and the D-pad do what they do on the Brick, that the lid sleeps and wakes, and that a game produces sound. If a button does nothing, put an empty file at `Slot/DEBUG_INPUT`, restart, and read `Slot/input-trace.log`. The pad map accepts the hat, `BTN_DPAD_*` (`0x220`–`0x223`) and `KEY_UP`/`DOWN`/`LEFT`/`RIGHT`, because H700 kernels have used each of those.
- Run `sh rg35xxsp/docker-build.sh` on the Mac mini once Docker Desktop is up. That command was not run here: this environment has no Docker.
- Join Wi-Fi against BaseOS's `wpa_supplicant`.
- A 640×480 overlay, if you want frames. None are shipped.
- Two-player link. The Brick's `slotlink.sh` is not on this card.
