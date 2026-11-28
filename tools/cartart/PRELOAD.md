# RG280V label preload (host)

A Mac tool that walks a ROM *manifest* (so it can run without the Brick card present), looks
each game up on ScreenScraper, fits a label with `slot-cartart`, and writes

`/Users/jeff/Shared-Space/rg280v/Slot/Labels/<DIR>/<stem>.png`

at **1×** the label window. N64, DS and PSP are skipped unless `--include-hidden` is passed.
Reruns skip labels that are already on disk. Credentials are never printed or written into
the report.

## Manifest

A tab-separated file: system dir, ROM filename. Extra columns are ignored. A `famicom` column
sends a NES label to `Labels/NES/Famicom/`. Lines starting with `#` are comments.

    GB	Tetris (World) (Rev 1).gb
    NES	Super Mario Bros. (World).nes
    SNES	Super Mario World (USA).sfc

## Login files

- Developer id: gitignored `screenscraper.properties` (repo root or `android/`), or
  `SLOT_SS_PROPERTIES` / `SLOT_SS_DEVID` + `SLOT_SS_DEVPASSWORD`. Never print these.
- User login (for a full run): gitignored `screenscraper-user.properties` in this worktree
  (`--user-login` or `SLOT_SS_USER_PROPERTIES`). Mode 600. Fill in exactly:

      user=
      password=

  A full library run needs Jeff's own ScreenScraper account. `--allow-dev-only` is for a
  small test only.

## Check the user login

Never prints the username or password:

    cargo run --release -p slot --bin rg280v-preload -- \
      --test-login \
      --dev-properties /Users/jeff/sbp-cartart/android/screenscraper.properties \
      --user-login /Users/jeff/sbp-rg280v-preload/screenscraper-user.properties

## Run

    cargo run --release -p slot --bin rg280v-preload -- \
      --manifest tools/cartart/preload-smoke.tsv \
      --out /Users/jeff/Shared-Space/rg280v/Slot \
      --scale 1

    # no network:
    cargo run --release -p slot --bin rg280v-preload -- \
      --manifest PATH --dry-run

After a fetch, build a contact sheet:

    python3 tools/cartart/labels_sheet.py \
      /Users/jeff/Shared-Space/rg280v/Slot/Labels \
      /Users/jeff/Shared-Space/rg280v/sheets

Reports land next to the Slot folder: `preload-report.tsv` and `preload-misses.tsv`.
