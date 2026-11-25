# Cartridge shells

The original six systems (GBA, GB, GBC, NES, SNES, PlayStation) are drawn in code.
Every system added after them is a PNG shell plus a label window. The shelf
composites `Labels/<DIR>/<stem>.png` into that window.

Drop a finished render in `assets/carts/<sys>/` and rebuild Slot. Nothing else
has to change if the label window stays where `meta.toml` says.

## Files

```
assets/carts/<sys>/shell.png    required  RGBA, the whole cart, alpha outside the plastic
assets/carts/<sys>/mask.png     optional  same pixel size as shell.png
assets/carts/<sys>/meta.toml    required  the label window
```

`<sys>` is the short name, not the folder on the card:

| Card folder | `<sys>` | Shell pixels | Label window `x, y, w, h` |
|---|---|---|---|
| `Games/MD` | `md` | 360 × 200 | 40, 28, 280, 128 |
| `Games/SMS` | `sms` | 240 × 320 | 32, 48, 176, 200 |
| `Games/GG` | `gg` | 200 × 280 | 28, 64, 144, 144 |
| `Games/ARCADE` | `arcade` | 360 × 140 | 18, 16, 324, 108 |
| `Games/PCE` | `pce` | 320 × 180 | 28, 22, 264, 124 |
| `Games/A2600` | `a2600` | 220 × 340 | 36, 70, 148, 180 |
| `Games/INTV` | `intv` | 240 × 320 | 30, 40, 180, 210 |
| `Games/LYNX` | `lynx` | 280 × 160 | 24, 22, 232, 108 |
| `Games/A7800` | `a7800` | 240 × 300 | 28, 36, 184, 200 |

The placeholders in those directories are flat stand-ins. Replace `shell.png`
(and `mask.png` where there is one) with the Blender render at the same size.
`assets/carts/make-placeholders.py` rewrites the stand-ins; it is not part of
a card build.

## `meta.toml`

```toml
# Pixels of shell.png. Origin is the top-left. y grows downward.
x = 40
y = 28
w = 280
h = 128

# Clockwise degrees, around the centre of the label window. 0 is upright.
rotation = 0.0

# Width / height of that window. The art tool uses it as the label's shape.
aspect = 2.1875

# Optional. Eight numbers, TL TR BR BL, in shell pixels. When set, the label
# is drawn into this quad instead of the axis-aligned window.
# quad = [40, 28, 320, 32, 316, 156, 44, 150]
```

`x + w` must be ≤ the shell width, and `y + h` ≤ the shell height.

## Mask

`mask.png` is the same size as `shell.png`. Only its alpha is read. The label
is multiplied by that alpha, so 0 keeps the shell (a recess, a rounded
marquee, a notch) and 255 shows the label. No mask means the window is a solid
rectangle.

Arcade and Atari 2600 ship a mask. The others do not.

## Label pictures

`Labels/<DIR>/<stem>.png` is scaled to cover the window (the middle is kept)
and then rotated and masked. A game with no picture gets a plain title plate
in the window.

`brick/build_sp_art.py` fills those files from an ES-DE `downloaded_media`
tree first (physical media, then covers, then marquees; arcade tries the
marquee first) and from libretro-thumbnails if nothing local matches.
