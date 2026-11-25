# Cartridge shells

The original six systems (GBA, GB, GBC, NES, SNES, PlayStation) are drawn in code.
Every system added after them is a PNG shell plus a label window. The shelf
composites `Labels/<DIR>/<stem>.png` into that window.

Drop a finished render in `assets/carts/<sys>/` and rebuild Slot. Nothing else
has to change if the label window stays where `meta.toml` says.

## Files

```
assets/carts/<sys>/shell.png    required  RGBA, the whole cart, alpha outside the plastic
assets/carts/<sys>/mask.png     optional  same pixel size; alpha 255 shows the label
assets/carts/<sys>/shade.png    optional  same pixel size; grey, multiplies the label
assets/carts/<sys>/meta.toml    required  the label window
```

`<sys>` is the short name, not the folder on the card. The shells are Blender
renders (parametric models built from measured cart sizes and label specs; the
scripts and .blend files live in `~/Projects/slot-cart-renders` on the build Mac),
downscaled from 960 px masters to 360 px wide, the width of the drawn GBA and SNES
carts, or narrower for a tall cart: none is taller than the NES cart (331 px), so a Sega
Card or a HuCard leaves the top of the screen clear. Seated in the slot, a tall cart is
likewise no taller than the NES cart.

| Card folder | `<sys>` | Cart | Shell pixels | Label window `x, y, w, h` |
|---|---|---|---|---|
| `Games/MD` | `md` | Genesis (US Sega shell: plain front, smooth side bulges) | 360 × 247 | 60, 9, 240, 197 |
| `Games/SMS` | `sms` | Sega Card (US) | 213 × 331 | 16, 15, 181, 198 |
| `Games/GG` | `gg` | Game Gear | 322 × 331 | 26, 40, 270, 207 |
| `Games/ARCADE` | `arcade` | backlit marquee | 360 × 122 | 15, 16, 330, 90 |
| `Games/PCE` | `pce` | HuCard / TurboChip | 213 × 331 | 16, 77, 181, 237 |
| `Games/A2600` | `a2600` | Atari 2600 (Atari shell) | 279 × 331 | 26, 31, 227, 279 |
| `Games/INTV` | `intv` | Intellivision, end label on the sloped end | 300 × 331 | 25, 251, 250, 59 |
| `Games/LYNX` | `lynx` | Lynx card | 316 × 331 | 15, 53, 286, 263 |
| `Games/A7800` | `a7800` | Atari 7800 | 279 × 331 | 26, 31, 227, 279 |
| `Games/GW` | `gw` | none: each game's own handheld (below) | 360 × 331 | 0, 0, 360, 331 |

### Game & Watch

A Game & Watch is not a cart, so `assets/carts/gw/shell.png` is clear and its label
window is the whole box: the label *is* the shelf face. Each game's handheld is rendered
from its owner's ES-DE media by `brick/gw-art` (Blender) and kept on the card:

- `Labels/GW/<game>.png`, 360 × 331: the handheld on the shelf, a clamshell closed, any
  other with its screen unlit.
- `Labels/GW/open/<game>.png`, 18 frames of 360 × 331 side by side: picking the game plays
  them (a clamshell opens and its screens light; the rest light up) before it goes into the
  slot, open. Ejecting plays them backwards. With no strip, the handheld goes in as it is.

Each state is fitted to the box on its own and stands on the box's bottom edge, so a
clamshell grows as it opens. `brick/gw-art/devices.csv` says which games fold, where, and
where their screens are; `kit.py`, `render.py` and `pack.py` (or `pack-all.py`) make the
files. A card built on this machine takes them from `brick/labels-local/GW` (not in git).

`assets/carts/sms/cart-variant/` holds the Master System cartridge (title strip
label) in the same format. It is not built in; swap it with the files above it to
use the cartridge instead of the card.

## `meta.toml`

```toml
# Pixels of shell.png. Origin is the top-left. y grows downward.
x = 60
y = 9
w = 240
h = 197

# Clockwise degrees, around the centre of the label window. 0 is upright.
rotation = 0.0

# Width / height of that window. The art tool uses it as the label's shape.
aspect = 1.2183

# Optional. A lit marquee: the label is brightened as if lit from behind, and
# shade.png is not used.
# backlit = true

# Optional. Eight numbers, TL TR BR BL, in shell pixels. When set, the label
# is drawn into this quad instead of the axis-aligned window.
# quad = [40, 28, 320, 32, 316, 156, 44, 150]
```

`x + w` must be ≤ the shell width, and `y + h` ≤ the shell height.

## Mask

`mask.png` is the same size as `shell.png`. Only its alpha is read. The label
is multiplied by that alpha, so 0 keeps the shell (a recess, a rounded label
corner, the marquee's retainers) and 255 shows the label. No mask means the
window is a solid rectangle. Every shipped shell has one.

## Shade

`shade.png` is the same size as `shell.png`. Its red channel multiplies the
label (255 leaves a pixel as printed): it is the same render with a blank white
label in place, so the art takes the cart's light, the darker label foot on a
Genesis, the falloff on the Intellivision's sloped end. A `backlit` shell
ignores it.

## Label pictures

`Labels/<DIR>/<stem>.png` is scaled to cover the window (the middle is kept)
and then rotated and masked. A game with no picture gets a plain title plate
in the window.

`brick/build_sp_art.py` fills those files from an ES-DE `downloaded_media`
tree first (physical media, then covers, then marquees) and from
libretro-thumbnails if nothing local matches. The Sega Card takes the cover
first. Arcade takes a marquee: ES-DE's for the set, then its parent's
(`--arcade-dat`), then the arcadeitalia scan, before any other picture; a marquee
that is only the game's logo, or far from the window's shape, gives way to the
arcadeitalia scan.

With Pillow installed, each label is then cut to its window ahead of time
(`brick/label_fit.py`), so the shelf's own centred cut has nothing left to do:

- a cart scan gives just its label (where it sits is known for each system's
  ScreenScraper scans; the Intellivision's end label, photographed from above, is
  made its true height again), and a scan of something else (a CD) is passed over;
- a picture within 1.3× of the window's shape is cut where its detail is, a box
  cover from the top, where its title is;
- otherwise a nearer picture is used; a picture with a plain edge (a marquee on
  black) is kept whole with that edge carried out; a logo is set whole on the art's
  colour; and failing all of those the whole picture sits on a dim blow-up of itself.
  Nothing is squeezed and no bars are added.

Labels are kept at twice the window size.

## Insert turn

A shell may carry `insert/`: the frames its cart turns through, on the shelf, before it slides
into the slot. The slide itself is the same as every other cart's (`INSERT_S`), so the turn
only adds its own length (18 frames at 30 fps, 0.6 s) in front of it. Systems without
`insert/` go straight in, as before. An eject plays the turn backwards once the cart is out,
and a cart refused mid-turn turns back from where it got to.

- `insert/NN.png`: frame NN of the shell, label window clear, cropped to what it covers.
- `insert/NN-map.png`: grey + alpha, the light the label takes (white = as printed) and how
  much of each pixel it covers.
- `insert/insert.toml`: `fps`, `flip = true` for a cart that ends the turn the other way up,
  and per frame a `uv` line (nine numbers: the projective map from a frame pixel to the flat
  label's u, v, v up) then a `frame = x, y, w, h` line (where the crop sits relative to
  `shell.png`'s top-left).

The label laid on is the shelf face's own fitted label, so every game turns with its own art.
A cart that turns over end to end (the Intellivision cart and the HuCard: their contacts are
at the top on the shelf) uses the label turned round while its back faces the screen, so it
goes in with the label upright; `flip` makes the last frame its face for the slide. The frames
come from `scripts/export_anim.py` and `scripts/pack_turn.py` in the cart renders project.
