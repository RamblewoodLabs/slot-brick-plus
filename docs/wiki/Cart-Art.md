# Cart Art, Arcade Names and Intellivision Labels

## Labels on the device (on main)

**HOME → Online → Cart Art → GET CART ART** (needs Wi-Fi) gives every game that has no label a scan of its real label.
It never replaces a label you already have.

| Shelf | Where the label comes from |
| :--- | :--- |
| GB, GBC, GBA | The slot art set behind Brandon T. Kowalski's [Cart Studio](https://studio.slot-cfw.fyi). Its scans come from ScreenScraper. Matched by the ROM's CRC or name. |
| NES, SNES | [LaunchBox Games Database](https://gamesdb.launchbox-app.com) "Cart - Front" pictures, cropped on the device. Famicom and SFC/PAL carts get the Japanese or European release's label. |
| PlayStation | LaunchBox "Disc" scans, kept whole |

Slot downloads only the pictures for your own games and ships none of them. `Slot/Logs/cart-art.txt` lists what each
game got. You can also search for one game's label and frame it to fit: START → Y.

## Labels built on a computer: `brick/build_sp_art.py`

For the newer shelves (MD, SMS, GG, Arcade, PCE, 2600, INTV, Lynx, 7800), labels are built on a computer and copied
into `Slot/Labels/<SYS>/`:

1. Your own **ES-DE** `downloaded_media` comes first (physical media, then covers, then marquees). SMS takes the cover
   first, because its Sega Card label is box-style art.
2. With `--download`, **libretro-thumbnails** (the GitHub mirror) fills the gaps: `Named_Boxarts`, or `Named_Titles`
   for Arcade.
3. `brick/label_fit.py` cuts each picture to the cart's label window. Labels are stored at 2× the window size.

### Arcade marquees

Arcade uses the marquee first, because it's the label on the wide, backlit panel (330×90 window):
ES-DE marquees, then the parent set's marquee for a clone, then the **Arcade Database** scan
(adb.arcadeitalia.net, `mame.current/marquees`), then covers or a libretro title screen.
In the MGT-15 label build (the labels themselves are not in git), that came to 560 ADB scans, 44 LaunchBox "Arcade - Marquee", 87 LaunchBox Clear Logos,
44 libretro-thumbnails MAME `Named_Logos`, 59 older ES-DE logos and 1 libretro title screen. Neo Geo's generic MVS
marquees were replaced with logos.

### Arcade names (in PR #28)

The Arcade shelf shows each game's real title ("Metal Slug - Super Vehicle-001", not `mslug`), and sorts and jumps by
letter on that title. Keep the zips named by their MAME/FBNeo short name, because the core needs it. Slot looks the
title up in `crates/slot-store/data/arcade-names.tsv` (10,934 sets), built from the **FBNeo** DATs, the
**MAME 2003-Plus** XML and **MAME 0.289** `-listxml`. The label stays `Labels/ARCADE/<short name>.png`.

### Intellivision labels (in PR #30)

The INTV cart's label is the end label on its sloped end (window 250×59 at x 25, y 251). PR #30 cuts the **whole**
end label from a cart scan, so a title's second line and the bottoms of its letters are no longer clipped. In the
rebuild, 153 labels came from ES-DE/ScreenScraper physical-media scans, 6 from clear logos (ES-DE wheels,
libretro-thumbnails "Mattel - Intellivision" `Named_Logos`, LaunchBox), and 9 were rendered as text.

## Game & Watch art

`brick/gw-art` renders each handheld in Blender from your own ES-DE media (faceplate and screenshot), producing an
unlit and a lit frame, or 18 opening frames for a clamshell. Its output goes to `brick/labels-local/GW`, which is
git-ignored, and onto the card.

## Cart shells

The shell PNGs and the 3D models of the classic carts were made in Blender from measured cart sizes and label specs.
The scripts and `.blend` files are in `~/Projects/slot-cart-renders` on the build Mac, not in git. The NES and SNES
SVG carts on main are original drawings, based on product photos (see Credits).
