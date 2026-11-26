"""Fit a label picture to its cart's label window, ahead of time (Pillow).

The shelf cover-fits whatever it is given into the window (docs/cart-shells.md), cutting
from the middle. Done here instead, the cut can be chosen: a cart scan gives just its
label, a picture is cut where the detail is (a box cover from the top, where its title
is), and a logo is set whole on a field of the art's colour. A picture far from the
window's shape is passed over for one nearer it, so nothing is squeezed or cut in half.

    python3 brick/label_fit.py CARD picture.png out.png   # one picture, for a look
"""
import io
import math
import pathlib
import re
import sys

from PIL import Image, ImageChops, ImageFilter, ImageStat

CARTS = pathlib.Path(__file__).resolve().parent.parent / "assets" / "carts"
SHELLS = {"MD": "md", "SMS": "sms", "GG": "gg", "ARCADE": "arcade", "PCE": "pce",
          "A2600": "a2600", "INTV": "intv", "LYNX": "lynx", "A7800": "a7800", "COLECO": "coleco",
          "A5200": "a5200", "A800": "a800", "VECTREX": "vectrex", "32X": "s32x", "N64": "n64",
          "NDS": "nds", "PSP": "psp"}

# ScreenScraper's cart scans (ES-DE physicalmedia) are drawn alike for a system: where the
# label sits in one (fractions of the picture), the picture's shape, and how much taller
# the label is than it looks (the Intellivision's sloped end label, photographed from above).
PHYSICAL = {
    "MD": ((0.158, 0.02, 0.842, 0.848), 1.47, 1.0),
    "GG": ((0.10, 0.18, 0.90, 0.78), 0.99, 1.0),
    "PCE": ((0.06, 0.32, 0.94, 0.94), 0.79, 1.0),
    "A2600": ((0.07, 0.08, 0.93, 0.93), 0.84, 1.0),
    "A7800": ((0.07, 0.04, 0.93, 0.93), 0.84, 1.0),
    "INTV": ((0.07, 0.775, 0.93, 0.915), 0.81, math.sqrt(2)),
    "LYNX": ((0.05, 0.04, 0.96, 0.84), 0.96, 1.0),
    "COLECO": ((0.06, 0.02, 0.94, 0.66), 0.87, 1.0),
    "A5200": ((0.12, 0.04, 0.88, 0.78), 1.09, 1.0),
    "A800": ((0.05, 0.02, 0.95, 0.30), 0.99, 1.0),
    "32X": ((0.18, 0.03, 0.82, 0.74), 1.52, 1.0),
    "N64": ((0.31, 0.14, 0.71, 0.95), 1.55, 1.0),
    "NDS": ((0.07, 0.12, 0.93, 0.86), 0.95, 1.0),
}
# How far a picture's shape may be from the window's and still be cut to it.
NEAR = math.log(1.3)
SCALE = 2  # labels are kept at twice the 360 px shell's window, for the bigger views


def window(card):
    """(w, h) of the label window in the shell's meta.toml."""
    text = (CARTS / SHELLS[card] / "meta.toml").read_text()
    val = {k: int(v) for k, v in re.findall(r"^\s*([wh])\s*=\s*(\d+)", text, re.M)}
    return val["w"], val["h"]


def load(data):
    im = Image.open(io.BytesIO(data))
    im.load()
    return im.convert("RGBA")


def trim(im):
    """Without transparent or flat borders."""
    box = im.getchannel("A").point(lambda a: 255 if a > 16 else 0).getbbox()
    if box:
        im = im.crop(box)
    rgb = im.convert("RGB")
    flat = Image.new("RGB", rgb.size, rgb.getpixel((0, 0)))
    box = ImageChops.difference(rgb, flat).convert("L").point(lambda v: 255 if v > 24 else 0).getbbox()
    if box and (box[2] - box[0]) * (box[3] - box[1]) > 0.5 * rgb.width * rgb.height:
        im = im.crop(box)
    return im


def physical_label(card, im):
    """The label off a cart scan, or None when the scan is not that system's cart."""
    spec = PHYSICAL.get(card)
    if not spec:
        return None
    (x0, y0, x1, y1), shape, taller = spec
    if abs(math.log(im.width / im.height / shape)) > 0.08:
        return None  # a CD, a box, another region's cart
    w, h = im.size
    label = im.crop((round(x0 * w), round(y0 * h), round(x1 * w), round(y1 * h)))
    if taller != 1.0:
        label = label.resize((label.width, round(label.height * taller)), Image.LANCZOS)
    return label


def flatten(im, colour=(0, 0, 0)):
    bg = Image.new("RGBA", im.size, colour + (255,))
    bg.alpha_composite(im)
    return bg.convert("RGB")


def smart_box(im, aspect, top):
    """The window-shaped cut with the most detail, a little in favour of the middle (or the top)."""
    w, h = im.size
    if w / h > aspect:
        cw, ch = round(h * aspect), h
    else:
        cw, ch = w, round(w / aspect)
    if (cw, ch) == (w, h):
        return (0, 0, w, h)
    small = im.convert("L")
    k = 160 / max(w, h)
    small = small.resize((max(1, round(w * k)), max(1, round(h * k))))
    edges = small.filter(ImageFilter.FIND_EDGES)
    span = (w - cw) if cw < w else (h - ch)
    best, best_score = 0, -1.0
    steps = 24
    for i in range(steps + 1):
        off = round(span * i / steps)
        box = (off, 0, off + cw, ch) if cw < w else (0, off, cw, off + ch)
        sbox = tuple(round(v * k) for v in box)
        if sbox[2] <= sbox[0] or sbox[3] <= sbox[1]:
            continue
        energy = ImageStat.Stat(edges.crop(sbox)).mean[0]
        u = i / steps
        prior = 1.0 - 0.3 * u if (top and cw == w) else 1.0 - 0.25 * abs(u - 0.5) * 2
        if energy * prior > best_score:
            best, best_score = off, energy * prior
    return (best, 0, best + cw, ch) if cw < w else (0, best, cw, best + ch)


def cover(im, size, top=False, field=(0, 0, 0)):
    im = flatten(im, field)
    box = smart_box(im, size[0] / size[1], top)
    return im.crop(box).resize(size, Image.LANCZOS)


def logo(im, size, field):
    """A logo whole, on the art's colour, as on a title-only end label."""
    im = trim(im)
    out = Image.new("RGBA", size, field + (255,))
    room = (size[0] * 0.88, size[1] * 0.78)
    k = min(room[0] / im.width, room[1] / im.height)
    im = im.resize((max(1, round(im.width * k)), max(1, round(im.height * k))), Image.LANCZOS)
    out.alpha_composite(im, ((size[0] - im.width) // 2, (size[1] - im.height) // 2))
    return out.convert("RGB")


def field_of(pictures):
    """A dark, saturated colour from the art (the cover), for a logo to sit on."""
    for im in pictures:
        rgb = flatten(im).resize((24, 24)).convert("HSV")
        h, s, v = [ImageStat.Stat(c).median[0] for c in rgb.split()]
        return Image.new("HSV", (1, 1), (int(h), max(int(s), 150), min(int(v), 120))).convert("RGB").getpixel((0, 0))
    return (24, 24, 30)


def border_colour(im):
    """The colour round a picture's edge, or None when the edge is busy."""
    rgb = flatten(im)
    w, h = rgb.size
    strips = [rgb.crop((0, 0, w, 2)), rgb.crop((0, h - 2, w, h)), rgb.crop((0, 0, 2, h)), rgb.crop((w - 2, 0, w, h))]
    px = [p for st in strips for p in st.getdata()]
    mean = [sum(c[i] for c in px) / len(px) for i in range(3)]
    spread = sum(abs(c[i] - mean[i]) for c in px for i in range(3)) / len(px) / 3
    return tuple(round(v) for v in mean) if spread < 18 else None


def extend(im, size, field):
    """The whole picture, its plain edge carried out to the window (a marquee on black)."""
    out = Image.new("RGB", size, field)
    k = min(size[0] / im.width, size[1] / im.height)
    pic = flatten(im, field).resize((max(1, round(im.width * k)), max(1, round(im.height * k))), Image.LANCZOS)
    out.paste(pic, ((size[0] - pic.width) // 2, (size[1] - pic.height) // 2))
    return out


def backdrop(im, size):
    """The whole picture over a soft, dim blow-up of itself: a far-off shape kept whole."""
    from PIL import ImageEnhance
    back = cover(im, size).filter(ImageFilter.GaussianBlur(max(size) / 40))
    back = ImageEnhance.Brightness(back).enhance(0.55)
    k = min(size[0] / im.width, size[1] / im.height)
    pic = flatten(im).resize((max(1, round(im.width * k)), max(1, round(im.height * k))), Image.LANCZOS)
    back.paste(pic, ((size[0] - pic.width) // 2, (size[1] - pic.height) // 2))
    return back


def far(card, im):
    """Too far from the window's shape to cut well."""
    w, h = window(card)
    return abs(math.log(im.width / im.height / (w / h))) > NEAR


def is_logo(im):
    """Mostly see-through: a wheel logo, not a picture."""
    alpha = im.getchannel("A")
    return ImageStat.Stat(alpha).mean[0] < 200


def choose(card, candidates):
    """(png bytes, how) for the best of (kind, bytes) candidates, in the system's order."""
    w, h = window(card)
    size = (w * SCALE, h * SCALE)
    aspect = w / h
    pictures, logos = [], []
    for kind, data in candidates:
        try:
            im = load(data)
        except Exception:
            continue
        if kind == "physicalmedia":
            label = physical_label(card, im)
            if label is not None:
                pictures.append((kind, label))
            continue
        if is_logo(im) and kind == "marquees":
            logos.append((kind, im))
        else:
            pictures.append((kind, trim(im) if kind == "marquees" else im))
    if not pictures and not logos:
        return None, None
    def miss(im):
        return abs(math.log(im.width / im.height / aspect))
    near = [(k, im) for k, im in pictures if miss(im) <= NEAR]
    if near:
        kind, im = near[0]
        out, how = cover(im, size, top=(kind == "covers")), f"{kind} cut"
    elif any(border_colour(im) for _, im in pictures):
        kind, im = next((k, im) for k, im in pictures if border_colour(im))
        out, how = extend(im, size, border_colour(im)), f"{kind} whole"
    elif logos:
        field = (0, 0, 0) if card == "ARCADE" else field_of([im for _, im in pictures])
        out, how = logo(logos[0][1], size, field), f"{logos[0][0]} logo"
    else:
        kind, im = min(pictures, key=lambda p: miss(p[1]))
        out, how = backdrop(im, size), f"{kind} whole on its own backdrop"
    buf = io.BytesIO()
    out.save(buf, "PNG", optimize=True)
    return buf.getvalue(), how


if __name__ == "__main__":
    card, src, dst = sys.argv[1:4]
    kind = sys.argv[4] if len(sys.argv) > 4 else "covers"
    data, how = choose(card, [(kind, pathlib.Path(src).read_bytes())])
    pathlib.Path(dst).write_bytes(data)
    print(how)
