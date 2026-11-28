# Systems and Cores

Each system has its own shelf and its own folder under `Slot/Games/`. **HOME → Library → Systems** shows or hides
each shelf. Cores are libretro cores, built from the pinned source in `vendor/<core>-src.tar.gz`.
Each core's licence is listed in [Credits and Attributions](Credits-and-Attributions.md#emulator-cores).

## On main

| Shelf | Folder | Files | Core | BIOS (`Slot/BIOS/`) |
| :--- | :--- | :--- | :--- | :--- |
| Game Boy Advance | `GBA` | `.gba` (+ `.zip`/`.7z`) | mGBA, or gpSP (pick per cart: START → core) | `gba_bios.bin` optional |
| Game Boy | `GB` | `.gb` `.gbc` | mGBA | none |
| Game Boy Color | `GBC` | `.gb` `.gbc` | mGBA | none |
| NES / Famicom | `NES` | `.nes` | FCEUmm | none |
| SNES / Super Famicom | `SNES` | `.sfc` `.smc` | Snes9x | none |
| PlayStation | `PS1` | `.cue`/`.bin` `.chd` `.pbp` `.iso` `.m3u` `.exe` | PCSX ReARMed | `scph5501.bin` etc. optional; else the core's own HLE BIOS |
| Genesis / Mega Drive | `MD` | `.md` `.gen` `.smd` `.bin` | Genesis Plus GX | none |
| Master System | `SMS` | `.sms` | Genesis Plus GX | none |
| Game Gear | `GG` | `.gg` | Genesis Plus GX | none |
| Arcade | `ARCADE` | `.zip` (the set itself, MAME/FBNeo short name) | FinalBurn Neo | `neogeo.zip` for Neo Geo |
| TurboGrafx-16 / PC Engine | `PCE` | `.pce` `.sgx` | Beetle PCE Fast | none |
| Atari 2600 | `A2600` | `.a26` `.bin` | Stella 2014 | none |
| Intellivision | `INTV` | `.int` `.bin` | FreeIntv | `exec.bin` and `grom.bin`, both required |
| Atari Lynx | `LYNX` | `.lnx` | Handy | `lynxboot.img` |
| Atari 7800 | `A7800` | `.a78` `.bin` | ProSystem | none |
| Game & Watch | `GW` | `.mgw` (MADrigal simulators) | gw-libretro | none; there are no save states |

Cartridge ROMs (not PS1 or Arcade) can be `.zip` or `.7z` with one ROM inside. A ROM is unpacked to
`Slot/System/Cache/Unpacked` when it starts.

Pinned core commits (from `brick/build-cores.sh` and the vendor tarballs):

| Core | Source | Commit |
| :--- | :--- | :--- |
| mGBA | libretro/mgba | `e31759b2` |
| gpSP | libretro/gpsp | `7b2dd373` |
| FCEUmm | libretro/libretro-fceumm | `7a542dab` |
| Snes9x | libretro/snes9x | `fae2fea0` |
| PCSX ReARMed | libretro/pcsx_rearmed | `c8816799` |
| Genesis Plus GX | libretro/Genesis-Plus-GX | `099c962e` |
| FinalBurn Neo | libretro/FBNeo | `2e6605c4` |
| Beetle PCE Fast | libretro/beetle-pce-fast-libretro | `4cf066e2` |
| Stella 2014 | libretro/stella2014-libretro | `7d1361e4` |
| FreeIntv | libretro/FreeIntv | `ef3e0fe3` |
| Handy | libretro/libretro-handy | `bc55d462` |
| ProSystem | libretro/prosystem-libretro | `8a880142` |
| gw-libretro | libretro/gw-libretro | `91d599b9` |

## In PR #31 (also in PR #32): eight new shelves

| Shelf | Folder | Files | Core (commit) | BIOS |
| :--- | :--- | :--- | :--- | :--- |
| ColecoVision | `COLECO` | `.col` `.rom` | Gearcoleco (`020740f6`) | `colecovision.rom` **required** |
| Atari 5200 | `A5200` | `.a52` `.bin` | a5200 (`40c6f2f1`) | `5200.rom` used if present |
| Atari 800 | `A800` | `.xex` `.atr` `.atx` `.xfd` `.car` `.cas` | Atari800 (`4e7fbc73`) | AltirraOS built in; `ATARIXL.ROM` and `ATARIBAS.ROM` used if present |
| Vectrex | `VECTREX` | `.vec` `.gam` | vecx (`dccff540`) | none. White vectors, no colour overlays. |
| 32X | `32X` | `.32x` `.bin` | PicoDrive (`1890c293`) | none |
| Nintendo 64 | `N64` | `.z64` `.n64` `.v64` | mupen64plus-next (`8623e490`), GLES | none |
| Nintendo DS | `NDS` | `.nds` | melonDS (`4081e835`) | `bios7.bin`, `bios9.bin` and `firmware.bin` optional |
| PSP | `PSP` | `.iso` `.cso` `.chd` `.pbp` | PPSSPP v1.20.4, GLES | PPSSPP assets in `Slot/BIOS/PPSSPP`, included in the card package |

- N64 and PSP have no analog stick on these handhelds, so the D-pad drives the stick. On PSP, hold L2 to use the D-pad
  as a D-pad. N64's C buttons are on the face and shoulder buttons.
- The DS shows both screens but has no touch input yet. DraStic, the usual DS emulator on these handhelds, is closed
  source and can't be redistributed, so the DS shelf runs melonDS.
- For N64 and PSP, PR #31 also fixes two threading problems: a core that asks for its framebuffer from its own thread
  now gets it, and a core that switches threads with libco can no longer corrupt Slot's floating-point registers.

### Curated N64, DS and PSP lists

These handhelds only manage part of each library, so PR #31 ships lists of games that community reports say run
well on the H700. The lists are in `docs/curated/`:

| List | Games | What the list is based on |
| :--- | ---: | :--- |
| `curated-n64.csv` | 79 | "H700 mupen64plus-next GLideN64 reports (Knulli/muOS/Retro Handhelds N64 sheet)" |
| `curated-nds.csv` | 56 | "H700 melonDS (arm64 JIT) reports; 2D or light 3D, plays on buttons" |
| `curated-psp.csv` | 28 | 20 from "Jkid873 H700 PPSSPP sheet (r/SBCGaming, RG35XX H/Plus/SP)", 8 from "H700 community lists (Knulli/muOS PSP threads)" |

Each row gives the file, the title, the size, the reported speed and the reason. The PSP list also says how each game
was staged, for example converted to CHD. The source sheets aren't linked from the repo. Their exact URLs are
**needs confirmation**.
