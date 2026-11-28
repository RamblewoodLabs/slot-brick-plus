# artlib — private local art library for Slotted

This tool keeps every cartridge label, box cover, and raw ScreenScraper picture
we have pulled or made in one private folder on the Mac mini. When you build a
new device (Brick, RG35XX SP, Android, or RG280V), you can fill its `Labels/`
folder from that library with no Wi-Fi and no ScreenScraper login.

The images themselves are copyrighted. They stay on the Mac mini and are never
committed to git or pushed to GitHub. Only this script (the code) belongs in
the repo.

## Where the library lives

```
/Volumes/External 4TB/AI/ArtLibrary/slotted
```

The External 4TB drive is attached to the Mac mini and has plenty of free
space. The internal disk is nearly full, and the top level of the External 4TB
volume is only writable by an administrator, so the library sits inside Jeff's
existing `AI` folder on that drive. The folder is mode 700: only Jeff's account
can open it.

A short pointer is at `~/ArtLibrary/README.txt`. That folder is *not* a network
share. `Shared-Space` is shared with the other computers, so the library is
kept out of it.

Override the location with `--root` or the `ARTLIB_ROOT` environment variable.

## Commands

From a checkout of this repo, on the Mac mini:

```bash
# Create the folder and database (also happens automatically on first ingest)
python3 tools/artlib/artlib.py init

# Add art. Sources are only copied, never moved or deleted.
python3 tools/artlib/artlib.py ingest /Users/jeff/Shared-Space/cartart-test \
    --source cartart-test

# After an RG280V preload run (safe to repeat; already-known files are skipped)
python3 tools/artlib/artlib.py ingest /Users/jeff/Shared-Space/rg280v/Slot/Labels \
    --source rg280v-preload --kind label --scale 1x

# See what is in the library
python3 tools/artlib/artlib.py stats

# Fill a device's Labels/ folder offline from a ROM list
# Manifest is a TSV: system<TAB>rom-filename
python3 tools/artlib/artlib.py export \
    --manifest /Users/jeff/Shared-Space/rg280v/brick-manifest.tsv \
    --scale 1x \
    --out /path/to/SlotRoot
```

`export` writes `Labels/<SYSTEM>/<game>.png` and a report file
`artlib-export-report.tsv` listing successes and gaps. Pass `--fallback` if you
are willing to use a 2x label when a 1x one is missing (or the other way around).

## What it stores

- Unique files, addressed by SHA-256, under `objects/sha256/…`
- A SQLite manifest (`manifest.sqlite`) with system, game name, type, size,
  hash, source path, date, and ScreenScraper ids when those were in a sidecar
  `meta.json`
- A browsable `index/` tree (labels by 1x/2x, raw media by type, other art)

Duplicate files from later ingest runs take no extra space. Credential files
(`screenscraper*.properties`, passwords, `.env`, and similar) are never copied
or logged.

## After a preload or a ScreenScraper fetch

The RG280V preload tool writes labels under
`/Users/jeff/Shared-Space/rg280v/Slot/Labels/`. Re-run the ingest command above
when a run finishes (or while it is running: already-complete images are
picked up, and a file still being written is skipped and can be added next
time). Wiring the preload tool to write into the library as it goes is a
small follow-up; the ingest command is the supported path today.

## Tests

```bash
python3 tools/artlib/test_artlib.py
```
