# Shelf UI

## Moving around (on main)

| Input | Action |
| :--- | :--- |
| Left / Right | Browse the shelf |
| Up / Down | Jump to the previous or next letter (Arcade uses the real title, in PR #28) |
| L1 / R1 | Change shelf |
| Tap A | Resume from the last save state. A multi-disc PS1 game fans its discs out first. |
| Hold A | Start the game fresh |
| START | Customise the cart: colour, finish, shape and label (on GBA, also the core) |
| Y | Star or unstar a favorite |
| SELECT | Change the sort order (and, with PR #29, the filter) |
| HOME | Menu (see [Settings Reference](Settings-Reference.md)) |

When you pick a game, a title card under its cart names it (**Library → Title Card**). Your play time shows beside
the battery (**Library → Play Time**).

## Carts and inserting

- **On main:** the first six systems (GB, GBC, GBA, NES/Famicom, SNES US/PAL, PlayStation disc) are drawn in code.
  Every later system is a PNG shell rendered in Blender, with a label window set in `assets/carts/<sys>/meta.toml`
  (see `docs/cart-shells.md`). START changes a cart's colour, finish (solid, clear, glitter) and shape. Choices are
  saved in `Slot/Labels/cart_shell.ini`.
- **In PR #27:** a TurboGrafx-16 HuCard turns end over end on the way in, so its contacts enter the slot first.
- **In PR #31:** each new shelf has its own cart: the ColecoVision, 5200 and 32X carts, a 5¼" floppy for the Atari 800,
  the DS card, the N64 Game Pak and a PSP UMD. Each turns until its contacts point down, then goes in contacts first.
  A UMD spins its disc behind the clear window, turns over, settles, then slides in curved end first.
- **In PR #32 (3D carts and insert pace):**
  - Every cart turns at the same unhurried pace, about **2.6 s**. The turn eases in and out, then the cart rests a
    moment before it drops into the slot.
  - GB, GBC, GBA, NES and SNES carts now turn too, and a PlayStation disc spins as it turns over.
  - The GB, GBC, GBA, NES, Famicom, SNES (US and PAL) carts and the PS1 disc are real 3D models (Blender), with
    moulded lettering, label wells, screws, stickers, backs and connector ends. Each keeps the shell colour you chose.
    The other carts gained backs, bottoms and contacts.
  - The Genesis cart has real depth, and the N64 cart is the real rectangular Game Pak.

## Game & Watch

On main, each handheld on the shelf is `Labels/GW/<game>.png`, and `Labels/GW/open/<game>.png` holds 18 frames of it
opening (a clamshell) or lighting up. Picking the game plays the frames, and ejecting plays them backwards. The frames
are rendered with `brick/gw-art` (Blender) from **your own** ES-DE media and aren't in git.

- **In PR #27:** the animation lasts about 1.6× as long (0.96 s) and eases in and out.
- **In PR #32:** it plays over the same ~2.6 s as a cart insert.

## Vectrex power-on (in PR #32)

The Vectrex shelf shows the console itself, switched off, with the game's art dim on its screen. Pick a game and the
console turns round, its screen lights up, and it goes in lit.

## Sorting, filters and favorites

- **On main:** SELECT steps through **A–Z, Last Played, Most Played**. **HOME → Library → Sort By** sets every shelf
  at once. **Library → Show: Favorites** leaves only starred games (or all games, if you have no favorites).
- **On main (PR #26):** a **Favorites shelf** gathers starred games from every system in one row, each in its own cart
  or handheld. On that shelf, SELECT is **Added** or **Last Played**. Library → Systems has Favorites as its top row,
  so you can hide every other shelf.
- **In PR #29 (shelf filters):** on a system shelf, SELECT steps through A–Z, Last Played, Most Played, then:

  | Filter | What it shows |
  | :--- | :--- |
  | **Top 100** | Best rated by players (LaunchBox community rating) |
  | **Popular** | Most rated (LaunchBox rating count) |
  | **Best Selling** | Games on that system's best-selling list (Wikipedia) |
  | **First Party** | Published by Nintendo, Sega, Sony, NEC/Hudson, Atari or Mattel |

  A filter that would show nothing on a shelf is skipped. Each shelf remembers its own setting, and a pill in the top
  corner names it. In the ranked filters the title card numbers each game (#1, #2, …), and a game that's on the card
  more than once (regional versions, arcade clones) shows only once. On the Favorites shelf, SELECT is Added, Last
  Played or Most Played. The rankings are built into Slot (`crates/slot-store/rankings/*.tsv`), so nothing is looked
  up online. Coverage of each system is in `crates/slot-store/rankings/COVERAGE.md`. Best Selling has no data for SMS,
  GG, Arcade, PCE, INTV, Lynx, 7800 or G&W.
- **Recently played:** "Last Played" is the sort order. PR #27 also caches achievement sets for the 20 most recently
  played games. A separate "Recent games" shelf was **not found** in main or any open branch.

Play time, last-played times and favorites are kept in `Slot/Config/library.txt`.
