# Cart art from ScreenScraper

`crates/slot-cartart` turns ScreenScraper pictures into the label PNG a shelf composites into a
cart (`Labels/<DIR>/<stem>.png`). It is plain Rust with one dependency (`png`), so it builds for
the Brick, the RG35XX SP, Android and the RG280V alike. It runs once per game, when the art is
fetched, and the result is kept on the card; nothing runs per frame.

## Turning it on

`Config/screenscraper.txt` on the card:

    cartart=on
    user=<ScreenScraper user name>
    password=<ScreenScraper password>

The developer id and password are built in from `SLOT_SS_DEVID` / `SLOT_SS_DEVPASSWORD` at
build time (or `devid=` / `devpassword=` in the same file for testing). Without `cartart=on`
and a developer id, HOME → Cart Art works exactly as before. When it is on,
`cart_art::screenscraper_label` is tried first for every cart and anything it can't fit falls
through to the existing sources.

`crates/slot/src/cart_art_ss.rs` does the fetching: one `jeuInfos` lookup per game (ROM name,
CRC for ROMs up to 64 MB, size), then only the pictures it needs, in tier order, each asked for
at no more than 1000 px. Calls are 1.2 s apart (the developer quota is one thread), only
`*.screenscraper.fr` is contacted, every credential is stripped from anything logged
(`redact`), and a quota or auth error (401/403/423/426/429/430/431) stops ScreenScraper for the
rest of the run.

## Tiers

Each system has a profile (`profile.rs`) with the order to try:

| Profile | Order | Systems |
|---|---|---|
| cart | support-texture, support-2D, box-2D, wheel | GB, GBC, GBA, NES (and Famicom), SNES, MD, SMS, GG, PCE, A2600, INTV, LYNX, A7800, COLECO, A5200, 32X, N64, NDS, PSP |
| disc | support-2D, support-texture, box-2D, wheel | PS1 |
| handheld | support-2D (the whole handheld, on transparency) | GW |
| sticker | support-texture, support-2D, wheel, box-2D | A800 (floppy sticker) |
| box first | box-2D, support-texture, support-2D, wheel | VECTREX |
| arcade | marquee (the real one; screenmarquee is ScreenScraper's generated banner), wheel, box-2D | ARCADE |

The label scan (support-texture) comes before the cart photo for carts: the shelf draws the
cart itself and needs only the label, and the scan is flat, square-on and sharper. A picture
that is not the label's shape (a wrap of the whole cart, the end strip) is passed over.

## What it does to a picture

1. Finds the object: alpha when the picture has a backdrop of transparency, else a flood from a
   flat border colour. If the flood eats into the picture (flat-coloured art), the picture is
   kept whole.
2. Straightens it when four edges agree on a tilt between 0.3° and 8° (never for discs).
3. Cuts the label: a label scan is the label (less a 1.2% inset for rounded corners); on a cart
   photo the label window is cut by the system's fractions and refined to the plastic's edge.
4. Fits it to the window: a stretch nobody can see (≤ 6%), else cut to shape where the detail is
   (up to the profile's `label_cut`), else the flat edge is carried out, else it sits on a soft
   blur of itself. A box on a wide window gives its top (the title).
5. Polishes: a gentle levels stretch (gain at most 1.3), +12.5% saturation for photos, unsharp
   mask, opaque output (a Game & Watch keeps its outline).

Labels are written at twice the window, at most 512 px across, as RGB PNG (fast deflate).

## Test tooling

- `tools/cartart/games.tsv`, `fetch_ss.py` (the developer credentials file is passed with
  `--creds`; no URL is ever printed or written) and `sheet.py` (contact sheets and an HTML page).
- `cargo run --release -p slot-ui --example cartart_preview -- ROOT [SCALE] [SYS,…]` fits every
  game under `ROOT/<DIR>/raw/` and draws it on its cart.
- `cargo run --release -p slot-ui --example cartart_bench -- ROOT [RUNS]` times the card's path.
