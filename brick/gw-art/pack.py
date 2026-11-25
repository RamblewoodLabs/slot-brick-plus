#!/usr/bin/env python3
"""Pack one game's Blender frames (render.py) into what Slot reads from the card.

    python3 brick/gw-art/pack.py FRAMES_DIR KIND OUT_LABELS STEM

FRAMES_DIR holds 00.png, 01.png (one screen: unlit, lit) or 00..17.png (a clamshell
opening, its screens lighting at the end). Writes, under OUT_LABELS (Slot/Labels/GW):

    <STEM>.png        the shelf face: closed, or the screen unlit. 360x331 RGBA.
    open/<STEM>.png   18 frames of that size side by side, played when the game is picked.

Each state is fitted to the 360x331 box on its own (a clamshell is small closed and as big
as fits open, growing as it opens), centred across and standing on the box's bottom edge.
"""
import pathlib
import sys

from PIL import Image

BOX = (360, 331)
FRAMES = 18


def smooth(u):
    u = min(1.0, max(0.0, u))
    return u * u * (3 - 2 * u)


def fit(size):
    return min(BOX[0] / size[0], BOX[1] / size[1])


def place(img, scale):
    w, h = max(1, round(img.width * scale)), max(1, round(img.height * scale))
    w, h = min(w, BOX[0]), min(h, BOX[1])
    out = Image.new("RGBA", BOX, (0, 0, 0, 0))
    out.paste(img.resize((w, h), Image.Resampling.LANCZOS), ((BOX[0] - w) // 2, BOX[1] - h))
    return out


def cropped(path):
    img = Image.open(path).convert("RGBA")
    box = img.getchannel("A").point(lambda a: 255 if a > 8 else 0).getbbox()
    return img.crop(box) if box else img


def frames(src, kind):
    if kind == "single":
        blank, lit = cropped(src / "00.png"), cropped(src / "01.png")
        if lit.size != blank.size:
            lit = lit.resize(blank.size, Image.Resampling.LANCZOS)
        s = fit(blank.size)
        a, b = place(blank, s), place(lit, s)
        # Three frames unlit, the screen coming up over twelve, three lit.
        return [Image.blend(a, b, smooth((i - 3) / 12)) for i in range(FRAMES)]
    shots = [cropped(src / f"{i:02}.png") for i in range(FRAMES)]
    closed, opened = fit(shots[0].size), fit(shots[-1].size)
    out = []
    for i, img in enumerate(shots):
        # The same easing render.py opens the lid with.
        u = smooth(min(1.0, (i / (FRAMES - 1)) / 0.78))
        out.append(place(img, min(closed + (opened - closed) * u, fit(img.size))))
    return out


def main(argv):
    src, kind, labels, stem = pathlib.Path(argv[0]), argv[1], pathlib.Path(argv[2]), argv[3]
    seq = frames(src, kind)
    (labels / "open").mkdir(parents=True, exist_ok=True)
    seq[0].save(labels / f"{stem}.png", optimize=True)
    strip = Image.new("RGBA", (BOX[0] * len(seq), BOX[1]))
    for i, f in enumerate(seq):
        strip.paste(f, (i * BOX[0], 0))
    strip.save(labels / "open" / f"{stem}.png", optimize=True)


if __name__ == "__main__":
    main(sys.argv[1:])
