#!/usr/bin/env python3
"""Fill Labels/<system>/<stem>.png from ES-DE media, then libretro-thumbnails.

ES-DE's downloaded_media is the first source. A picture already next to the
ROM's name is copied as the label. libretro-thumbnails is the fallback, and
only when --download is passed (it fetches over HTTPS).

    python3 brick/build_sp_art.py --games Slot/Games --out Slot/Labels \
        --media /path/to/downloaded_media [--download]
    python3 brick/build_sp_art.py --self-test

Arcade uses the marquee first (it is the label on the wide, backlit panel): ES-DE
marquees from every arcade folder (arcade, fbneo, mame), then the parent set's
marquee for a clone (--arcade-dat, an FBNeo/MAME XML dat with cloneof), then, with
--download, the arcadeitalia marquee scan of the set or its parent, and only then
covers or a libretro title screen. The Master System shell is a Sega Card, whose
label is box-style art, so SMS takes the cover first. Every other new system uses
physical media, then the cover, then the marquee. The shelf scales whatever it
gets into the shell's label window (docs/cart-shells.md).

Names follow libretro: &, *, /, :, backtick, <, >, ?, backslash, | and double quote become _.
"""
import argparse
import pathlib
import sys
import urllib.parse
import urllib.request

try:  # Pillow: cut each picture to its label window (brick/label_fit.py); without it, copy.
    import label_fit
except ImportError:
    label_fit = None

# Card folder, ES-DE folder names, libretro-thumbnails playlist, kind order.
SYSTEMS = [
    ("MD", ("megadrive", "genesis"), "Sega - Mega Drive - Genesis", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("SMS", ("mastersystem", "sms"), "Sega - Master System - Mark III", ("covers", "physicalmedia", "marquees"), "Named_Boxarts"),
    ("GG", ("gamegear", "gg"), "Sega - Game Gear", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("ARCADE", ("arcade", "fbneo", "mame"), "FBNeo - Arcade Games", ("marquees", "physicalmedia", "covers"), "Named_Titles"),
    ("PCE", ("pcengine", "tg16", "turbografx"), "NEC - PC Engine - TurboGrafx 16", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("A2600", ("atari2600", "a2600"), "Atari - 2600", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("INTV", ("intellivision", "intv"), "Mattel - Intellivision", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("LYNX", ("atarilynx", "lynx"), "Atari - Lynx", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("A7800", ("atari7800", "a7800"), "Atari - 7800", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
]

# thumbnails.libretro.com answers 404 for whole playlists now; the GitHub mirror is complete.
THUMBS = "https://raw.githubusercontent.com/libretro-thumbnails"
MARQUEES = "https://adb.arcadeitalia.net/media/mame.current/marquees"
FORBIDDEN = set("&*/:`<>?\\|\"")


def libretro_name(name):
    return "".join("_" if c in FORBIDDEN else c for c in name)


def index_media(folder):
    """kind -> {lowercase filename: path} for png/jpg in that ES-DE system folder."""
    found = {kind: {} for kind in ("physicalmedia", "covers", "marquees")}
    if folder is None or not folder.is_dir():
        return found
    for kind in found:
        for candidate in (folder / kind, folder / "media" / kind):
            if not candidate.is_dir():
                continue
            for path in candidate.iterdir():
                if path.suffix.lower() in {".png", ".jpg", ".jpeg"}:
                    found[kind][path.name.lower()] = path
    return found


def local_art(stem, order, indexed):
    names = {stem.lower(), libretro_name(stem).lower()}
    for kind in order:
        for name in names:
            for ext in (".png", ".jpg", ".jpeg"):
                hit = indexed[kind].get(name + ext)
                if hit:
                    return hit
    return None


def thumbnail_url(playlist, named, stem):
    # libretro stores the picture under the No-Intro name with those characters swapped,
    # in a repository named after the playlist with spaces as underscores.
    file = urllib.parse.quote(libretro_name(stem) + ".png")
    repo = urllib.parse.quote(playlist.replace(" ", "_"))
    return f"{THUMBS}/{repo}/master/{named}/{file}"


def marquee_url(name):
    return f"{MARQUEES}/{urllib.parse.quote(name)}.png"


def read_dat(path):
    """set name -> (parent or None, description) from a ClrMamePro/MAME XML dat."""
    import xml.etree.ElementTree as ET

    sets = {}
    if path is None:
        return sets
    for _, el in ET.iterparse(str(path)):
        if el.tag in ("game", "machine") and el.get("name"):
            desc = el.findtext("description") or ""
            sets[el.get("name")] = (el.get("cloneof"), desc)
            el.clear()
    return sets


def merged_media(media, aliases):
    """Every alias folder's pictures together: an arcade set may sit under arcade, fbneo or mame."""
    merged = {kind: {} for kind in ("physicalmedia", "covers", "marquees")}
    for alias in reversed(aliases):
        indexed = index_media(media / alias if media else None)
        for kind in merged:
            merged[kind].update(indexed[kind])
    return merged


def fetch(url):
    try:
        with urllib.request.urlopen(url, timeout=30) as reply:
            data = reply.read()
        return data if data[:8] == b"\x89PNG\r\n\x1a\n" else None
    except Exception:
        return None


def poor_marquee(data):
    """Only the game's logo, or far from the marquee window's shape."""
    try:
        im = label_fit.load(data)
        return label_fit.is_logo(im) or label_fit.far("ARCADE", label_fit.trim(im))
    except Exception:
        return False


def fitted(card, candidates):
    """The label for the first usable candidate, cut to the window when Pillow is here."""
    if label_fit:
        try:
            data, how = label_fit.choose(card, candidates)
            if data:
                return data, how
        except Exception as e:
            print(f"label_fit: {e}", file=sys.stderr)
    for kind, data in candidates:
        if data[:2] != b"\xff\xd8":  # as it is, but never a JPEG: the shelf decodes PNG only
            return data, kind
    return None, None


def arcade_art(stem, indexed, sets, download):
    """Marquee first, the set's own then its parent's, local then arcadeitalia."""
    parent, desc = sets.get(stem, (None, ""))
    names = [stem] + ([parent] if parent else [])
    for name in names:
        hit = local_art(name, ("marquees",), indexed)
        if hit:
            data = hit.read_bytes()
            # Some ES-DE marquees are only the game's logo, or cut oddly: the arcadeitalia scan is the real panel.
            if download and label_fit and poor_marquee(data):
                scan = fetch(marquee_url(name))
                if scan:
                    return scan, f"arcadeitalia marquee {name}"
            return data, f"marquee {hit.name}"
    if download:
        for name in names:
            data = fetch(marquee_url(name))
            if data:
                return data, f"arcadeitalia marquee {name}"
    for name in names:
        hit = local_art(name, ("covers", "physicalmedia"), indexed)
        if hit:
            return hit.read_bytes(), f"{hit.parent.name} {hit.name}"
    if download and desc:
        data = fetch(thumbnail_url("FBNeo - Arcade Games", "Named_Titles", desc))
        if data:
            return data, "libretro title"
    return None, None


def each_rom(games):
    for card, aliases, playlist, order, named in SYSTEMS:
        folder = games / card
        if not folder.is_dir():
            continue
        for path in sorted(folder.iterdir()):
            if path.is_file() and not path.name.startswith("."):
                yield card, aliases, playlist, order, named, path


def run(games, out, media, download, arcade_dat=None):
    written = 0
    missing = 0
    sources = {}
    sets = read_dat(arcade_dat)
    cache = {}
    for card, aliases, playlist, order, named, rom in each_rom(games):
        stem = rom.stem
        if card not in cache:
            cache[card] = merged_media(media, aliases)
        indexed = cache[card]
        data = None
        if card == "ARCADE":
            data, how = arcade_art(stem, indexed, sets, download)
            if data:
                kind = "marquees" if "marquee" in how else "covers"
                data, cut = fitted(card, [(kind, data)])
                how = f"{how.split(' ')[0]} {cut}"
        else:
            candidates = []
            for kind in order:
                src = local_art(stem, (kind,), indexed)
                if src and (label_fit or src.suffix.lower() == ".png"):  # the shelf decodes PNG only
                    candidates.append((kind, src.read_bytes()))
            if not candidates and download:
                art = fetch(thumbnail_url(playlist, named, stem))
                candidates = [("covers", art)] if art else []
            data, how = fitted(card, candidates) if candidates else (None, None)
        if not data:
            missing += 1
            print(f"miss {card}/{stem}", file=sys.stderr)
            continue
        dest = out / card
        dest.mkdir(parents=True, exist_ok=True)
        (dest / f"{stem}.png").write_bytes(data)
        written += 1
        key = (card, how.split(" ")[0])
        sources[key] = sources.get(key, 0) + 1
    for (card, how), n in sorted(sources.items()):
        print(f"  {card:7} {how:24} {n}")
    print(f"{written} labels, {missing} without a picture")
    return written


def self_test():
    import tempfile

    with tempfile.TemporaryDirectory() as tmp:
        root = pathlib.Path(tmp)
        games = root / "Games" / "MD"
        games.mkdir(parents=True)
        (games / "Sonic the Hedgehog (USA).md").write_bytes(b"rom")
        (games / "Bare.bin").write_bytes(b"rom")
        media = root / "downloaded_media" / "megadrive"
        for kind, colour in (("physicalmedia", b"phys"), ("covers", b"cover"), ("marquees", b"marq")):
            folder = media / kind
            folder.mkdir(parents=True)
            (folder / "Sonic the Hedgehog (USA).png").write_bytes(colour)
        arcade_games = root / "Games" / "ARCADE"
        arcade_games.mkdir(parents=True)
        (arcade_games / "mslug.zip").write_bytes(b"zip")
        arcade = root / "downloaded_media" / "arcade"
        for kind, colour in (("physicalmedia", b"cab"), ("marquees", b"marquee")):
            folder = arcade / kind
            folder.mkdir(parents=True)
            (folder / "mslug.png").write_bytes(colour)
        # A clone with no marquee of its own takes its parent's, found under the fbneo folder.
        (arcade_games / "mslugb.zip").write_bytes(b"zip")
        (root / "downloaded_media" / "fbneo" / "marquees").mkdir(parents=True)
        (root / "downloaded_media" / "fbneo" / "marquees" / "mslug2.png").write_bytes(b"parent")
        (arcade_games / "mslug2t.zip").write_bytes(b"zip")
        dat = root / "arcade.dat"
        dat.write_text('<datafile><game name="mslug2"><description>Metal Slug 2</description></game>'
                       '<game name="mslug2t" cloneof="mslug2"><description>Metal Slug 2 Turbo</description></game></datafile>')
        sms = root / "Games" / "SMS"
        sms.mkdir(parents=True)
        (sms / "Teddy Boy.sms").write_bytes(b"rom")
        for kind, colour in (("physicalmedia", b"strip"), ("covers", b"card")):
            folder = root / "downloaded_media" / "mastersystem" / kind
            folder.mkdir(parents=True)
            (folder / "Teddy Boy.png").write_bytes(colour)
        out = root / "Labels"
        run(root / "Games", out, root / "downloaded_media", False, dat)
        assert (out / "ARCADE" / "mslug2t.png").read_bytes() == b"parent"
        assert not (out / "ARCADE" / "mslugb.png").exists()
        assert (out / "SMS" / "Teddy Boy.png").read_bytes() == b"card"
        assert (out / "MD" / "Sonic the Hedgehog (USA).png").read_bytes() == b"phys"
        assert not (out / "MD" / "Bare.png").exists()
        assert (out / "ARCADE" / "mslug.png").read_bytes() == b"marquee"
        # A title that only matches once the forbidden characters are swapped.
        indexed = index_media(media)
        (media / "physicalmedia" / "A_B.png").write_bytes(b"swapped")
        indexed = index_media(media)
        assert local_art("A&B", ("physicalmedia",), indexed).read_bytes() == b"swapped"
        url = thumbnail_url("Sega - Mega Drive - Genesis", "Named_Boxarts", "A&B")
        assert url.endswith("/Sega_-_Mega_Drive_-_Genesis/master/Named_Boxarts/A_B.png"), url
        assert marquee_url("mslug").endswith("/marquees/mslug.png")
    print("self-test ok")


def main(argv):
    if "--self-test" in argv:
        self_test()
        return 0
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--games", type=pathlib.Path, required=True, help="Slot/Games")
    p.add_argument("--out", type=pathlib.Path, required=True, help="Slot/Labels")
    p.add_argument("--media", type=pathlib.Path, help="ES-DE downloaded_media directory")
    p.add_argument("--download", action="store_true", help="fall back to arcadeitalia marquees and libretro-thumbnails")
    p.add_argument("--arcade-dat", type=pathlib.Path, help="FBNeo/MAME XML dat, for a clone's parent and title")
    args = p.parse_args(argv)
    run(args.games, args.out, args.media, args.download, args.arcade_dat)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
