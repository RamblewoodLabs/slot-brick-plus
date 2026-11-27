# Credits and Attributions

Slot Brick Plus is built on other people's work. Each entry below was checked against the repository on
2026-11-26: `LICENSE`, `licenses/`, `brick/SOURCES.md`, the licence files inside each `vendor/*-src.tar.gz`, git
history, and the GitHub API for the upstream repos. Anything that couldn't be checked is marked **needs
confirmation**. Corrections are welcome.

Slot itself, and every addition made in this fork, is under the **GNU GPL v3.0 or later** (`LICENSE`).

## Lineage

| Project | People | Licence | What we use |
| :--- | :--- | :--- | :--- |
| [Slot.](https://github.com/slot-cfw/slot) (formerly `BrandonKowalski/slot`) | **Brandon T. Kowalski**, author. Contributors on GitHub include MarkusMaribu, paulocoe-creator, pvaibhav, diluno, flo333, GamerAnim, karlprieb, machomano11 and others ([full list](https://github.com/slot-cfw/slot/graphs/contributors)). | GPL-3.0-or-later | The whole frontend. The port began from slot 1.5.0 (commit `118c6957`), later merged with upstream 1.6.0. Also Cart Studio and its label art set. |
| [Slot-Brick](https://github.com/LordNordyx/Slot-Brick) | **LordNordyx** (port author), with **Kahoiz / Benjamin Christiansen** (the largest share of commits in this history), **kirby-letsgo** (pause menu, zipped games), **TT-Hipster1941** (PS1 startup fix, Swap A/B, .cue handling), **wargenng / Wargen Guittap** (low-battery LED warning), and **@jyav-19** (Slot OS testing on a Brick Pro) | GPL-3.0 | The TrimUI Brick port: NES/SNES/PS1, the extra shelves, Slot OS, LEDs, sleep, Wi-Fi, updates and the simulator |
| **Slot Brick Plus** (this fork) | **Jeff Finkelstein** (jfink22-design), with AI coding agents (Cursor Agent). The upstream port's README says it was developed with help from Claude Opus. | GPL-3.0-or-later | RG35XX SP build, Favorites shelf, PR #24–#32 work |

Commit authors on `main`: Kahoiz / Benjamin Christiansen (82 commits, one GitHub ID) and benni (63; probably the same person, **needs confirmation**), jfink22-design
and Jeff Finkelstein (29), LordNordyx (4), kirby-letsgo (2), Wargen Guittap (2), TT-Hipster1941 (2), Cursor Agent (2).
Commits merged from upstream Slot keep their original authors.

## Platform references (no code shipped)

| Reference | Used for |
| :--- | :--- |
| [LoveRetro/NextUI](https://github.com/LoveRetro/NextUI) @ `5921134e`, `workspace/tg5040` | Brick button indices, screen, battery sysfs, brightness ioctl, LED sysfs and effects, suspend, the Wi-Fi helper layout, and the stock-firmware boot hook. These were consulted as references. NextUI's offline ledger was **not** copied, because that tree is PolyForm Noncommercial. |
| [TrimUI toolchain SDK](https://github.com/trimui/toolchain_sdk_smartpro) 20231018 | The Linaro gcc 7.5 cross toolchain. Its GE8300 SDL source was a reference for EGL. SDL itself is loaded from the firmware and isn't redistributed. |
| [pierceTee/TrimuiLEDController](https://github.com/pierceTee/TrimuiLEDController) | LED driver documentation |
| [Zero-Labs-CFW/MinUI-Zero](https://github.com/Zero-Labs-CFW/MinUI-Zero) | Design notes for adaptive clocks (conceptual; no code copied) |
| [big-due/TrimUI-Brick-Hardware-Testing](https://github.com/big-due/TrimUI-Brick-Hardware-Testing) | Power measurements |
| wpa_supplicant control interface docs (w1.fi), Linux CPUFreq docs | Wi-Fi client and clock controller |
| [pvaibhav/BaseOS](https://github.com/pvaibhav/BaseOS) v1.4.0 | The OS the RG35XX SP build runs on (TF1). Not redistributed. [pvaibhav/slot](https://github.com/pvaibhav/slot) 1.16 was evaluated as a reference; whether any of its code was used is **needs confirmation**. The docs say the H700 path follows upstream Slot. |
| [TrimUI firmware](https://github.com/trimui/firmware_brick) 1.1.1 ([Brick Pro](https://github.com/trimui/firmware_brickpro)), Debian 12 | Shipped inside **Slot OS** images. Their licences are in `slot-os/licenses` on the card (`brick/os/NOTICE.txt`). |

## Emulator cores

All are libretro cores. The exact source of each is in `vendor/<core>-src.tar.gz`, and the licence is taken from the
licence file **inside that tarball**.

| Core | Systems | Upstream / authors | Licence (from the source) | Pinned |
| :--- | :--- | :--- | :--- | :--- |
| [mGBA](https://github.com/libretro/mgba) | GB, GBC, GBA | Vicki Pfau (endrift) and contributors | MPL-2.0 | `e31759b2` |
| [gpSP](https://github.com/libretro/gpsp) | GBA | Exophase, notaz, libretro | GPL-2.0 | `7b2dd373` |
| [FCEUmm](https://github.com/libretro/libretro-fceumm) | NES | FCEU team, libretro | GPL-2.0 | `7a542dab` |
| [Snes9x](https://github.com/libretro/snes9x) | SNES | Snes9x team | **Snes9x licence: non-commercial** | `fae2fea0` |
| [PCSX ReARMed](https://github.com/libretro/pcsx_rearmed) | PS1 | notaz and the PCSX teams | GPL-2.0 | `c8816799` |
| [Genesis Plus GX](https://github.com/libretro/Genesis-Plus-GX) | MD, SMS, GG | Charles MacDonald, Eke-Eke | **Own licence: non-commercial**; modified redistributions must include source; the notice must be reproduced | `099c962e` |
| [FinalBurn Neo](https://github.com/libretro/FBNeo) | Arcade | FBNeo team (based on FinalBurn and MAME code) | **Own licence: non-commercial**, no donations, the licence text must be included verbatim; MAME code is also under MAME's licence | `2e6605c4` |
| [Beetle PCE Fast](https://github.com/libretro/beetle-pce-fast-libretro) | PCE / TG16 | Mednafen team, libretro | GPL-2.0 | `4cf066e2` |
| [Stella 2014](https://github.com/libretro/stella2014-libretro) | Atari 2600 | Stella team | GPL-2.0 | `7d1361e4` |
| [FreeIntv](https://github.com/libretro/FreeIntv) | Intellivision | David Richardson | GPL-2.0-or-later | `ef3e0fe3` |
| [Handy](https://github.com/libretro/libretro-handy) | Lynx | K. Wilkins | **zlib** | `bc55d462` |
| [ProSystem](https://github.com/libretro/prosystem-libretro) | Atari 7800 | Greg Stanton | GPL-2.0 | `8a880142` |
| [gw-libretro](https://github.com/libretro/gw-libretro) | Game & Watch | Andre Leiradella (also retroluxury; bundles Lua (MIT) and bzip2) | zlib | `91d599b9` |
| [Gearcoleco](https://github.com/drhelius/Gearcoleco) *(PR #31)* | ColecoVision | Ignacio Sanchez (drhelius) | GPL-3.0 | `020740f6` |
| [a5200](https://github.com/libretro/a5200) *(PR #31)* | Atari 5200 | libretro (Atari800 lineage) | GPL-2.0 | `40c6f2f1` |
| [Atari800](https://github.com/libretro/libretro-atari800) *(PR #31)* | Atari 800 | Atari800 team; AltirraOS by Avery Lee | GPL-2.0-or-later | `4e7fbc73` |
| [vecx](https://github.com/libretro/libretro-vecx) *(PR #31)* | Vectrex | Valavan Manohararajah, libretro | GPL-3.0 | `dccff540` |
| [PicoDrive](https://github.com/libretro/picodrive) *(PR #31)* | 32X | notaz, fDave | **Own licence: non-commercial**; modified builds must include source | `1890c293` |
| [melonDS](https://github.com/libretro/melonDS) *(PR #31)* | Nintendo DS | Arisotura and contributors | GPL-3.0 | `4081e835` |
| [mupen64plus-next](https://github.com/libretro/mupen64plus-libretro-nx) *(PR #31)* | N64 | Mupen64Plus team, GLideN64 (Gonetz), libretro | GPL-2.0 | `8623e490` |
| [PPSSPP](https://github.com/hrydgard/ppsspp) *(PR #31)* | PSP | Henrik Rydgård and contributors; FFmpeg (LGPL-2.1+) | GPL-2.0-or-later | v1.20.4 |

Author names are as commonly credited upstream. Licences come from the tarballs. The PR #31 licences come from
`licenses/new-cores2.txt` on that branch. **Needs confirmation:** check the author names against each repo's
AUTHORS file, and check the PR #31 licences against each tarball's licence file.

## Libraries built into Slot

| Library | Licence | Notes |
| :--- | :--- | :--- |
| [rcheevos](https://github.com/RetroAchievements/rcheevos) v12.5.0 (RetroAchievements) | MIT | Unmodified apart from leaving out archive and encryption hashing |
| libchdr (from PCSX ReARMed's `deps`), LZMA SDK 26.02, Zstandard 1.5.7, miniz 3.1.2 | BSD-3-Clause, public domain, BSD, MIT/Unlicense | `licenses/libchdr.txt` |
| [sevenz-rust2](https://github.com/hasenbanck/sevenz-rust) 0.24.1 and lzma-rust2 0.23.0 (hasenbanck; fork of dyz1990/sevenz-rust) | Apache-2.0 | Reads `.7z` |
| rustls, ring, webpki-roots, flate2, miniz_oxide and the other Rust crates | various (see `Cargo.lock`) | **Needs confirmation:** a full crate licence inventory (`cargo about` or `cargo deny`) hasn't been produced |

## Art, data and media

| Item | Source / author | Licence | Shipped? |
| :--- | :--- | :--- | :--- |
| Overlays *Vanilla* and *CRT* | [KrutzOtrem](https://github.com/KrutzOtrem/Trimui-Brick-Overlays) | MIT | Yes (Brick) |
| SP bezel `640x480.png` ("Perfect GBA SP (no grid)") | mugwomp93 ([Knulli_Customization](https://github.com/mugwomp93/Knulli_Customization)), collected in [knulli-bezels](https://github.com/chrizzo-hb/knulli-bezels) by chrizzo-hb | CC BY-SA 4.0 (scaled and recoloured, same licence) | Yes |
| Screen shaders | libretro [glsl-shaders](https://github.com/libretro/glsl-shaders): LCD3x (Gigaherz, public domain), Dot (Themaister, public domain), zfast_lcd_standard (Greg Hogan / SoltanGris42, GPL-2.0+), simpletex_lcd and its paper texture (jdgleaver, GPL-2.0+) | as listed | Yes |
| Sounds: "Pop" (favorite star) | freesound_community on Pixabay | Pixabay Content License | Yes |
| Sounds: "Video game menu click sounds", "Cart pop", "Label put on" | Pixabay (uploaders **needs confirmation**) | Pixabay Content License | Yes |
| GB/GBC/GBA cart labels | Cart Studio art set (Brandon T. Kowalski); scans from [ScreenScraper](https://www.screenscraper.fr) | **needs confirmation** (community scans) | No. Downloaded per user. |
| NES/SNES cart labels, PS1 disc scans | [LaunchBox Games Database](https://gamesdb.launchbox-app.com) | LaunchBox terms (**needs confirmation**); community scans | No. Downloaded per user. |
| Labels for the newer shelves | Your own ES-DE `downloaded_media` (ScreenScraper scrapes), [libretro-thumbnails](https://github.com/libretro-thumbnails) | libretro-thumbnails' terms and ScreenScraper's (**needs confirmation**) | No. Built per user. |
| Arcade marquees | [Arcade Database](https://adb.arcadeitalia.net) by Motoschifo (scans from progettoSNAPS by AntoPISA, **needs confirmation**), LaunchBox, libretro-thumbnails MAME `Named_Logos`, ES-DE | site terms (**needs confirmation**) | No |
| Intellivision label scans | ES-DE / ScreenScraper physical media, ES-DE wheels, libretro-thumbnails `Mattel - Intellivision`, LaunchBox | as above | No |
| Game & Watch handhelds | Rendered from the owner's ES-DE media with Blender (`brick/gw-art`) | n/a | Only on the owner's card (`brick/labels-local`, git-ignored) |
| Arcade name table (PR #28) | Titles from [FBNeo](https://github.com/finalburnneo/FBNeo) DATs, [MAME 2003-Plus](https://github.com/libretro/mame2003-plus-libretro) XML, [MAME](https://www.mamedev.org) 0.289 `-listxml` | Facts (titles), credited here | Yes (`arcade-names.tsv`) |
| Shelf rankings (PR #29) | LaunchBox Games DB public `Metadata.zip` (community rating, count, publisher, Mame.xml); [libretro-database](https://github.com/libretro/libretro-database) `metadat/publisher`; Wikipedia's "List of best-selling … video games" pages for NES, SNES, Game Boy, GBA, PlayStation, Genesis and Atari 2600 | Ranks only. Wikipedia text is CC BY-SA 4.0 and is credited here. | Yes (`rankings/*.tsv`) |
| Curated N64/DS/PSP lists (PR #31) | Community H700 reports: Knulli, muOS, the Retro Handhelds N64 sheet, Jkid873's H700 PPSSPP sheet (r/SBCGaming) | **Needs confirmation:** links not recorded | `docs/curated/` |
| NES and SNES cart drawings (SVG) | Original drawings, based on product photos: a replacement NES shell (Retrotainment Games) and a US Super Mario Kart cart (PJ's Games) | Original | Yes |
| 3D carts and shell renders | Original parametric Blender models (scripts in `~/Projects/slot-cart-renders`, not in git), built from measured sizes. The reference images in its `refs/` folder appear to be physical-media and marquee pictures (ES-DE, FBNeo and MAME); their exact provenance is **needs confirmation**. | Original; reference images not shipped | Renders yes |
| Pip, the companion | Original art and logic; not PakUI's BitPal | GPL-3.0+ | Yes (switched off) |

## Fonts

| Font | Author | Licence | Notes |
| :--- | :--- | :--- | :--- |
| `label.ttf`, Slot's own label font | Open Sans, by The Open Sans Project Authors | SIL OFL 1.1 (`crates/slot-ui/assets/label.ttf.OFL.txt`) | Compiled into the binary |
| `SymbolsNerdFontMono` (icons) | Nerd Fonts, by Ryan L McIntyre | MIT (`…ttf.LICENSE.txt`) | Compiled into the binary (subset `-Slot.ttf`) |
| GohuFont | Hugo Chargois (vector TTF from fontpixel.com) | WTFPL | `Slot/System/Fonts/GohuFont.ttf` |
| PPSSPP's PSP fonts (`flash0/font/*.pgf`) *(PR #31)* | Part of PPSSPP | PPSSPP's licence | `Slot/BIOS/PPSSPP` |

## Tools

[Blender](https://www.blender.org) (GPL; renders aren't covered by Blender's licence), [ES-DE](https://es-de.org)
(as the media source), Docker, Rust, Python with Pillow, numpy and OpenCV, Raspberry Pi Imager, balenaEtcher, Rufus,
SD Card Formatter.

## Licence obligations and known gaps

These come from comparing the licence files with what ships. The wiki only reports them; nothing has been fixed yet.

1. **Cores labelled GPL that aren't.** `licenses/new-cores-GPL-2.0.txt` says Genesis Plus GX, FinalBurn Neo and Handy
   are "GNU GPL version 2 or later". Their own licence files say otherwise: **Genesis Plus GX** and **FBNeo** have
   non-commercial licences, and **Handy** is zlib. FBNeo requires its licence text verbatim, and Genesis Plus GX
   requires its notice to be reproduced. Neither is shipped today.
2. **The RG35XX SP card ships too few licences.** `rg35xxsp/package-card.sh` copies only
   `brick/card/Slot/System/licenses` (bezel, gpSP, mGBA, shaders, Slot) and Slot's GPL. It doesn't copy `licenses/`,
   so the SP card carries the FCEUmm, Snes9x, PCSX ReARMed, FBNeo, gw-libretro, rcheevos, libchdr, sevenz-rust2, font,
   overlay and sound notices nowhere. With PR #31, it also lacks the new cores' licences. The Brick card
   (`brick/package-card.sh`) copies `licenses/` and is fine.
3. **Font licences aren't on either card.** Open Sans (OFL requires the licence to travel with the font) and the Nerd
   Font symbols (MIT notice) are compiled into `slot`, but their licence files live only in `crates/slot-ui/assets/`.
4. **Non-commercial cores.** Snes9x, Genesis Plus GX, FBNeo and PicoDrive forbid commercial use, and FBNeo also
   forbids asking for donations. Slot must stay free and unmonetised while it ships them.
5. **GPL source.** The cores' exact sources are in `vendor/*-src.tar.gz`, and Slot's own source is this repo. Release
   pages should link the matching source tag. **Needs confirmation:** check that each release does, including SP
   builds staged by hand.
6. **Wikipedia (CC BY-SA 4.0).** Only rank positions are stored, which are most likely facts. The credit above covers
   attribution either way.
7. **Rust crate licences.** No full inventory exists yet (see Libraries).
