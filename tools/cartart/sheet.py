#!/usr/bin/env python3
"""Contact sheets for the cart-art test: per system and one overall (test tooling).

    python3 tools/cartart/sheet.py ROOT

Each row: the ScreenScraper pictures (texture, photo, box, logo) on a checkerboard so
transparency shows, then the fitted label, then the cart as the shelf draws it with that
label, then the cart as it is today with no art (the title plate). Carts are drawn at their
real shelf size (1:1 pixels).
"""
import csv, html, pathlib, sys
from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(sys.argv[1]).expanduser()
KINDS = [("support-texture", "texture"), ("support-2D", "photo"), ("box-2D", "box"), ("wheel-hd", "logo"), ("wheel", "logo"), ("marquee", "marquee"), ("screenmarquee", "marquee")]
THUMB = 200
TIER_H = 200
TIERS = ['texture', 'photo', 'box', 'logo', 'marquee']
BG = (34, 36, 42)
try:
    FONT = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 15)
    SMALL = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", 12)
    BIG = ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial Bold.ttf", 22)
except Exception:
    FONT = SMALL = BIG = ImageFont.load_default()


def checker(size):
    im = Image.new("RGB", size, (90, 90, 90))
    d = ImageDraw.Draw(im)
    for y in range(0, size[1], 10):
        for x in range(0, size[0], 10):
            if (x // 10 + y // 10) % 2:
                d.rectangle([x, y, x + 9, y + 9], fill=(120, 120, 120))
    return im


def thumb(path, box):
    im = Image.open(path).convert("RGBA")
    im.thumbnail(box, Image.LANCZOS)
    bg = checker(im.size)
    bg.paste(im, (0, 0), im)
    return bg


def load_results():
    rows = {}
    for name in ("results.tsv", "results-partial.tsv"):
        f = ROOT / name
        if f.exists():
            for r in csv.DictReader(f.open(), delimiter="\t"):
                rows[(r["system"], r["game"])] = r
    return rows


def system_sheet(sysdir, results):
    raw = ROOT / sysdir / "raw"
    fitted = ROOT / sysdir / "fitted"
    games = sorted(p for p in raw.iterdir() if p.is_dir()) if raw.is_dir() else []
    if not games:
        return None
    rows = []
    for g in games:
        cart = fitted / f"{g.name}.cart.png"
        plate = fitted / f"{g.name}.plate.png"
        label = fitted / f"{g.name}.label.png"
        cart_h = Image.open(cart).height if cart.exists() else 0
        rows.append((g, cart, plate, label, max(THUMB, cart_h) + 46 + TIER_H))
    raw_cols = 4
    cart_w = max([Image.open(r[1]).width for r in rows if r[1].exists()] + [360])
    label_w = 260
    width = 16 + raw_cols * (THUMB + 10) + label_w + 20 + (cart_w + 20) * 2
    height = 50 + sum(r[4] for r in rows)
    sheet = Image.new("RGB", (width, height), BG)
    d = ImageDraw.Draw(sheet)
    d.text((16, 12), f"{sysdir} — ScreenScraper art → fitted label → on the shelf's cart (1:1)   |   right: today, no art", font=BIG, fill=(240, 240, 240))
    y = 50
    for g, cart, plate, label, rh in rows:
        r = results.get((sysdir, g.name), {})
        d.text((16, y + 2), f"{g.name}", font=FONT, fill=(255, 220, 120))
        tier = r.get("tier", "?")
        d.text((16, y + 20), f"tier: {tier}   {r.get('how', '')}   fit {r.get('fit_ms', '?')} ms   label {int(r.get('label_bytes') or 0) // 1024} KB", font=SMALL, fill=(180, 200, 255))
        x = 16
        shown = set()
        col = 0
        for media, nm in KINDS:
            if col >= raw_cols or nm in shown:
                continue
            f = next(iter(g.glob(media + ".*")), None)
            if not f:
                continue
            shown.add(nm)
            t = thumb(f, (THUMB, THUMB))
            sheet.paste(t, (x, y + 40))
            colr = (120, 255, 140) if nm == tier else (170, 170, 170)
            d.text((x, y + 40 + t.height + 2), f"{nm} {Image.open(f).size[0]}×{Image.open(f).size[1]}", font=SMALL, fill=colr)
            x += THUMB + 10
            col += 1
        x = 16 + raw_cols * (THUMB + 10)
        if label.exists():
            t = thumb(label, (label_w, THUMB))
            sheet.paste(t, (x, y + 40))
            d.text((x, y + 40 + t.height + 2), "fitted label", font=SMALL, fill=(170, 170, 170))
        x += label_w + 20
        for f, cap in ((cart, "on the cart"), (plate, "today (no art)")):
            if f.exists():
                im = Image.open(f).convert("RGBA")
                bg = Image.new("RGBA", im.size, BG + (255,))
                bg.alpha_composite(im)
                sheet.paste(bg.convert("RGB"), (x, y + 36))
                d.text((x, y + 36 + im.height + 2), cap, font=SMALL, fill=(170, 170, 170))
            x += cart_w + 20
        # The fallback chain: each tier on its own, half size.
        ty = y + rh - TIER_H + 4
        d.text((16, ty), "each tier alone (½ size):", font=SMALL, fill=(150, 150, 150))
        x = 190
        for t in TIERS:
            f = fitted / f"{g.name}.tier-{t}.png"
            if not f.exists():
                continue
            im = Image.open(f).convert("RGBA")
            im = im.resize((im.width // 2, im.height // 2), Image.LANCZOS)
            bg = Image.new("RGBA", im.size, BG + (255,))
            bg.alpha_composite(im)
            sheet.paste(bg.convert("RGB"), (x, ty))
            colr = (120, 255, 140) if t == tier else (170, 170, 170)
            d.text((x, ty + im.height + 2), t + (" ← chosen" if t == tier else ""), font=SMALL, fill=colr)
            x += max(im.width, 120) + 24
        y += rh
        d.line([(10, y - 4), (width - 10, y - 4)], fill=(60, 62, 70))
    out = ROOT / "sheets"
    out.mkdir(exist_ok=True)
    path = out / f"{sysdir}.png"
    sheet.save(path, optimize=True)
    return path


def overall(paths, results):
    """Every system's carts in one grid: the fitted cart for each game."""
    systems = [p.stem for p in paths]
    cell_w, cell_h = 380, 380
    cols = 6
    entries = []
    for s in systems:
        for g in sorted((ROOT / s / "fitted").glob("*.cart.png")):
            entries.append((s, g))
    rows = (len(entries) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * cell_w, rows * cell_h + 50), BG)
    d = ImageDraw.Draw(sheet)
    d.text((16, 12), "Slotted cart art — every system (fitted from ScreenScraper, drawn on the shelf's carts)", font=BIG, fill=(240, 240, 240))
    for i, (s, g) in enumerate(entries):
        x, y = (i % cols) * cell_w, 50 + (i // cols) * cell_h
        im = Image.open(g).convert("RGBA")
        im.thumbnail((cell_w - 20, cell_h - 50), Image.LANCZOS)
        bg = Image.new("RGBA", im.size, BG + (255,))
        bg.alpha_composite(im)
        sheet.paste(bg.convert("RGB"), (x + (cell_w - im.width) // 2, y + 6))
        game = g.name[: -len(".cart.png")]
        r = results.get((s, game), {})
        d.text((x + 10, y + cell_h - 40), f"{s}: {game[:40]}", font=SMALL, fill=(255, 220, 120))
        d.text((x + 10, y + cell_h - 24), f"{r.get('tier', '?')} · {r.get('fit_ms', '?')} ms", font=SMALL, fill=(180, 200, 255))
    path = ROOT / "sheets" / "ALL.png"
    sheet.save(path, optimize=True)
    return path


def page(paths, results):
    h = ["<!doctype html><meta charset=utf-8><title>Slotted cart art test</title>",
         "<style>body{background:#22242a;color:#ddd;font:14px -apple-system,sans-serif;margin:24px}img{max-width:100%;image-rendering:auto}table{border-collapse:collapse}td,th{padding:3px 8px;border-bottom:1px solid #444;text-align:left}</style>",
         "<h1>Slotted cart art test</h1><p>ScreenScraper art fitted on the Mac by <code>slot-cartart</code>, drawn on the real shelf carts.</p>",
         "<table><tr><th>System</th><th>Game</th><th>Tier</th><th>How</th><th>Fit ms</th><th>Label KB</th></tr>"]
    for (s, g), r in sorted(results.items()):
        h.append(f"<tr><td>{s}</td><td>{html.escape(g)}</td><td>{r['tier']}</td><td>{html.escape(r['how'])}</td><td>{r['fit_ms']}</td><td>{int(r['label_bytes'] or 0)//1024}</td></tr>")
    h.append("</table><h2>All systems</h2><img src='sheets/ALL.png'>")
    for p in paths:
        h.append(f"<h2 id='{p.stem}'>{p.stem}</h2><img src='sheets/{p.name}'>")
    (ROOT / "index.html").write_text("\n".join(h))


if __name__ == "__main__":
    results = load_results()
    systems = ["GBA", "GB", "GBC", "NES", "SNES", "PS1", "MD", "SMS", "GG", "ARCADE", "PCE", "A2600", "INTV", "LYNX", "A7800", "GW", "COLECO", "A5200", "A800", "VECTREX", "32X", "N64", "NDS", "PSP"]
    only = sys.argv[2].split(",") if len(sys.argv) > 2 else systems
    paths = [p for p in (system_sheet(s, results) for s in only) if p]
    if len(sys.argv) <= 2:
        overall(paths, results)
        page(paths, results)
    print("\n".join(str(p) for p in paths))
