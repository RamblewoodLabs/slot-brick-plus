#!/usr/bin/env python3
"""Contact sheet for preloaded Labels/<DIR>/<stem>.png trees.

    python3 tools/cartart/labels_sheet.py LABELS_DIR OUT_DIR
"""
import html
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

THUMB = 160
COLS = 8
BG = (34, 36, 42)
FG = (240, 240, 240)
CAP = (255, 220, 120)


def font(size, bold=False):
    name = "Arial Bold.ttf" if bold else "Arial.ttf"
    try:
        return ImageFont.truetype(f"/System/Library/Fonts/Supplemental/{name}", size)
    except Exception:
        return ImageFont.load_default()


def sheet_for(sysdir: Path, out: Path):
    files = sorted(p for p in sysdir.glob("*.png") if p.is_file())
    famicom = sorted(p for p in (sysdir / "Famicom").glob("*.png") if p.is_file()) if (sysdir / "Famicom").is_dir() else []
    files = files + famicom
    if not files:
        return None
    rows = (len(files) + COLS - 1) // COLS
    cell_w, cell_h = THUMB + 12, THUMB + 36
    im = Image.new("RGB", (16 + COLS * cell_w, 50 + rows * cell_h), BG)
    d = ImageDraw.Draw(im)
    d.text((16, 12), f"{sysdir.name} — {len(files)} labels (1×)", font=font(22, True), fill=FG)
    big = font(12)
    for i, p in enumerate(files):
        r, c = divmod(i, COLS)
        x, y = 16 + c * cell_w, 50 + r * cell_h
        try:
            src = Image.open(p).convert("RGBA")
        except Exception:
            continue
        src.thumbnail((THUMB, THUMB), Image.LANCZOS)
        bg = Image.new("RGB", (THUMB, THUMB), (50, 52, 60))
        ox, oy = (THUMB - src.width) // 2, (THUMB - src.height) // 2
        bg.paste(src.convert("RGB"), (ox, oy), src)
        im.paste(bg, (x, y))
        d.text((x, y + THUMB + 2), p.stem[:28], font=big, fill=CAP)
    dest = out / f"{sysdir.name}.png"
    im.save(dest)
    return dest, files


def main():
    if len(sys.argv) < 3:
        print("usage: labels_sheet.py LABELS_DIR OUT_DIR", file=sys.stderr)
        return 2
    labels = Path(sys.argv[1]).expanduser()
    out = Path(sys.argv[2]).expanduser()
    out.mkdir(parents=True, exist_ok=True)
    systems = sorted(p for p in labels.iterdir() if p.is_dir())
    parts = []
    for s in systems:
        got = sheet_for(s, out)
        if not got:
            continue
        dest, files = got
        imgs = "\n".join(
            f'<div class="cell"><img src="../Slot/Labels/{html.escape(s.name)}/{html.escape(p.name)}" alt=""><div>{html.escape(p.stem)}</div></div>'
            if p.parent.name != "Famicom"
            else f'<div class="cell"><img src="../Slot/Labels/{html.escape(s.name)}/Famicom/{html.escape(p.name)}" alt=""><div>{html.escape(p.stem)} (Famicom)</div></div>'
            for p in files
        )
        parts.append(f"<h2>{html.escape(s.name)} ({len(files)})</h2>\n<p><a href=\"{html.escape(dest.name)}\">PNG sheet</a></p>\n<div class=\"grid\">{imgs}</div>")
    index = out / "index.html"
    index.write_text(
        """<!doctype html><meta charset="utf-8"><title>RG280V preloaded labels</title>
<style>
body{font:14px/1.4 -apple-system,sans-serif;background:#222;color:#eee;margin:24px}
h1,h2{font-weight:600}
.grid{display:flex;flex-wrap:wrap;gap:12px}
.cell{width:160px}
.cell img{width:160px;height:160px;object-fit:contain;background:#333}
.cell div{font-size:11px;color:#fdc;margin-top:4px}
</style>
<h1>RG280V preloaded labels (1×)</h1>
"""
        + "\n".join(parts)
        + "\n"
    )
    print(f"wrote {index}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
