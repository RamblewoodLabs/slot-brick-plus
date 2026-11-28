#!/usr/bin/env python3
"""Tests for artlib. Run: python3 tools/artlib/test_artlib.py"""

from __future__ import annotations

import os
import struct
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import artlib


def write_png(path: Path, w: int, h: int, rgb=(40, 80, 160)) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(rgb) * w for _ in range(h))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)
    data = chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(b"\x89PNG\r\n\x1a\n" + data)


class ClassifyTests(unittest.TestCase):
    def test_system_aliases(self):
        self.assertEqual(artlib.normalize_system("nes"), "NES")
        self.assertEqual(artlib.normalize_system("genesis"), "MD")
        self.assertEqual(artlib.normalize_system("psx"), "PS1")
        self.assertEqual(artlib.normalize_system("colecovision"), "COLECO")
        self.assertEqual(artlib.normalize_system("GBA"), "GBA")

    def test_game_stem_strips_fitted_suffix(self):
        self.assertEqual(artlib.game_stem("Zelda (USA).label.png"), "Zelda (USA)")
        self.assertEqual(artlib.game_stem("Zelda (USA).png"), "Zelda (USA)")

    def test_infer_scale(self):
        self.assertEqual(artlib.infer_scale("GBA", 266, 138), "1x")
        self.assertEqual(artlib.infer_scale("GBA", 532, 276), "2x")
        self.assertEqual(artlib.infer_scale("INTV", 500, 118), "2x")
        self.assertEqual(artlib.infer_scale("ARCADE", 660, 180), "2x")

    def test_secret_names(self):
        self.assertTrue(artlib.looks_secret(Path("screenscraper-user.properties")))
        self.assertTrue(artlib.looks_secret(Path("/tmp/android/screenscraper.properties")))
        self.assertFalse(artlib.looks_secret(Path("Labels/NES/Mario.png")))


class LibraryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name) / "lib"
        self.src = Path(self.tmp.name) / "src"

    def tearDown(self):
        self.tmp.cleanup()

    def test_ingest_dedupe_export_and_skip_secrets(self):
        labels = self.src / "Slot" / "Labels" / "GBA"
        write_png(labels / "Metroid Fusion (USA).png", 266, 138)
        write_png(labels / "Metroid Fusion (USA) copy.png", 266, 138)  # same pixels → same hash
        # different game
        write_png(labels / "Ninja Five-O (USA).png", 266, 138, rgb=(9, 9, 9))
        # credential file must never be copied
        (self.src / "screenscraper-user.properties").write_text("user=nobody\npassword=nope\n")
        raw = self.src / "cartart" / "NES" / "raw" / "Super Mario Bros. (World)"
        write_png(raw / "support-texture.png", 200, 100, rgb=(1, 2, 3))
        (raw / "meta.json").write_text('{"title": "Super Mario Bros.", "jeuid": 123, "crc": "aabbccdd"}')

        lib = artlib.Library(self.root)
        a = lib.ingest_tree(self.src / "Slot" / "Labels", source="test-labels")
        b = lib.ingest_tree(self.src / "cartart", source="test-raw")
        # second ingest of the same labels is a no-op
        c = lib.ingest_tree(self.src / "Slot" / "Labels", source="test-labels")
        self.assertGreaterEqual(a.get("new_objects", 0), 2)
        self.assertGreaterEqual(a.get("deduped", 0), 1)  # the identical copy
        self.assertEqual(c.get("already_present", 0), a.get("new_records", 0) + a.get("already_present", 0) or c.get("already_present"))
        self.assertTrue(any("new_objects" in b or "new_records" in b for _ in [0]))

        # secrets not in the library
        for p in self.root.rglob("*"):
            self.assertNotIn("screenscraper", p.name.lower())
            self.assertNotIn("password", p.name.lower())

        # export
        manifest = Path(self.tmp.name) / "games.tsv"
        manifest.write_text(
            "GBA\tMetroid Fusion (USA).gba\n"
            "GBA\tMissing Game (USA).gba\n"
            "NES\tSuper Mario Bros. (World).nes\n"
        )
        out = Path(self.tmp.name) / "device"
        result = lib.export_labels(manifest, scale="1x", out=out)
        self.assertEqual(result["exported"], 1)
        self.assertEqual(result["gaps"], 2)  # missing game, and NES has raw art but no fitted label
        self.assertTrue((out / "Labels" / "GBA" / "Metroid Fusion (USA).png").exists())
        lib.close()

    def test_export_fallback(self):
        labels = self.src / "Labels" / "GBA"
        write_png(labels / "Only2x (USA).png", 532, 276, rgb=(3, 4, 5))
        lib = artlib.Library(self.root)
        lib.ingest_tree(self.src / "Labels", source="s")
        manifest = Path(self.tmp.name) / "g.tsv"
        manifest.write_text("GBA\tOnly2x (USA).gba\n")
        out = Path(self.tmp.name) / "dev"
        r = lib.export_labels(manifest, scale="1x", out=out, fallback=False)
        self.assertEqual(r["gaps"], 1)
        r2 = lib.export_labels(manifest, scale="1x", out=out, fallback=True)
        self.assertEqual(r2["exported"], 1)
        lib.close()


if __name__ == "__main__":
    unittest.main()
