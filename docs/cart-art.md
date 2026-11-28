# Cart art from ScreenScraper

`crates/slot-cartart` turns ScreenScraper pictures into the label PNG a shelf composites into a
cart (`Labels/<DIR>/<stem>.png`). It is plain Rust with one dependency (`png`), so it builds for
the Brick, the RG35XX SP, Android and the RG280V alike. It runs once per game, when the art is
fetched, and the result is kept on the card; nothing runs per frame.

## The only art source, on by default

HOME → Cart Art fetches from ScreenScraper only; the older sources (the slot-cfw.fyi art set,
LaunchBox cart pictures, and their lists in `Slot/System/`) are gone, as is the code that read
them. The per-game label search (Y on a cart, FIND A LABEL) searches ScreenScraper too
(`jeuRecherche.php`) and fits each picture the same way. Labels are written at 2× the window.

Cart Art is on by default. It needs two things:

- **A developer id, built in.** `SLOT_SS_DEVID` / `SLOT_SS_DEVPASSWORD` at build time.
  `brick/docker-build.sh` and `rg35xxsp/docker-build.sh` source `brick/ss-dev-env.sh`, which
  takes them from the environment or from `devid=` / `devpassword=` in a gitignored
  `screenscraper.properties` (repo root, `android/`, or the path in `SLOT_SS_PROPERTIES`) and never prints them. The release
  workflow passes the repo secrets `SLOT_SS_DEVID` and `SLOT_SS_DEVPASSWORD` (they must be added
  under Settings → Secrets). `devid=` / `devpassword=` in `Config/screenscraper.txt` also work,
  for testing. A Slot built without one says so on the Cart Art screen.
- **The player's own ScreenScraper login.** Every Slot shares one developer id, whose quota is
  small, so nothing is looked up without the player's account (free at screenscraper.fr).

`cartart=off` in `Config/screenscraper.txt` switches it off.

## Logging in on the device

HOME → Cart Art (needs Wi-Fi). The first row is **SCREENSCRAPER**, showing the account or NOT
LOGGED IN; A opens its screen:

- Not logged in: **USERNAME** and **PASSWORD** (on-screen keyboard; the password is shown as
  stars), then **TEST LOGIN AND SAVE**. That calls `ssuserInfos.php`; on success it saves
  `user=` and `password=` to `Config/screenscraper.txt` (other lines kept) and shows the account,
  its level, threads and the day's lookups ("Logged in as jeff, level 1, 1 thread. 12 of 20000
  lookups used today"). A refused login is reported and nothing is saved.
- Logged in: **LOGGED IN AS**, **TEST LOGIN** (checks the saved login and shows the quota again)
  and **SIGN OUT** (removes `user=` and `password=`).

**GET CART ART** without a login opens the login screen ("Enter your ScreenScraper username and
password") instead of running. A refused login during a run stops it and says to check the
login under HOME → Cart Art. The password is never logged; the report (`Logs/cart-art.txt`)
names only the user.

The file can still be written by hand:

    user=<ScreenScraper user name>
    password=<ScreenScraper password>

## Fetching

`crates/slot/src/cart_art_ss.rs` does the fetching: one `jeuInfos` lookup per game (ROM name,
CRC for ROMs up to 64 MB, size), then only the pictures it needs, in tier order, each asked for
at no more than 1000 px. Calls are 1.2 s apart (the developer quota is one thread), only
`*.screenscraper.fr` is contacted, every credential is stripped from anything logged
(`redact`), and a quota or auth error (401/403/423/426/429/430/431) stops ScreenScraper for the
rest of the run. A game ScreenScraper has nothing usable for keeps its file-name label.

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
