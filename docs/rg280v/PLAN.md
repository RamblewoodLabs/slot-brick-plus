# Slot on the Anbernic RG280V — Plan

This is a plan only. It does not flash a card, install firmware, or download BIOS, keys, or firmware. Nothing is written to a microSD card until Jeff says yes, names the card, and we have checked its label.

The user-facing name on the RG280V is **Slotted**, the same name as the Android APK. The TrimUI Brick and RG35XX SP builds stay **Slot**. In this repository the port lives under `rg280v/`, and the on-device app, OPK, and Shared-Space artifacts are named Slotted.

The RG280V will work once a properly flashed OS card is in **TF1** (the slot labelled TF1 / INT). A black screen with no card in that slot is normal. The machine has no internal eMMC. Plug-in “signs of life” are consistent with charging, not with a successful boot.

**Recommended OS base:** [Adam image v2.1](https://github.com/eduardofilo/RG350_adam_image/releases/tag/v2.1) (OpenDingux Beta, kernel 5.15). Slotted would replace SimpleMenu as the frontend, with GMenu2X kept as a fallback.

**Systems we should ship:** everything through 16-bit plus most PS1, with a few caveats on SNES, arcade, 32X and the heavier PS1 titles. N64, DS and PSP should stay off the default shelves.

**Cards to buy:** a name-brand 16 GB or 32 GB card for TF1, and a name-brand 64 GB (or 128 GB if the PS1 library is large) card for TF2. Official maximum is 256 GB. Avoid no-name cards and any card that shipped with the handheld.

---

## 1. Hardware

The RG280V is a vertical (Game Boy-shaped) handheld released in November 2020. It is the same Ingenic JZ4770 class as the RG350, RG350P, RG350M, RG280M and RG300X, so emulator performance matches those machines, not the TrimUI Brick (tg5040) or the RG35XX SP (Allwinner H700).

| | |
| :--- | :--- |
| SoC | Ingenic JZ4770 (65 nm), 2011. One application **XBurst** CPU, **MIPS32 Release 2**, little-endian, 8-stage pipeline, Ingenic MXU SIMD, IEEE-754 FPU, 256 KB L2, clocked at **1.0 GHz**. A second **~500 MHz MIPS VPU** does 1080p video decode (H.264, VC-1, VP8). It is not a general-purpose second Linux core. Anbernic’s “4770 dual 1.0 GHz” wording counts that VPU; Linux userspace sees one application CPU. The true dual-core SMP chip in this family is the later JZ4780. |
| RAM | 512 MB DDR2 |
| Screen | 2.8-inch IPS, **320×480**, 2:3 portrait, ~206 ppi, 60 Hz, glass cover, no touch. This is not 640×480 (that is the RG35XX SP, and the RG350M). Slot’s existing logical layout is 640×480, scaled 8/5 to the Brick’s 1024×768 and drawn 1:1 on the SP. On this panel a 640×480 layout only fits as a **320×240** integer half, letterboxed. |
| GPU | Vivante **GC860**, about 315–575 MHz depending on the source, OpenGL ES 2.0. OpenDingux Beta uses the open-source **Etnaviv** Mesa driver, not the Vivante blob. The SoC also has an **IPU** for hardware scale/rotate of frames, which emulators rely on; SDL 1.2 on the current kernel cannot reach the IPU through the old fbdev API. |
| Storage | **No internal eMMC.** Two microSD slots on the left edge, officially up to 256 GB. Anbernic’s “built-in 16G” is a card in the box for TF1, not soldered storage. |
| Battery | 2100 mAh Li-polymer, USB-C charge, about 5–7 hours quoted. |
| Wireless | **None.** No Wi-Fi, no Bluetooth, no HDMI. USB-C is charging and, once booted, MTP or mass storage. RetroAchievements, in-app updates, and on-device art scraping need a later USB-networking path or a computer-side tool. |
| Other | One rear mono speaker, 3.5 mm jack, rumble motor, Power and Reset on the right, volume +/− on the left. |

Sources: [Anbernic product page](https://anbernic.com/collections/handheld-game-console/products/anbernic-rg280v); [Handhelds Arena](https://handheldsarena.com/devices/anbernic/rg-280v/) (internal storage: none; screen 320×480, 2:3 portrait); [retroshelf](https://www.retroshelf.org/devices/anbernic-rg-280v); [Wikipedia, Ingenic Semiconductor, JZ4770 row](https://en.wikipedia.org/wiki/Ingenic_Semiconductor) (MIPS32 r2, GC860, secondary 500 MHz VPU); [HandheldWiki, JZ4770](https://handheldwiki.com/chip/ingenic-jz4770/) (single-core XBurst @ 1.0 GHz, GLES 2.0 / Etnaviv); [Retro Dodo review](https://retrododo.com/rg280v/).

### Controls

No analog sticks. Layout:

- **D-pad** lower left, **A B X Y** lower right (soft-touch, less clicky than some Anbernics)
- **Start** and **Select** on the face
- **L1 / L2 / R1 / R2** along the top edge, side by side; L2 and R2 sit slightly proud, L1/R1 wrap a little around the back
- **Volume +/−**, **Power**, **Reset**

Slot already reads evdev (`crates/slot/src/input/evdev.rs`) and accepts hats, `BTN_DPAD_*`, and `KEY_UP`/`DOWN`/`LEFT`/`RIGHT`, plus L1/R1/L2/R2, Start/Select, volume and power. The RG280V map should be discovered on first boot with a `DEBUG_INPUT` trace, the same way the SP port does, because OpenDingux key codes may not match the H700 ones.

There is no stick to map for N64 C-buttons or a PSP analog nub. For those systems (which we are not shipping by default) Slot already has a “d-pad as stick” path.

### The two TF slots, and which one boots

| Slot | Label on the shell | Role |
| :--- | :--- | :--- |
| **TF1** | TF1 / INT | Boot card. U-Boot, kernel, rootfs, and (in this plan) Slot live here. **The device will not boot, and the screen will not show an OS, without a valid image in this slot.** |
| **TF2** | TF2 / EXT | External storage. Games, BIOS Jeff supplies, saves, labels. Not used to boot. |

If the cards are swapped, the handheld stays black. This is a common trap. Sources: [Retro Game Corps, RG350 & RG280 starter guide](https://retrogamecorps.com/2021/08/20/rg350-rg280-series-starter-guide/); [Joey’s RG280V/RG280M setup guide](https://www.joeysretrohandhelds.com/guides/anbernic-rg280v-rg280m-setup-guide/); [r/RG280V, November 2025](https://www.reddit.com/r/RG280V/comments/1p731sd/if_your_rg280v_has_a_faint_green_light_but_will/).

---

## 2. No card in TF1: black screen is normal

The RG280V has no on-chip storage and no boot ROM UI. The SoC tries to boot from the card in TF1. With that slot empty there is no kernel, no framebuffer setup, and nothing to draw. Holding Power with no TF1 card and seeing a black screen is expected, not a sign the unit is dead.

**Charging and LEDs.** USB-C charging works with no card inserted. Community reports for this model:

- A **solid red LED** while plugged in means it is charging. That is ordinary “signs of life.” Leaving it plugged in overnight is fine, and a good idea if the unit has sat unused.
- The red light typically **goes out when the battery is full**; some units are also described as going green when full. Treat “red while charging” as the reliable signal.
- A **quickly blinking red LED** while charging has been reported as a possible battery or charge-path problem, not as normal charging. See [r/RG350, “charging red light vs blinking red light”](https://www.reddit.com/r/RG350/comments/rexlvv/rg280v_charging_red_light_vs_blinking_red_light/).
- A **faint green light** near the USB / headphone area can be on whenever the battery is connected, even if the screen stays black. It is not a successful boot. See [r/RG280V, “won’t power on, black screen, battery works”](https://www.reddit.com/r/RG280V/comments/11bigzn/rg280v_wont_power_on_black_screen_battery_works/) and [r/SBCGaming, “RG280V not turning on”](https://www.reddit.com/r/SBCGaming/comments/zvifk2/rg280v_not_turning_on/).

Use a plain **USB-A-to-C** cable and a **5 V / 1–2 A** charger, not a USB-C PD / Quick Charge brick. These devices are fussy about chargers. After a long sit, give it a few hours on the charger before judging the battery.

Once a correctly flashed TF1 card is in (RG280V kernel selected — see below), hold Power for a few seconds. Adam’s first boot can take a while and can sit on a black screen for a short time. That is different from “no card at all.”

If it still never leaves a black screen after a known-good Adam card is in TF1, the next checks are: wrong slot, wrong kernel for this model, a bad or mis-flashed card, or, later, a battery reseat. We should not open the unit unless Jeff asks.

---

## 3. Operating system

### Options

| Option | What it is | Verdict |
| :--- | :--- | :--- |
| **Stock Anbernic OpenDingux** | Old kernel (3.12-era), stock UI. | Do not use as a Slot base. |
| **Adam image v2.1** (July 2022) | Ready-to-flash TF1 image for GCW-Zero, PocketGo 2, RG350 / RG280 / RG300X. OpenDingux Beta **2022-02-13**, Linux **5.15**, RetroArch 1.10.3, SimpleMenu, standalone OPKs. [Releases](https://github.com/eduardofilo/RG350_adam_image/releases/tag/v2.1). Rolled the kernel back from 5.17 to 5.15 for stability. Supports exFAT on TF2 since v2.0. | **Recommended user-facing base.** Kernel selection for the RG280V is documented. Retro Game Corps and Joey’s guides still point here. |
| **Upstream OpenDingux Beta 2022-09-22** | Latest tagged release of [OpenDingux/buildroot](https://github.com/OpenDingux/buildroot/releases/tag/od-2022.09.22): `gcw0-update-2022-09-22.opk` and the matching **gcw0 toolchain**. Newer than the OD Beta inside Adam v2.1. No newer official tag through 2026. | Use this **toolchain** to compile Slot. Optionally update a booting Adam card with the official OPK later, after Slot has been proven on v2.1. |
| Community “updated Adam” builds | Informal 2024-era rebuilds appear on forums. | Not a first choice. If one is still maintained when we flash, we can re-evaluate. |
| RetroFW, ArkOS, Knulli, Android | Different chips or a different OS class. | Not for the JZ4770 RG280V. |

### Best base for a custom frontend

**Adam v2.1 on TF1**, with Slotted as the default frontend and GMenu2X left installed as a fallback. We do not invent a new distribution in phase 1. We do not reflash TF1 until Jeff says yes.

After flashing Adam, the boot files for this exact model must be selected. On Windows that is `select_kernel.bat`. On a Mac the matching RG280V kernel files are copied to the boot partition root (Adam’s wiki and the Retro Game Corps Mac notes). A kernel for an RG350 or RG280M will not boot this panel. If macOS or Windows offers to format a partition after the flash, choose **Cancel** every time.

### How frontends and apps launch

- The default OpenDingux launcher is **GMenu2X**. Adam boots **SimpleMenu** instead, with GMenu2X still available.
- The file that chooses the frontend is **`/usr/local/sbin/frontend_start`**. Changing that script is the supported way to start Slotted on boot. Source: [OpenDingux wiki, Frontend](https://github-wiki-see.page/m/retrogamehandheld/OpenDingux/wiki/Frontend).
- Apps and emulators are **`.opk`** files (a squashfs plus a desktop entry). They live in `/media/data/apps` on TF1, or in an `apps` folder on TF2. Installing or updating Slot can be “replace this OPK.”
- Plan: package Slotted as an OPK, point `frontend_start` at it, and keep a hold-button or a small wrapper that falls back to GMenu2X if Slotted exits or is missing. That is the same idea as BaseOS’s `System/launch_frontend.sh` on the SP, just in the OpenDingux place.

### SDL, KMS, and GLES

| Layer | Status on OpenDingux Beta (2022) |
| :--- | :--- |
| Kernel | 5.15 (Adam v2.1) or later 5.x in the 2022-09-22 snapshot, with **KMS/DRM**, Etnaviv, huge pages, 8-bit paletted mode. |
| GLES 2.0 | Yes, via Mesa **Etnaviv** (`libEGL.so`, `libGLESv2.so`). |
| SDL 1.2 | Widely used by older OPKs. **Cannot use the IPU** through the drm-framebuffer emulation of the old fbdev API. [pcercuei, opendingux.net](https://pcercuei.gitlab.io/OpenDingux.html). |
| SDL 2 | Present in the gcw0 2022 toolchain, with a **DRM/KMS** backend. RetroArch on these devices can be built with SDL2; software-rendered SDL2 is not always faster than the patched SDL1. |

Slot’s (and Slotted’s) device path already talks **EGL + OpenGL ES 2** through `libEGL.so` / `libGLESv2.so` (`crates/slot-gfx/src/fbdev.rs`), and already has an ES shader path. It does not need winit or glutin on device. The catch: the current fbdev window is a Mali-style `{width, height}` struct (and a different PowerVR path on the Brick). **Etnaviv/Mesa on KMS almost certainly will not accept that window struct.** Phase 1 has to prove one of: GBM/KMS EGL, an SDL2 window, or whatever EGL surface OpenDingux actually exports. That is the main graphics risk, not “does the chip have GLES.”

Audio on device is already **ALSA** (`crates/slot/src/audio`), which OpenDingux provides. Input is already **evdev**. Those two should port cleanly once the key codes are known.

---

## 4. Toolchain and how hard it is to compile Slot

### OpenDingux toolchain

Use the official **gcw0** toolchain (JZ4770), not lepus or rs90 (those are weaker Ingenic chips):

- [opendingux-gcw0-toolchain.2022-09-22.tar.xz](https://github.com/OpenDingux/buildroot/releases/download/od-2022.09.22/opendingux-gcw0-toolchain.2022-09-22.tar.xz) from [OpenDingux Beta 2022-09-22](https://github.com/OpenDingux/buildroot/releases/tag/od-2022.09.22)
- Unpack under `/opt/gcw0-toolchain` and run `relocate-sdk.sh`
- Cross compiler: `mipsel-linux-gcc` (uClibc, **hard-float**, MIPS32r2)
- The toolchain is an x86-64 Linux sysroot. On the Mac mini, build in **Docker `linux/amd64`**, the same pattern as `rg35xxsp/docker-build.sh`

OpenDingux userspace is **uClibc (uClibc-ng), hard-float**, not glibc and not musl. Linking a musl or glibc binary will not run. The default `mipsel-unknown-linux-musl` and `mipsel-unknown-linux-uclibc` Rust targets are **soft-float**, which is the wrong ABI for this chip. Source: [MaulingMonkey/rust-opendingux-test notes](https://github.com/MaulingMonkey/rust-opendingux-test/blob/master/Notes.md); Rust’s own `mipsel-unknown-linux-uclibc` spec still sets `+mips32r2,+soft-float`.

### Rust

Slot is pinned to **Rust 1.96** (`rust-toolchain.toml`). The targets we care about are all **Tier 3**: the Rust project does not ship prebuilt `std` and does not test them automatically. They “may or may not work.” Sources: [rustc Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html) (`mipsel-unknown-linux-gnu`, `mipsel-unknown-linux-musl`, `mipsel-unknown-linux-uclibc` are all Tier 3; `std` is marked supported); [mipsel-unknown-linux-gnu target doc](https://doc.rust-lang.org/stable/rustc/platform-support/mipsel-unknown-linux-gnu.html).

Practical recipe:

1. A **custom target JSON** (`mipsel-gcw0-linux-uclibc`, or similar): `env: uclibc`, `features: +mips32r2` **without** soft-float, linker `/opt/gcw0-toolchain/usr/bin/mipsel-linux-gcc`. MaulingMonkey’s 2020 notes are the starting point; they must be regenerated against a current rustc, not copied blindly.
2. **Nightly** plus `rust-src` and **`-Z build-std`** (xargo is obsolete). The 1.96 pin may need a matching nightly for this target only, in Docker, without changing the ARM/host toolchains.
3. Feature-gate **Wi-Fi, rustls, ring, and in-app updates** off. This device has no Wi-Fi, and `ring` on 32-bit MIPS has a history of pain. RetroAchievements can wait for a USB-networking phase.
4. Device features stay on the existing path: `slot-gfx` EGL/GLES, ALSA, evdev. Do not enable `host` (winit/glutin/cpal/gilrs) for this target.

There is a prior art repo, [MaulingMonkey/rust-opendingux-test](https://github.com/MaulingMonkey/rust-opendingux-test), that ran GLES on an RG350M in 2020. Useful, but its nightly is `2020-06-15`. We should not pin Slot to that.

### How hard is a full Slot binary?

**Hard, but structured — harder than the SP port, easier than inventing a new emulator.** The SP port reused an ARM glibc toolchain and a known Mali fbdev window, and kept the 640×480 layout 1:1. This port adds a Tier 3 ISA, a uClibc hard-float custom target, a different EGL window, a 320×480 portrait panel, and 512 MB RAM.

| Piece | Expected difficulty |
| :--- | :--- |
| Docker + gcw0 toolchain + hello C OPK | Low |
| Custom Rust target + `build-std` + “clear the screen, read a button, beep” | Medium, and it will catch ABI mistakes early |
| Full Slot with Wi-Fi / rustls / ring left on | High; do not try this first |
| Full Slot with network crates gated off, letterboxed 320×240 UI | High-medium; this is the first real milestone |
| Native 320×480 portrait UI + scaled insert frames | Medium once the shell runs; it is layout work, not a new renderer |
| Embedding every libretro core | Per-core. Interpreters should be fine. Dynarecs need a MIPS backend. Skip N64/DS/PSP. |

A hello OPK in a few days of toolchain work is realistic. A playable Slot shelf on the device is a multi-week project, not a weekend one, and it is not done until it has been played on the real handheld.

---

## 5. Systems, mapped to Slot’s 24 shelves

Slot’s `Platform::ALL` in `crates/slot-store/src/platform.rs` is 24 systems. Performance below is for the JZ4770 class (RG350 / RG280), from [Retro Game Corps’ OpenDingux Beta OPK list](https://retrogamecorps.com/2020/12/05/opendingux-beta-firmware-for-rg350-and-rg280-devices/), [retrogamehandheld/OpenDingux](https://github.com/retrogamehandheld/OpenDingux), [Retro Dodo (RG350 and RG280V)](https://retrododo.com/rg280v/), [GBAtemp JZ4770 comparison](https://gbatemp.net/threads/anbernic-jz4770-retro-handhelds-comparison-rg280v-rg350p-rg350m.580864/), [retroshelf](https://www.retroshelf.org/devices/anbernic-rg-280v), and [RETROPULSE](https://theretropulse.com/handheld-database/rg-280v).

“Recommended core” is the usual OpenDingux standalone or RetroArch core, for phase 3 (launch an existing OPK). Phase 5 can replace that with a mipsel build of Slot’s own core when it compiles and runs well. We do not download BIOS. Where a BIOS is required or helpful, Jeff supplies his own file; the table only names the filename the emulator looks for.

### Ship — runs well

| Shelf | Slot core today | Recommended on this hardware | Notes |
| :--- | :--- | :--- | :--- |
| Game Boy | mGBA | **Gambatte** (DMS standalone) or mGBA | Full speed. |
| Game Boy Color | mGBA | **Gambatte** or mGBA | Full speed. |
| Game Boy Advance | mGBA / gpSP | **ReGBA** (pcercuei or jdgleaver) or **gpSP** | Full speed. Optional `gba_bios.bin` in the emulator’s BIOS dir, supplied by Jeff. |
| NES | FCEUmm | **FCEUX** (gameblabla) or FCEUmm | Full speed. No BIOS. |
| Genesis / Mega Drive | Genesis Plus GX | **PicoDrive** or Genesis Plus GX | Full speed. PicoDrive is the usual “fast” choice here. |
| Master System | Genesis Plus GX | PicoDrive, Genesis Plus GX, or **SMS Plus GX** | Full speed. |
| Game Gear | Genesis Plus GX | PicoDrive or Genesis Plus GX | Full speed. Portrait screen is a good fit. |
| TurboGrafx-16 / PC Engine | mednafen_pce_fast | **Temper** or mednafen_pce_fast | Generally full speed. |
| Atari 2600 | stella2014 | **stella2014** (RetroArch) | Very light. |
| Atari 7800 | prosystem | **prosystem** (RetroArch) | Light. |
| Atari 5200 | a5200 | **a5200** | Light. Optional `5200.rom`, supplied by Jeff. |
| Atari 800 | atari800 | **atari800** | Light. Optional `ATARIXL.ROM` / `ATARIBAS.ROM`, supplied by Jeff. |
| Atari Lynx | handy | **Handy** | Good. Optional `lynxboot.img`, supplied by Jeff. |
| ColecoVision | gearcoleco | **SMS Plus GX** or gearcoleco | Light. Optional `colecovision.rom`, supplied by Jeff. |
| Intellivision | freeintv | **freeintv** | Light. Needs `exec.bin` and `grom.bin`, supplied by Jeff. |
| Game & Watch | gw | **gw** libretro | Tiny. Already in Adam’s RetroArch set. |
| Vectrex | vecx | **VecxEmu** / vecx | Light. Overlay/bezel work can come later. |

### Ship with caveats — good or mixed

| Shelf | Slot core today | Recommended on this hardware | Notes |
| :--- | :--- | :--- | :--- |
| SNES | Snes9x | **PocketSNES** (`PocketSNES_new.opk`) or Snes9x | Most of the library is full speed. A few chip-heavy / Super FX titles may dip. Still worth shipping. |
| PlayStation | PCSX ReARMed | **PCSX4All** (jdgleaver preferred on OD Beta) or **pcsx_rearmed** in RetroArch | Most games are playable; a few need frameskip. No analog stick, which matters for some titles. Optional `scph1001.bin` (or the filename that build documents), supplied by Jeff, never downloaded. `.chd` / `.pbp` preferred on a FAT32 TF2 because of the 4 GiB file limit. |
| Arcade | FBNeo | **FinalBurn Alpha** first; an OpenDingux FBNeo only if it is proven on JZ4770 | Neo Geo and CPS1/2 are the sweet spot. Later boards vary. 512 MB will not love a huge FBNeo set. Neo Geo needs `neogeo.zip` supplied by Jeff. FBNeo’s non-commercial terms still apply: Slot stays free, and we do not ask for donations. |
| Sega 32X | PicoDrive | **PicoDrive** | Mixed. Some titles are fine, heavier ones struggle. Ship the shelf, do not promise the whole library. |

### Do not ship by default

| Shelf | Why |
| :--- | :--- |
| **Nintendo 64** | Mupen64 Plus on OpenDingux is a proof of concept. Only the easiest games are barely playable, and this handheld has no analog stick. Keep the platform in the code, hide the shelf until/unless a specific title is proven. |
| **Nintendo DS** | DeSmuME builds exist and are unplayably slow. No touch screen. Hide the shelf. |
| **PlayStation Portable** | Not realistic on this SoC. Hide the shelf. |

Dreamcast, Saturn, GameCube, PS2, 3DS, Vita, Xbox and Switch are Android / Slotted-only in this repository and are out of reach here.

**Count:** 17 shelves in good shape, 4 with caveats (SNES, PS1, Arcade, 32X), 3 hidden. That is a real Slot, not a toy port, as long as we are honest about N64/DS/PSP.

Optional later, not in the 24: WonderSwan (SwanEmu), Neo Geo Pocket (NGPCemu), Pokémon Mini, Amiga (UAE4All) all have mature OpenDingux OPKs. We should not add shelves until the 21 shipped ones feel good.

---

## 6. Card layout and what to buy

No card is flashed or formatted as part of this plan.

### Layout

```
TF1 / INT   (boot, ext4 + boot partition, Adam v2.1 + Slot)
  /usr/local/sbin/frontend_start   -> starts Slotted, falls back to GMenu2X
  /media/data/apps/slotted.opk     -> the frontend
  (GMenu2X, SimpleMenu, standalone OPKs, RetroArch remain installed)

TF2 / EXT   (games and user data, FAT32; exFAT is possible on Adam >= 2.0)
  Slot/Games/{GB,GBC,GBA,NES,SNES,PS1,MD,SMS,GG,...}
  Slot/BIOS/                       -> only files Jeff supplies
  Slot/Saves/  Slot/States/  Slot/Labels/  Slot/Config/  Slot/Fonts/
  Slot/Overlays/                   -> 320×480 PNGs, not the Brick’s 1024×768
  (Adam may also create roms/, bios/, savestates/ at the TF2 root for
   standalones in phase 3; we can symlink or follow Adam’s paths until
   cores are embedded in phase 5)
```

Keep the `Slot/` folder shape on TF2 so a Brick library can be copied the way `rg35xxsp/copy-from-brick.sh` already does: Games, BIOS, Saves, States, Labels, Fonts, Config. Do not copy `System/` (wrong CPU) or 1024×768 overlays. The frontend binary and OPK are named Slotted; the library folder name stays `Slot/` so cards can be shared.

Conservative TF2 format: **FAT32, MBR**, one partition. Older Adam guides also wanted a **blank volume label**; follow the Adam wiki at flash time. Adam v2.0 added exFAT, which avoids the 4 GiB file limit for large PS1 images, but FAT32 is the path every guide documents. Prefer `.pbp` or split dumps for discs that would exceed 4 GiB.

### What to buy

| Slot | Size | Brand and grade |
| :--- | :--- | :--- |
| **TF1** | **16 GB or 32 GB** | Name-brand **SanDisk Extreme** or **Samsung EVO Plus / EVO Select**, or a **Samsung Pro Endurance** / SanDisk High Endurance if we want the card to take many re-flashes. Adam is about 3.2 GB; 8 GB would boot, 16 GB is the comfortable minimum. Games do not go here. |
| **TF2** | **64 GB**, or **128 GB** if the PS1 set is large | The same brands. 64 GB holds the cartridge systems plus a solid PS1 set. Official maximum is **256 GB**. A1/A2 UHS-I is enough; this SoC is not a modern high-speed host. |

Do not use no-name cards, and do not trust a card that came in the box. Anbernic’s included cards are a known failure point on this model. Two cards, not one: TF1 stays small and is rarely rewritten; TF2 carries the library.

---

## 7. Adaptations

### UI and the 320×480 portrait panel

Do not pretend this is a 640×480 landscape device. Two steps:

1. **Bring-up layout (phase 2).** Integer-scale the existing 640×480 logical layout to **320×240**, centred on the 320×480 panel, 120-pixel bars above and below. It is exact half resolution, so pixels stay sharp. On a 2.8-inch screen that picture is small, but it proves GLES, evdev, ALSA, and the shelf without a redesign.
2. **Native portrait layout (phase 4).** A 320×480 `OUT_W` / `OUT_H` (new `rg280v` cargo feature, alongside `brick` and `rg35xxsp`). Vertical carousel, larger carts, title and metadata in the spare portrait space, insert animations playing in the long axis so the cart drops toward a slot at the bottom of the screen. This is the product we should ship.

Overlays, when we add them, are 320×480 PNGs, not 1024×768 and not 640×480.

### Insert animations

The 3D work is already done offline. Shells are modelled in Blender and shipped as **pre-rendered frame sequences** in `assets/carts/<sys>/insert/`; `crates/slot-ui/src/turn.rs` composites the game’s label onto each frame. The six original systems spin from the live shelf face (64 frames at 30 fps, then a 0.5 s hold). Game & Watch is a 2D open. Nothing is real-time 3D on the device.

That is the right approach on a ~1 GHz MIPS with a weak GC860:

- Re-export or downscale the existing frames to a portrait-friendly size (on the order of 200–320 px on the long edge, not Brick-sized textures).
- Keep the same timing (turn, 0.5 s hold, drop).
- If fillrate or RAM is tight, drop to 16–20 fps or fewer unique frames; the easing already lives in the frame set.
- Do not play the inserts as h.264 and hope for the VPU in phase 2. Sprite frames are enough. Software-decoded short clips are a later experiment only if frames are not smooth.
- Do not turn on heavy lcd3x / grade shaders at 320×480 until a simple blit is at 60 fps. The GPU is a 2011 GLES 2 part; a sprite shelf should be fine, a stacked post-process may not.

### Performance budget

- One 1.0 GHz MIPS32r2 core, 512 MB DDR2, GC860 GLES 2, IPU for emulator scaling.
- Target **60 fps** for the idle shelf, **30 fps** acceptable during an insert.
- Budget RAM so OS + Slot + one system’s insert frames + one core stay comfortably under 512 MB. Do not preload every system’s full-size frames.
- In game, prefer the emulator’s own IPU/KMS path once we embed cores; do not composite a 640×480 UI shader stack on top of PS1.

### Controls

Map evdev once on hardware (D-pad, ABXY, L1/L2/R1/R2, Start, Select, Volume, Power). Suggested menu conventions, to be confirmed against Adam’s SimpleMenu habits and Slot’s Brick map:

- **A** confirm, **B** back, **Start** or **Select** menu, **L1/R1** change shelf, **Power** (short) sleep or the power menu, **Power** (hold) shutdown.
- No stick, no lid, no LEDs, no Wi-Fi rows. Hide Update, Wi-Fi, and two-player link. The SP already hides Update for the same “wrong binary” reason; here the hardware cannot join a network at all.
- Rumble can be a later nicety (the motor exists).

---

## 8. Phased build plan and risks

Nothing in phases 0–1 writes a card until Jeff says yes.

### Phase 0 — Bring the hardware up (Adam only)

1. Buy the cards in §6.
2. After an explicit yes: flash Adam v2.1 to TF1, select the **RG280V** kernel, format TF2 as agreed, boot SimpleMenu, confirm the screen, the pad, sound, and charging.
3. Confirm that a cold boot without TF1 is still a black screen, so the earlier behaviour is explained.

**Risk:** wrong kernel, swapped slots, or a bad card. Mitigation: follow Adam’s install notes, Cancel every format prompt, verify the image checksum, use name-brand cards.

### Phase 1 — Toolchain and a hello OPK

Docker `linux/amd64`, gcw0 2022-09-22 toolchain, custom hard-float uClibc Rust target, `-Z build-std`. A tiny OPK that:

- gets an EGL or SDL2/KMS surface at 320×480
- clears the screen to a known colour and draws a few pixels
- logs evdev codes
- plays a tone through ALSA

Copy it to TF2 `apps/` or `/media/data/apps` and launch it from GMenu2X. This is the go/no-go for the ABI and the graphics path.

**Risks:** soft-float mismatch; `std::env::args` breakage if the target `env` is wrong (documented by MaulingMonkey); Mesa EGL rejecting the Mali fbdev window; nightly `build-std` fighting the 1.96 pin. Mitigation: prove C first, then Rust; keep this work in Docker, off the ARM toolchains.

### Phase 2 — Slot shell, letterboxed

New feature `rg280v`. The on-device name is Slotted. 320×240 letterboxed shelf, evdev, ALSA, battery and backlight if sysfs exists, network crates off, GMenu2X fallback in `frontend_start`. No emulators yet. Downscaled insert frames if they are cheap; a still cart is acceptable here.

**Risks:** 512 MB and large PNG assets; font size at 320×240 on 2.8 inches. Mitigation: cut asset resolution; treat this layout as temporary.

### Phase 3 — Play games through existing OpenDingux emulators

Launch standalones / RetroArch with the selected ROM, the way Slotted launches third-party Android emulators. Ship the “runs well” shelves first, then SNES, PS1, Arcade, 32X. Hide N64, DS and PSP. BIOS files are only those Jeff places in `Slot/BIOS/` or the path that OPK already documents.

**Risks:** save and state paths will not match the Brick until we embed cores; some OPKs want games under Adam’s `roms/` tree. Mitigation: document the paths, symlink if needed, do not promise Brick state compatibility until phase 5.

### Phase 4 — Portrait UI and insert animations

Native 320×480 layout, scaled insert sequences, simple GLES blit at 60 fps, then decide whether lcd3x is affordable. This is what we should put in a player’s hands.

**Risk:** the redesign is most of the remaining visual work. Mitigation: keep 320×240 as a compile-time fallback.

### Phase 5 — Embed mipsel libretro cores

Cross-compile the cores we already vendor, where they have a MIPS path or are portable C, and load them in-process the way Brick/SP do. Priority: Gambatte or mGBA, ReGBA or gpSP, FCEUmm, PocketSNES or Snes9x, PicoDrive / Genesis Plus GX, PCSX ReARMed or PCSX4All, Temper, Handy, the Atari / Coleco / Intellivision / GW / Vectrex cores.

**Risks:** missing MIPS dynarecs; FBNeo size and licence; PS1 compatibility differences versus PCSX4All. Mitigation: leave phase 3 launchers in place per system until that core is proven on device.

### Phase 6 — Polish

Sleep and shutdown, power menu, rumble, 320×480 overlays, a computer-side or USB-tether path for cart art and RetroAchievements, wiki attribution, hide unsupported shelves in Library → Systems by default. Still no BIOS downloads, and still no card flash without a yes.

### Risk summary

| Risk | Severity | What we do |
| :--- | :--- | :--- |
| Rust Tier 3 + uClibc hard-float + 1.96 | High | Custom target, `build-std`, hello OPK before the full crate graph |
| `ring` / rustls on mipsel | High | Gate network features off; this hardware has no Wi-Fi anyway |
| EGL window (Mali fbdev vs KMS/GBM/SDL2) | High | Solved in the hello OPK, or the port does not proceed |
| GC860 fillrate / 512 MB | Medium | Downscaled frames, simple blit, no preload of every insert |
| Portrait UI is a redesign | Medium | Letterbox first, portrait as the ship layout |
| SNES / PS1 / 32X / arcade not perfect | Low (expected) | Ship with honest caveats |
| N64 / DS / PSP | n/a | Hidden |
| Adam image is from 2022 | Medium | Still the supported base; toolchain is the 2022-09-22 tag; we can apply the official OD update OPK later |
| Wrong TF slot or wrong kernel | High for first boot | Written into phase 0; Cancel format prompts; checksum the image |
| No Wi-Fi | Accepted | Hide those menus; art and achievements later over USB or a desktop tool |
| FBNeo licence | Accepted | Slot stays free; no donation prompts |

---

## 9. What this document is not

- It is not permission to flash TF1 or TF2.
- It is not a BIOS pack. We never download BIOS, keys, or firmware. Jeff supplies any BIOS he wants to use.
- It is not a promise that N64, DS or PSP will be playable.
- It is not a rename of the Brick or SP builds. Those stay Slot. The RG280V port and the Android APK are both called Slotted.

When Jeff has cards and says to flash TF1, the first concrete step is phase 0: Adam v2.1, RG280V kernel, a known-good boot into SimpleMenu, then phase 1’s hello OPK.

---

## Sources

- Anbernic, [RG280V product page](https://anbernic.com/collections/handheld-game-console/products/anbernic-rg280v) — screen 320×480, “4770 dual 1.0 GHz,” 512 MB DDR2, TF up to 256 GB, OpenDingux, 2100 mAh.
- [Handhelds Arena, RG-280V](https://handheldsarena.com/devices/anbernic/rg-280v/) — no internal storage; 320×480 2:3 IPS; GC860; L1/L2/R1/R2; portrait form factor.
- [retroshelf, RG280V](https://www.retroshelf.org/devices/anbernic-rg-280v) — JZ4770, 512 MB, 320×480, system ratings through PS1.
- [Wikipedia, Ingenic Semiconductor](https://en.wikipedia.org/wiki/Ingenic_Semiconductor) — JZ4770: XBurst1, MIPS32 r2, 1.0 GHz, FPU, Vivante GC860, 1080p VPU described as a secondary 500 MHz MIPS core.
- [HandheldWiki, Ingenic JZ4770](https://handheldwiki.com/chip/ingenic-jz4770/) — single-core XBurst @ 1.0 GHz, GLES 2.0, Etnaviv.
- Retro Dodo, [RG280V review](https://retrododo.com/rg280v/) and [RG350 review](https://retrododo.com/rg350/) — controls, 320×480, no sticks, PS1 good, N64 poor, TF1 vs TF2.
- [GBAtemp, JZ4770 handheld comparison](https://gbatemp.net/threads/anbernic-jz4770-retro-handhelds-comparison-rg280v-rg350p-rg350m.580864/).
- [RETROPULSE, RG-280V](https://theretropulse.com/handheld-database/rg-280v).
- Retro Game Corps, [RG350 & RG280 starter guide](https://retrogamecorps.com/2021/08/20/rg350-rg280-series-starter-guide/) and [OpenDingux Beta firmware guide](https://retrogamecorps.com/2020/12/05/opendingux-beta-firmware-for-rg350-and-rg280-devices/) — Adam, TF1/TF2, OPK list, kernel 5.x, KMS, Etnaviv.
- [Joey’s Retro Handhelds, RG280V/RG280M setup](https://www.joeysretrohandhelds.com/guides/anbernic-rg280v-rg280m-setup-guide/).
- [eduardofilo/RG350_adam_image](https://github.com/eduardofilo/RG350_adam_image) and [v2.1](https://github.com/eduardofilo/RG350_adam_image/releases/tag/v2.1) (16 July 2022): ODbeta 2022-02-13, kernel 5.15, RetroArch 1.10.3, exFAT since v2.0.
- [OpenDingux/buildroot releases, od-2022.09.22](https://github.com/OpenDingux/buildroot/releases/tag/od-2022.09.22) — latest tagged gcw0 toolchain and update OPK (18 October 2022).
- [OpenDingux wiki, Frontend](https://github-wiki-see.page/m/retrogamehandheld/OpenDingux/wiki/Frontend) — `frontend_start`, GMenu2X.
- [pcercuei, opendingux.net](https://pcercuei.gitlab.io/OpenDingux.html) — SDL 1.2 vs IPU, KMS, SDL2.
- [retrogamehandheld/OpenDingux software list](https://github.com/retrogamehandheld/OpenDingux).
- rustc book, [Platform Support](https://doc.rust-lang.org/nightly/rustc/platform-support.html) and [mipsel-unknown-linux-gnu](https://doc.rust-lang.org/stable/rustc/platform-support/mipsel-unknown-linux-gnu.html) — Tier 3, no prebuilt artifacts.
- [MaulingMonkey/rust-opendingux-test](https://github.com/MaulingMonkey/rust-opendingux-test) and [Notes.md](https://github.com/MaulingMonkey/rust-opendingux-test/blob/master/Notes.md) — custom uClibc hard-float target, soft-float trap, `std::env::args`.
- Community boot and LED threads: [r/RG280V, faint green light / wrong TF slot](https://www.reddit.com/r/RG280V/comments/1p731sd/if_your_rg280v_has_a_faint_green_light_but_will/); [r/RG280V, black screen, red charge LED](https://www.reddit.com/r/RG280V/comments/11bigzn/rg280v_wont_power_on_black_screen_battery_works/); [r/SBCGaming, will not turn on](https://www.reddit.com/r/SBCGaming/comments/zvifk2/rg280v_not_turning_on/); [r/RG350, solid vs blinking red](https://www.reddit.com/r/RG350/comments/rexlvv/rg280v_charging_red_light_vs_blinking_red_light/).
- This repository: `crates/slot-store/src/platform.rs` (24 systems), `docs/rg35xxsp.md` (SP card layout and 640×480 layout), `crates/slot-gfx/src/fbdev.rs` and `surface.rs` (EGL/GLES, 640×480 logical layout, 1024×768 Brick panel), `crates/slot-ui/src/turn.rs` (pre-rendered insert frames), `crates/slot/src/input/evdev.rs`, device ALSA audio.

