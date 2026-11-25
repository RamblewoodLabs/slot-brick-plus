#!/usr/bin/env python3
"""Fill Labels/<system>/<stem>.png from ES-DE media, then libretro-thumbnails.

ES-DE's downloaded_media is the first source. A picture already next to the
ROM's name is copied as the label. libretro-thumbnails is the fallback, and
only when --download is passed (it fetches over HTTPS).

    python3 brick/build_sp_art.py --games Slot/Games --out Slot/Labels \
        --media /path/to/downloaded_media [--download]
    python3 brick/build_sp_art.py --self-test

Arcade uses the marquee first (it is the label on the wide panel). Every other
new system uses physical media, then the cover, then the marquee. The shelf
scales whatever it gets into the shell's label window (docs/cart-shells.md).

Names follow libretro: &, *, /, :, backtick, <, >, ?, backslash, | and double quote become _.
"""
import argparse
import pathlib
import sys
import urllib.parse
import urllib.request

# Card folder, ES-DE folder names, libretro-thumbnails playlist, kind order.
SYSTEMS = [
    ("MD", ("megadrive", "genesis"), "Sega - Mega Drive - Genesis", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("SMS", ("mastersystem", "sms"), "Sega - Master System - Mark III", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("GG", ("gamegear", "gg"), "Sega - Game Gear", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("ARCADE", ("arcade", "fbneo", "mame"), "FBNeo - Arcade Games", ("marquees", "physicalmedia", "covers"), "Named_Titles"),
    ("PCE", ("pcengine", "tg16", "turbografx"), "NEC - PC Engine - TurboGrafx 16", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("A2600", ("atari2600", "a2600"), "Atari - 2600", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("INTV", ("intellivision", "intv"), "Mattel - Intellivision", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("LYNX", ("atarilynx", "lynx"), "Atari - Lynx", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
    ("A7800", ("atari7800", "a7800"), "Atari - 7800", ("physicalmedia", "covers", "marquees"), "Named_Boxarts"),
]

THUMBS = "https://thumbnails.libretro.com"
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
    # libretro stores the picture under the No-Intro name with those characters swapped.
    file = urllib.parse.quote(libretro_name(stem) + ".png")
    playlist = urllib.parse.quote(playlist)
    return f"{THUMBS}/{playlist}/{named}/{file}"


def each_rom(games):
    for card, aliases, playlist, order, named in SYSTEMS:
        folder = games / card
        if not folder.is_dir():
            continue
        for path in sorted(folder.iterdir()):
            if path.is_file() and not path.name.startswith("."):
                yield card, aliases, playlist, order, named, path


def run(games, out, media, download):
    written = 0
    missing = 0
    for card, aliases, playlist, order, named, rom in each_rom(games):
        stem = rom.stem
        indexed = {}
        if media:
            for alias in aliases:
                indexed = index_media(media / alias)
                if any(indexed[k] for k in indexed):
                    break
        else:
            indexed = index_media(None)
        src = local_art(stem, order, indexed)
        data = None
        if src:
            data = src.read_bytes()
        elif download:
            url = thumbnail_url(playlist, named, stem)
            try:
                with urllib.request.urlopen(url, timeout=30) as reply:
                    data = reply.read()
            except Exception as exc:
                print(f"miss {card}/{stem}: {exc}", file=sys.stderr)
        if not data:
            missing += 1
            continue
        dest = out / card
        dest.mkdir(parents=True, exist_ok=True)
        target = dest / f"{stem}.png"
        # A JPEG from ES-DE is still written as .png only when it already is a PNG.
        # The shelf decodes PNG. A JPEG is saved beside it with its own suffix and skipped.
        if src and src.suffix.lower() != ".png":
            print(f"skip {src.name}: the shelf wants a PNG label", file=sys.stderr)
            missing += 1
            continue
        target.write_bytes(data)
        written += 1
        print(f"{card}/{stem}")
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
        out = root / "Labels"
        run(root / "Games", out, root / "downloaded_media", False)
        assert (out / "MD" / "Sonic the Hedgehog (USA).png").read_bytes() == b"phys"
        assert not (out / "MD" / "Bare.png").exists()
        assert (out / "ARCADE" / "mslug.png").read_bytes() == b"marquee"
        # A title that only matches once the forbidden characters are swapped.
        indexed = index_media(media)
        (media / "physicalmedia" / "A_B.png").write_bytes(b"swapped")
        indexed = index_media(media)
        assert local_art("A&B", ("physicalmedia",), indexed).read_bytes() == b"swapped"
        url = thumbnail_url("Sega - Mega Drive - Genesis", "Named_Boxarts", "A&B")
        assert "A_B.png" in url and "Sega%20-%20Mega%20Drive%20-%20Genesis" in url
    print("self-test ok")


def main(argv):
    if "--self-test" in argv:
        self_test()
        return 0
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--games", type=pathlib.Path, required=True, help="Slot/Games")
    p.add_argument("--out", type=pathlib.Path, required=True, help="Slot/Labels")
    p.add_argument("--media", type=pathlib.Path, help="ES-DE downloaded_media directory")
    p.add_argument("--download", action="store_true", help="fall back to libretro-thumbnails")
    args = p.parse_args(argv)
    run(args.games, args.out, args.media, args.download)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
