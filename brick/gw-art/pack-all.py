#!/usr/bin/env python3
"""Pack render.py's frames for every game in devices.csv (key gwNN is row NN) as Labels/GW.
    python3 brick/gw-art/pack-all.py FRAMES_DIR LABELS_DIR
"""
import csv
import pathlib
import sys

from pack import main

here = pathlib.Path(__file__).resolve().parent
src, out = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
with open(here / "devices.csv", newline="") as f:
    for i, row in enumerate(csv.DictReader(f)):
        key = src / f"gw{i:02}"
        if not key.is_dir():
            print(f"{row['file']}: no frames in {key}", file=sys.stderr)
            continue
        main([str(key), row["kind"], str(out), pathlib.Path(row["file"]).stem])
        print(row["file"], flush=True)
