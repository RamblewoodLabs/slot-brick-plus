import sys, pathlib
from PIL import Image, ImageDraw
root = pathlib.Path(sys.argv[1]); out = sys.argv[2]
kinds = sys.argv[3].split(",")
rows = sorted(p for p in root.iterdir() if p.is_dir())
cell = 300
sheet = Image.new("RGB", (cell*len(kinds), (cell+16)*len(rows)), (60,60,60))
d = ImageDraw.Draw(sheet)
for r, g in enumerate(rows):
    for c, k in enumerate(kinds):
        f = next(iter(g.glob(k+".*")), None)
        d.text((c*cell+4, r*(cell+16)+2), f"{g.name[:30]} {k}", fill=(255,255,0))
        if not f: continue
        im = Image.open(f).convert("RGBA"); w,h = im.size
        im.thumbnail((cell-8, cell-8))
        bg = Image.new("RGBA", im.size, (255,0,255,255)); bg.alpha_composite(im)
        sheet.paste(bg.convert("RGB"), (c*cell+4, r*(cell+16)+16))
        d.text((c*cell+4, r*(cell+16)+cell+2), f"{w}x{h}", fill=(200,200,200))
sheet.save(out)
