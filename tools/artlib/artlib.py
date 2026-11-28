#!/usr/bin/env python3
"""Private local art library for Slotted / slot-brick-plus.

Keeps cart labels, box art, and raw ScreenScraper media in one place on the
Mac mini so a new device can be filled offline. Stdlib only.

Never prints, logs, or copies ScreenScraper credentials (or any password file).
The image library itself must stay out of git: it is copyrighted art.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import sqlite3
import stat
import subprocess
import sys
import tempfile
import time
import zipfile
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Iterable, Iterator, Optional

DEFAULT_ROOT = Path("/Volumes/External 4TB/AI/ArtLibrary/slotted")
POINTER_ROOT = Path.home() / "ArtLibrary"
SCHEMA_VERSION = 1

IMAGE_EXTS = {".png", ".jpg", ".jpeg", ".webp", ".gif", ".avif", ".bmp"}
META_EXTS = {".json"}
# Never ingest these, even if they appear next to art.
SKIP_EXTS = {
    ".properties",
    ".env",
    ".pem",
    ".key",
    ".mgw",
    ".gba",
    ".gbc",
    ".gb",
    ".nes",
    ".sfc",
    ".smc",
    ".z64",
    ".n64",
    ".v64",
    ".iso",
    ".cue",
    ".bin",
    ".chd",
    ".pbp",
    ".cso",
    ".nds",
    ".3ds",
    ".cia",
    ".nsp",
    ".xci",
    ".rvz",
    ".wad",
    ".wbfs",
    ".zip",  # handled separately when an ingest path *is* a zip
    ".7z",
    ".rar",
    ".mp4",
    ".webm",
    ".mkv",
    ".avi",
    ".mov",
    ".pdf",
    ".exe",
    ".dll",
    ".so",
    ".dylib",
    ".rom",
}
SECRET_NAME_RE = re.compile(
    r"(screenscraper.*\.properties|password|passwd|devpassword|secret|\.env$|"
    r"credentials|id_rsa|id_ed25519|wifi\.txt|retroachievements)",
    re.I,
)
SECRET_KEY_RE = re.compile(
    r"(password|passwd|devpassword|devid|secret|token|api[_-]?key|login)",
    re.I,
)

# Slot folder names, plus aliases used by EmuDeck / ES-DE / Windows dumps.
SYSTEM_ALIASES = {
    "32x": "32X",
    "s32x": "32X",
    "sega32x": "32X",
    "a2600": "A2600",
    "atari2600": "A2600",
    "a5200": "A5200",
    "atari5200": "A5200",
    "a7800": "A7800",
    "atari7800": "A7800",
    "a800": "A800",
    "atari800": "A800",
    "arcade": "ARCADE",
    "mame": "ARCADE",
    "fbneo": "ARCADE",
    "cps1": "ARCADE",
    "cps2": "ARCADE",
    "cps3": "ARCADE",
    "coleco": "COLECO",
    "colecovision": "COLECO",
    "gb": "GB",
    "gameboy": "GB",
    "gba": "GBA",
    "gbc": "GBC",
    "gbcolor": "GBC",
    "gg": "GG",
    "gamegear": "GG",
    "gw": "GW",
    "gameandwatch": "GW",
    "gameandwatchhandheld": "GW",
    "intv": "INTV",
    "intellivision": "INTV",
    "lynx": "LYNX",
    "atarilynx": "LYNX",
    "md": "MD",
    "genesis": "MD",
    "megadrive": "MD",
    "n64": "N64",
    "nds": "NDS",
    "nes": "NES",
    "pce": "PCE",
    "tg16": "PCE",
    "pcengine": "PCE",
    "turbografx16": "PCE",
    "turbografx-16": "PCE",
    "ps1": "PS1",
    "psx": "PS1",
    "psp": "PSP",
    "sms": "SMS",
    "mastersystem": "SMS",
    "snes": "SNES",
    "vectrex": "VECTREX",
    "slotintv": "INTV",
}

# 1x label windows from the cart shells in slot-brick-plus (assets/carts/*/meta.toml).
LABEL_1X = {
    "32X": (240, 197),
    "A2600": (227, 279),
    "A5200": (216, 181),
    "A7800": (227, 279),
    "A800": (261, 86),
    "ARCADE": (330, 90),
    "COLECO": (254, 268),
    "GB": (220, 195),
    "GBA": (266, 138),
    "GBC": (220, 195),
    "GG": (270, 207),
    "GW": (360, 331),
    "INTV": (250, 59),
    "LYNX": (286, 263),
    "MD": (240, 196),
    "N64": (164, 167),
    "NDS": (312, 270),
    "NES": (132, 222),
    "NES-FC": (214, 110),  # Famicom
    "PCE": (181, 237),
    "PS1": (300, 300),
    "PSP": (286, 287),
    "SMS": (181, 198),
    "SNES": (216, 99),
    "SNES-PAL": (272, 93),
    "VECTREX": (170, 192),
}

MEDIA_ALIASES = {
    "covers": "box-2D",
    "cover": "box-2D",
    "boxart": "box-2D",
    "box-art": "box-2D",
    "box-2d": "box-2D",
    "box2d": "box-2D",
    "backcovers": "box-2D-back",
    "backcover": "box-2D-back",
    "box-2d-back": "box-2D-back",
    "box-2d-side": "box-2D-side",
    "3dboxes": "box-3D",
    "3dbox": "box-3D",
    "box-3d": "box-3D",
    "box-texture": "box-texture",
    "physicalmedia": "support-2D",
    "physical-media": "support-2D",
    "support-2d": "support-2D",
    "support-2D": "support-2D",
    "cart": "support-2D",
    "cartphoto": "support-2D",
    "support-texture": "support-texture",
    "texture": "support-texture",
    "label-scan": "support-texture",
    "marquees": "marquee",
    "marquee": "marquee",
    "screenmarquee": "screenmarquee",
    "miximages": "mix",
    "mix": "mix",
    "mixrbv1": "mixrbv1",
    "mixrbv2": "mixrbv2",
    "fanart": "fanart",
    "titlescreens": "titlescreen",
    "titlescreen": "titlescreen",
    "wheel": "logo",
    "wheel-hd": "logo-hd",
    "logo": "logo",
    "screenshots": "screenshot",
    "screenshot": "screenshot",
    "label": "label",
    "labels": "label",
    "fitted": "label",
    "imgs": "box-2D",
}

# Folder names that mean "this tree is already Labels/<SYS>/<game>.png"
LABELS_DIR_NAMES = {"labels", "label"}
RAW_DIR_NAMES = {"raw"}
FITTED_DIR_NAMES = {"fitted"}

SKIP_DIR_NAMES = {
    ".git",
    ".svn",
    "node_modules",
    "target",
    "build",
    "roms",
    "games",
    "bios",
    "saves",
    "states",
    "system",
    "config",
    "overlays",
    "videos",
    "manuals",
    "__macosx",
}


def utc_now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def eprint(*args, **kwargs) -> None:
    print(*args, file=sys.stderr, **kwargs)


def looks_secret(path: Path) -> bool:
    name = path.name
    if SECRET_NAME_RE.search(name):
        return True
    # Path segments like screenscraper-user.properties
    return any(SECRET_NAME_RE.search(part) for part in path.parts)


def normalize_system(name: str) -> str:
    key = name.strip().replace(" ", "").replace("_", "").replace("-", "").lower()
    # try raw lower first (gb, gba), then stripped
    lower = name.strip().lower()
    if lower in SYSTEM_ALIASES:
        return SYSTEM_ALIASES[lower]
    if key in SYSTEM_ALIASES:
        return SYSTEM_ALIASES[key]
    # already a Slot folder like "GBA"
    if name.strip().upper() in {v for v in SYSTEM_ALIASES.values()}:
        return name.strip().upper()
    # keep unknown systems as a tidy uppercase token
    cleaned = re.sub(r"[^A-Za-z0-9]+", "", name).upper() or name.strip()
    return cleaned


def normalize_media(name: str) -> str:
    key = name.strip().lower()
    if key in MEDIA_ALIASES:
        return MEDIA_ALIASES[key]
    return name.strip()


def game_stem(filename: str) -> str:
    name = Path(filename).name
    # fitted variants: "Zelda (USA).label.png" -> "Zelda (USA)"
    for suffix in (
        ".label",
        ".cart",
        ".plate",
        ".tier-box",
        ".tier-logo",
        ".tier-photo",
        ".tier-texture",
        ".tier-marquee",
    ):
        if name.lower().endswith(suffix + Path(name).suffix.lower()) or name.lower().endswith(
            suffix + ".png"
        ):
            name = re.sub(re.escape(suffix) + r"(\.[^.]+)?$", "", name, flags=re.I)
            break
    else:
        name = Path(name).stem
    return name.strip()


def fitted_variant(filename: str) -> Optional[str]:
    name = Path(filename).name.lower()
    for suffix in (
        "label",
        "cart",
        "plate",
        "tier-box",
        "tier-logo",
        "tier-photo",
        "tier-texture",
        "tier-marquee",
    ):
        if f".{suffix}." in name or name.endswith(f".{suffix}.png"):
            return suffix
    return None


def safe_segment(s: str) -> str:
    # Keep No-Intro names readable; strip path separators and NULs only.
    s = s.replace("\x00", "").replace("/", "／").replace("\\", "／")
    s = s.strip().strip(".")
    return s or "unnamed"


def sha256_file(path: Path, bufsize: int = 1024 * 1024) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        while True:
            chunk = f.read(bufsize)
            if not chunk:
                break
            h.update(chunk)
    return h.hexdigest()


def png_size(path: Path) -> Optional[tuple[int, int]]:
    try:
        with path.open("rb") as f:
            sig = f.read(8)
            if sig != b"\x89PNG\r\n\x1a\n":
                return None
            length = int.from_bytes(f.read(4), "big")
            typ = f.read(4)
            if typ != b"IHDR" or length < 8:
                return None
            w = int.from_bytes(f.read(4), "big")
            h = int.from_bytes(f.read(4), "big")
            return w, h
    except OSError:
        return None


def infer_scale(system: str, width: Optional[int], height: Optional[int]) -> Optional[str]:
    if not width or not height:
        return None
    candidates = [(system, LABEL_1X[system])] if system in LABEL_1X else []
    # Also try close relatives (GB/GBC share a window; NES-FC).
    if system == "GB":
        candidates.append(("GBC", LABEL_1X["GBC"]))
    if system == "GBC":
        candidates.append(("GB", LABEL_1X["GB"]))
    if system == "NES":
        candidates.append(("NES-FC", LABEL_1X["NES-FC"]))
    if system == "SNES":
        candidates.append(("SNES-PAL", LABEL_1X["SNES-PAL"]))
    for _name, (lw, lh) in candidates:
        for scale, tag in ((1, "1x"), (2, "2x"), (3, "3x")):
            if abs(width - lw * scale) <= 2 and abs(height - lh * scale) <= 4:
                return tag
    # Known N64 labels on the systems2 card were 328x352 (2x width, slightly taller).
    if system == "N64" and width in (328, 164) and height in (167, 334, 352):
        return "2x" if width == 328 else "1x"
    return None


def object_relpath(sha: str, ext: str) -> Path:
    ext = ext.lower() if ext.startswith(".") else f".{ext.lower()}"
    return Path("objects") / "sha256" / sha[:2] / sha[2:4] / f"{sha}{ext}"


def copy_into(src: Path, dest: Path) -> str:
    """Copy src to dest. Prefer APFS clone when it works; otherwise a normal copy.

    Never moves or deletes the source.
    """
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = dest.with_name(dest.name + ".tmp")
    if tmp.exists():
        tmp.unlink()
    method = "copy"
    # clonefile (APFS, same volume) — chmod on the dest inode is then safe.
    try:
        r = subprocess.run(
            ["cp", "-c", str(src), str(tmp)],
            check=False,
            capture_output=True,
        )
        if r.returncode == 0 and tmp.exists() and tmp.stat().st_size == src.stat().st_size:
            method = "clone"
        else:
            if tmp.exists():
                tmp.unlink()
            shutil.copy2(src, tmp)
    except OSError:
        if tmp.exists():
            tmp.unlink()
        shutil.copy2(src, tmp)
    os.replace(tmp, dest)
    return method


def connect(root: Path) -> sqlite3.Connection:
    root.mkdir(parents=True, exist_ok=True)
    try:
        os.chmod(root, 0o700)
        if root.parent.name == "ArtLibrary":
            os.chmod(root.parent, 0o700)
    except OSError:
        pass
    db = root / "manifest.sqlite"
    con = sqlite3.connect(str(db))
    con.row_factory = sqlite3.Row
    con.execute("PRAGMA journal_mode=WAL")
    con.execute("PRAGMA foreign_keys=ON")
    con.executescript(
        """
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS artifacts (
            sha256 TEXT PRIMARY KEY,
            size INTEGER NOT NULL,
            ext TEXT NOT NULL,
            width INTEGER,
            height INTEGER,
            stored_path TEXT NOT NULL,
            first_ingested_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            sha256 TEXT NOT NULL REFERENCES artifacts(sha256),
            system TEXT NOT NULL,
            game TEXT NOT NULL,
            kind TEXT NOT NULL,
            media_type TEXT NOT NULL,
            scale TEXT,
            region TEXT,
            source TEXT NOT NULL,
            source_path TEXT NOT NULL,
            rom_crc TEXT,
            rom_name TEXT,
            ss_media_id TEXT,
            ingested_at TEXT NOT NULL,
            UNIQUE(sha256, system, game, media_type, scale, source, source_path)
        );
        CREATE INDEX IF NOT EXISTS idx_files_sys_game ON files(system, game);
        CREATE INDEX IF NOT EXISTS idx_files_kind ON files(kind, media_type);
        CREATE INDEX IF NOT EXISTS idx_files_scale ON files(scale);
        """
    )
    cur = con.execute("SELECT value FROM meta WHERE key='schema'").fetchone()
    if cur is None:
        con.execute(
            "INSERT INTO meta(key, value) VALUES('schema', ?)", (str(SCHEMA_VERSION),)
        )
        con.execute("INSERT INTO meta(key, value) VALUES('created_at', ?)", (utc_now(),))
        con.commit()
    return con


def write_library_readme(root: Path) -> None:
    path = root / "README.txt"
    if path.exists():
        return
    path.write_text(
        "Slotted private art library\n"
        "===========================\n\n"
        "This folder holds cartridge labels, box art, and raw ScreenScraper media\n"
        "for Jeff's Slotted / slot-brick-plus project. The images are copyrighted.\n"
        "Do not copy them into a public git repository, attach them to a pull\n"
        "request, or share this folder over the network.\n\n"
        "Permissions are 700 (Jeff's account only).\n\n"
        "Use the artlib CLI in the slot-brick-plus repo:\n"
        "  python3 tools/artlib/artlib.py stats\n"
        "  python3 tools/artlib/artlib.py ingest PATH --source NAME\n"
        "  python3 tools/artlib/artlib.py export --manifest GAMES.tsv --scale 1x --out OUT\n\n"
        f"Library root: {root}\n",
        encoding="utf-8",
    )


def write_home_pointer(real_root: Path) -> None:
    # Never retarget ~/ArtLibrary when operating on a temporary or custom root
    # (unit tests, one-off --root paths).
    official = Path(os.environ.get("ARTLIB_ROOT", str(DEFAULT_ROOT))).expanduser()
    try:
        if real_root.resolve() != official.resolve():
            return
    except OSError:
        return
    POINTER_ROOT.mkdir(parents=True, exist_ok=True)
    try:
        os.chmod(POINTER_ROOT, 0o700)
    except OSError:
        pass
    readme = POINTER_ROOT / "README.txt"
    readme.write_text(
        "The Slotted art library lives on the Mac mini's External 4TB drive,\n"
        "not in this folder, because the internal disk is nearly full and the\n"
        "image set is larger than the free space here.\n\n"
        f"  {real_root}\n\n"
        "This folder stays mode 700 and holds only a pointer, not the images.\n"
        "Time Machine backs up /Users/jeff (this pointer) but not the External\n"
        "4TB volume itself. The original EmuDeck media also remains on the EVO.\n"
        "The generated labels and ScreenScraper raw media we made are in the\n"
        "library; re-run ingest after a preload run to pick up new labels.\n",
        encoding="utf-8",
    )
    link = POINTER_ROOT / "slotted"
    try:
        if link.is_symlink() or link.exists():
            if link.is_symlink() and os.readlink(link) == str(real_root):
                return
        if not link.exists():
            link.symlink_to(real_root)
    except OSError:
        pass


class Library:
    def __init__(self, root: Path):
        self.root = root.expanduser().resolve()
        self.con = connect(self.root)
        write_library_readme(self.root)
        write_home_pointer(self.root)

    def close(self) -> None:
        self.con.close()

    def ingest_tree(
        self,
        src: Path,
        source: str,
        kind_hint: Optional[str] = None,
        default_system: Optional[str] = None,
        default_scale: Optional[str] = None,
    ) -> dict:
        src = src.expanduser()
        if not src.exists():
            raise FileNotFoundError(f"source not found: {src}")
        counters = Counter()
        if src.is_file() and src.suffix.lower() == ".zip":
            return self.ingest_zip(src, source, kind_hint, default_system, default_scale)
        for rec in classify_tree(src, kind_hint, default_system, default_scale):
            self._ingest_one(rec, source, counters)
        self.con.commit()
        return dict(counters)

    def ingest_zip(
        self,
        zip_path: Path,
        source: str,
        kind_hint: Optional[str],
        default_system: Optional[str],
        default_scale: Optional[str],
    ) -> dict:
        # Extract onto the library volume, never the nearly-full internal disk.
        dest = self.root / "tmp" / f"extract-{int(time.time())}"
        dest.mkdir(parents=True, exist_ok=True)
        counters = Counter()
        try:
            with zipfile.ZipFile(zip_path) as zf:
                for info in zf.infolist():
                    name = info.filename.replace("\\", "/")
                    if info.is_dir():
                        continue
                    # zip-slip guard
                    target = (dest / name).resolve()
                    if not str(target).startswith(str(dest.resolve()) + os.sep) and target != dest.resolve():
                        counters["skipped_unsafe"] += 1
                        continue
                    if looks_secret(Path(name)):
                        counters["skipped_secret"] += 1
                        continue
                    ext = Path(name).suffix.lower()
                    if ext not in IMAGE_EXTS | META_EXTS:
                        counters["skipped_non_image"] += 1
                        continue
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with zf.open(info) as src, target.open("wb") as out:
                        shutil.copyfileobj(src, out)
            for rec in classify_tree(dest, kind_hint, default_system, default_scale):
                # Record the zip as the source path, plus the inner path.
                rec["source_path_override"] = f"{zip_path}::{rec['path'].relative_to(dest)}"
                self._ingest_one(rec, source, counters)
            self.con.commit()
        finally:
            shutil.rmtree(dest, ignore_errors=True)
            try:
                (self.root / "tmp").rmdir()
            except OSError:
                pass
        return dict(counters)

    def _ingest_one(self, rec: dict, source: str, counters: Counter) -> None:
        path: Path = rec["path"]
        if looks_secret(path):
            counters["skipped_secret"] += 1
            return
        ext = path.suffix.lower()
        if ext not in IMAGE_EXTS | META_EXTS:
            counters["skipped_non_image"] += 1
            return
        if ext in META_EXTS:
            if meta_contains_secrets(path):
                counters["skipped_secret"] += 1
                return
        try:
            size = path.stat().st_size
        except OSError as e:
            counters["errors"] += 1
            eprint(f"skip (stat): {path}: {e}")
            return
        if size == 0:
            counters["skipped_empty"] += 1
            return
        try:
            sha = sha256_file(path)
        except OSError as e:
            counters["errors"] += 1
            eprint(f"skip (read): {path}: {e}")
            return

        width = height = None
        if ext == ".png":
            wh = png_size(path)
            if wh:
                width, height = wh
            elif rec.get("kind") == "label":
                # A label being written right now can be a truncated PNG; skip and retry later.
                counters["skipped_partial"] += 1
                return

        system = rec["system"]
        game = rec["game"]
        kind = rec["kind"]
        media_type = rec["media_type"]
        scale = rec.get("scale")
        if kind == "label" and not scale:
            scale = infer_scale(system, width, height)
        scale = scale or ""
        region = rec.get("region") or ""
        ss_media_id = rec.get("ss_media_id")
        rom_crc = rec.get("rom_crc")
        rom_name = rec.get("rom_name")
        source_path = rec.get("source_path_override") or str(path)

        art = self.con.execute(
            "SELECT stored_path FROM artifacts WHERE sha256=?", (sha,)
        ).fetchone()
        if art is None:
            rel = object_relpath(sha, ext)
            dest = self.root / rel
            if not dest.exists():
                try:
                    copy_into(path, dest)
                except OSError as e:
                    counters["errors"] += 1
                    eprint(f"skip (copy): {path}: {e}")
                    return
            self.con.execute(
                "INSERT INTO artifacts(sha256, size, ext, width, height, stored_path, first_ingested_at) "
                "VALUES (?,?,?,?,?,?,?)",
                (sha, size, ext, width, height, str(rel), utc_now()),
            )
            counters["new_objects"] += 1
            counters["new_bytes"] += size
        else:
            counters["deduped"] += 1

        try:
            self.con.execute(
                "INSERT INTO files(sha256, system, game, kind, media_type, scale, region, "
                "source, source_path, rom_crc, rom_name, ss_media_id, ingested_at) "
                "VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
                (
                    sha,
                    system,
                    game,
                    kind,
                    media_type,
                    scale,
                    region,
                    source,
                    source_path,
                    rom_crc,
                    rom_name,
                    ss_media_id,
                    utc_now(),
                ),
            )
            counters["new_records"] += 1
            self._link_index(
                sha, ext, system, game, kind, media_type, scale, region
            )
        except sqlite3.IntegrityError:
            counters["already_present"] += 1

    def _link_index(
        self,
        sha: str,
        ext: str,
        system: str,
        game: str,
        kind: str,
        media_type: str,
        scale: Optional[str],
        region: Optional[str],
    ) -> None:
        obj = self.root / object_relpath(sha, ext)
        if not obj.exists():
            return
        if kind == "label":
            scale_part = scale or "unknown"
            rel = (
                Path("index")
                / "labels"
                / scale_part
                / safe_segment(system)
                / f"{safe_segment(game)}{ext.lower() if ext.startswith('.') else '.'+ext.lower()}"
            )
        elif kind == "raw":
            extra = []
            if region:
                extra.append(region)
            suffix = ("--" + "--".join(extra)) if extra else ""
            rel = (
                Path("index")
                / "raw"
                / safe_segment(system)
                / safe_segment(game)
                / f"{safe_segment(media_type)}{suffix}{ext if ext.startswith('.') else '.'+ext}"
            )
        else:
            rel = (
                Path("index")
                / "other"
                / safe_segment(media_type)
                / safe_segment(system)
                / f"{safe_segment(game)}{ext if ext.startswith('.') else '.'+ext}"
            )
        dest = self.root / rel
        # Collision: same name, different hash → include short hash.
        if dest.exists():
            try:
                if dest.stat().st_ino == obj.stat().st_ino:
                    return
            except OSError:
                pass
            dest = dest.with_name(f"{dest.stem}--{sha[:8]}{dest.suffix}")
            if dest.exists():
                return
        dest.parent.mkdir(parents=True, exist_ok=True)
        try:
            os.link(obj, dest)
        except OSError:
            try:
                copy_into(obj, dest)
            except OSError:
                pass

    def export_labels(
        self,
        manifest: Path,
        scale: str,
        out: Path,
        fallback: bool = False,
    ) -> dict:
        rows = parse_rom_manifest(manifest)
        scale = scale.lower().replace("×", "x")
        if scale in {"1", "2", "3"}:
            scale = f"{scale}x"
        out_labels = out / "Labels"
        out_labels.mkdir(parents=True, exist_ok=True)
        report = []
        counters = Counter()
        for row in rows:
            system = normalize_system(row["system"])
            stem = row["stem"]
            dest = out_labels / system / f"{stem}.png"
            hit = self._best_label(system, stem, scale)
            if hit is None and fallback:
                hit = self._best_label(system, stem, None)
                if hit:
                    row_note = f"fallback:{hit['scale'] or 'unknown'}"
                else:
                    row_note = ""
            else:
                row_note = ""
            if hit is None:
                other = self._best_label(system, stem, None)
                report.append(
                    {
                        "system": system,
                        "game": stem,
                        "status": "gap",
                        "note": (
                            f"have scale {other['scale']}" if other else "no label in library"
                        ),
                    }
                )
                counters["gaps"] += 1
                continue
            src = self.root / hit["stored_path"]
            dest.parent.mkdir(parents=True, exist_ok=True)
            copy_into(src, dest)
            report.append(
                {
                    "system": system,
                    "game": stem,
                    "status": "ok",
                    "scale": hit["scale"],
                    "sha256": hit["sha256"],
                    "note": row_note,
                }
            )
            counters["exported"] += 1
        gaps_path = out / "artlib-export-report.tsv"
        with gaps_path.open("w", encoding="utf-8") as f:
            f.write("system\tgame\tstatus\tscale\tnote\tsha256\n")
            for r in report:
                f.write(
                    f"{r.get('system','')}\t{r.get('game','')}\t{r.get('status','')}\t"
                    f"{r.get('scale','')}\t{r.get('note','')}\t{r.get('sha256','')}\n"
                )
        return {
            "exported": int(counters.get("exported", 0)),
            "gaps": int(counters.get("gaps", 0)),
            "report": str(gaps_path),
        }

    def _best_label(self, system: str, stem: str, scale: Optional[str]) -> Optional[dict]:
        # Prefer an exact game-name match, then a looser one (ignore punctuation).
        if scale:
            rows = self.con.execute(
                "SELECT f.sha256, f.scale, a.stored_path, f.media_type, f.game "
                "FROM files f JOIN artifacts a ON a.sha256=f.sha256 "
                "WHERE f.system=? AND f.kind='label' AND f.scale=? "
                "ORDER BY CASE f.media_type WHEN 'label' THEN 0 ELSE 1 END, f.id DESC",
                (system, scale),
            ).fetchall()
        else:
            rows = self.con.execute(
                "SELECT f.sha256, f.scale, a.stored_path, f.media_type, f.game "
                "FROM files f JOIN artifacts a ON a.sha256=f.sha256 "
                "WHERE f.system=? AND f.kind='label' "
                "ORDER BY CASE f.scale WHEN '2x' THEN 0 WHEN '1x' THEN 1 ELSE 2 END, "
                "CASE f.media_type WHEN 'label' THEN 0 ELSE 1 END, f.id DESC",
                (system,),
            ).fetchall()
        exact = None
        loose = None
        want = normalize_game_key(stem)
        for r in rows:
            key = normalize_game_key(r["game"])
            if r["game"] == stem or Path(r["game"]).stem == Path(stem).stem:
                exact = dict(r)
                break
            if key == want and loose is None:
                loose = dict(r)
        return exact or loose

    def stats(self) -> dict:
        arts = self.con.execute(
            "SELECT COUNT(*) c, COALESCE(SUM(size),0) b FROM artifacts"
        ).fetchone()
        files = self.con.execute("SELECT COUNT(*) c FROM files").fetchone()[0]
        by_system = self.con.execute(
            "SELECT system, kind, media_type, scale, COUNT(*) c FROM files "
            "GROUP BY 1,2,3,4 ORDER BY 1,2,3"
        ).fetchall()
        return {
            "root": str(self.root),
            "objects": arts["c"],
            "bytes": arts["b"],
            "records": files,
            "rows": [dict(r) for r in by_system],
        }


def normalize_game_key(name: str) -> str:
    s = game_stem(name).lower()
    s = re.sub(r"\([^)]*\)|\[[^\]]*\]", "", s)
    s = re.sub(r"[^a-z0-9]+", "", s)
    return s


def meta_contains_secrets(path: Path) -> bool:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return True
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return bool(SECRET_KEY_RE.search(text))
    return _json_has_secret_keys(data)


def _json_has_secret_keys(obj) -> bool:
    if isinstance(obj, dict):
        for k, v in obj.items():
            if SECRET_KEY_RE.search(str(k)):
                return True
            if _json_has_secret_keys(v):
                return True
    elif isinstance(obj, list):
        return any(_json_has_secret_keys(v) for v in obj)
    return False


def classify_tree(
    root: Path,
    kind_hint: Optional[str],
    default_system: Optional[str],
    default_scale: Optional[str],
) -> Iterator[dict]:
    """Walk a source tree and yield ingest records. Read-only."""
    root = root.resolve()
    for dirpath, dirnames, filenames in os.walk(root):
        # Prune
        dirnames[:] = [
            d
            for d in dirnames
            if d.lower() not in SKIP_DIR_NAMES and not d.startswith(".")
        ]
        rel_parts = Path(dirpath).resolve().relative_to(root).parts
        if rel_parts == (".",) or rel_parts == ():
            rel_parts = ()
        for name in filenames:
            path = Path(dirpath) / name
            if name.startswith("."):
                continue
            if looks_secret(path):
                continue
            ext = path.suffix.lower()
            if ext not in IMAGE_EXTS | META_EXTS:
                continue
            rec = classify_file(root, path, rel_parts, kind_hint, default_system, default_scale)
            if rec:
                yield rec


def classify_file(
    root: Path,
    path: Path,
    rel_parts: tuple,
    kind_hint: Optional[str],
    default_system: Optional[str],
    default_scale: Optional[str],
) -> Optional[dict]:
    # Include the ingest root itself (ingesting a folder named Labels/ must
    # still be seen as a label tree even though "Labels" is not a relative part).
    parts_lower = [x.lower() for x in (root.parts + rel_parts)]
    system = default_system
    kind = kind_hint
    media_type = None
    scale = default_scale
    region = None
    game = game_stem(path.name)
    variant = fitted_variant(path.name)

    # Full path components, so system/game are taken from names around raw/fitted/Labels.
    full = list(path.parent.parts)
    full_l = [x.lower() for x in full]

    # cartart-test layout: <SYS>/raw/<game>/<media>.png  or  <SYS>/fitted/<game>.label.png
    if "raw" in full_l:
        i = full_l.index("raw")
        if i >= 1:
            system = normalize_system(full[i - 1])
        if i + 1 < len(full):
            game = full[i + 1]
        media_type = normalize_media(path.stem)
        kind = kind or "raw"
    elif "fitted" in full_l:
        i = full_l.index("fitted")
        # cartart-test is SYS/fitted/file; slot-cart-renders is fitted/SYS/file
        if i + 1 < len(full):
            system = normalize_system(full[i + 1])
        elif i >= 1:
            system = normalize_system(full[i - 1])
        kind = "label"
        media_type = variant or "label"
    elif "labels" in full_l or (kind_hint == "label"):
        if "labels" in full_l:
            i = full_l.index("labels")
            if i + 1 < len(full):
                system = normalize_system(full[i + 1])
        kind = "label"
        media_type = variant or "label"
    else:
        # EmuDeck / mgt17 / slot-intv: <system>/<type>/<game>.png
        for i, p in enumerate(full_l):
            if p in MEDIA_ALIASES:
                media_type = normalize_media(p)
                if system is None:
                    # Walk left past generic folders like media/ or art/.
                    generic = {
                        "media", "art", "images", "imgs", "downloaded_media",
                        "artwork", "pictures", "boxart", "modern",
                    }
                    for j in range(i - 1, -1, -1):
                        cand = full[j]
                        if cand.lower() in generic:
                            continue
                        system = normalize_system(cand)
                        break
                break
        if kind is None:
            if media_type in {
                "support-2D",
                "support-texture",
                "box-2D",
                "box-2D-back",
                "box-2D-side",
                "box-3D",
                "box-texture",
                "marquee",
                "screenmarquee",
                "mix",
                "mixrbv1",
                "mixrbv2",
                "fanart",
                "logo",
                "logo-hd",
                "titlescreen",
                "screenshot",
            }:
                # Raw ScreenScraper dumps use these names under raw/; EmuDeck uses covers/ etc.
                kind = "other" if media_type in {
                    "box-2D",
                    "box-2D-back",
                    "box-3D",
                    "support-2D",
                    "marquee",
                    "mix",
                    "fanart",
                    "titlescreen",
                    "screenshot",
                    "logo",
                    "logo-hd",
                } and "raw" not in parts_lower else "raw"
                # EmuDeck-style trees are "other" (box/physical/marquee collections).
                if "raw" not in parts_lower:
                    kind = "other"
            else:
                kind = kind_hint or "other"
                media_type = media_type or (variant or "image")

    known = set(SYSTEM_ALIASES.values())
    if system is not None and system not in known and default_system:
        system = normalize_system(default_system)
    if system is None and default_system:
        system = normalize_system(default_system)
    if system is None and rel_parts:
        cand = normalize_system(rel_parts[0])
        if cand:
            system = cand
    if system is None:
        system = "UNKNOWN"
    if not media_type:
        media_type = variant or ("label" if kind == "label" else "image")
    if not kind:
        kind = "other"

    rec = {
        "path": path,
        "system": system,
        "game": game,
        "kind": kind,
        "media_type": media_type,
        "scale": scale,
        "region": region,
    }
    # Sidecar ScreenScraper meta.json lives next to raw media.
    if path.suffix.lower() == ".json" and path.name.lower() == "meta.json":
        rec["kind"] = "meta"
        rec["media_type"] = "meta"
        extra = parse_ss_meta(path)
        rec.update({k: v for k, v in extra.items() if k != "game"})
        # Keep the folder / file stem (No-Intro name). The SS title is not the ROM name.
    return rec


def parse_ss_meta(path: Path) -> dict:
    out: dict = {}
    if meta_contains_secrets(path):
        return out
    try:
        data = json.loads(path.read_text(encoding="utf-8", errors="replace"))
    except (OSError, json.JSONDecodeError):
        return out
    if not isinstance(data, dict):
        return out
    title = data.get("title") or data.get("romnom") or data.get("name")
    if title:
        out["game"] = str(title)
    # Do not keep a different title as the folder game; the folder name is the ROM / No-Intro name.
    # Prefer the folder name already set; only fill rom_name / ids.
    if data.get("jeuid") is not None:
        out["ss_media_id"] = str(data.get("jeuid"))
    crc = data.get("crc") or data.get("romcrc") or data.get("crc32")
    if crc:
        out["rom_crc"] = str(crc)
    if data.get("romnom"):
        out["rom_name"] = str(data["romnom"])
    return out


def parse_rom_manifest(path: Path) -> list[dict]:
    """Accept TSV: system<TAB>filename  (optional extra columns). Header optional."""
    rows = []
    text = path.read_text(encoding="utf-8", errors="replace")
    for i, line in enumerate(text.splitlines()):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if i == 0 and parts[0].lower() in {"system", "dir", "platform"}:
            continue
        if len(parts) < 2:
            continue
        system, filename = parts[0], parts[1]
        # skip obvious non-rom list rows
        stem = Path(filename).stem
        rows.append({"system": system, "file": filename, "stem": stem})
    return rows


def cmd_init(args: argparse.Namespace) -> int:
    lib = Library(Path(args.root))
    print(f"library ready: {lib.root}")
    lib.close()
    return 0


def cmd_ingest(args: argparse.Namespace) -> int:
    lib = Library(Path(args.root))
    srcs = [Path(s) for s in args.paths]
    total = Counter()
    for src in srcs:
        print(f"ingesting {src}  (source={args.source})")
        try:
            counts = lib.ingest_tree(
                src,
                source=args.source,
                kind_hint=args.kind,
                default_system=args.system,
                default_scale=args.scale,
            )
        except FileNotFoundError as e:
            eprint(e)
            lib.close()
            return 1
        for k, v in counts.items():
            total[k] += v
        print("  " + ", ".join(f"{k}={v}" for k, v in sorted(counts.items())))
    print("total: " + ", ".join(f"{k}={v}" for k, v in sorted(total.items())))
    lib.close()
    return 0


def cmd_export(args: argparse.Namespace) -> int:
    lib = Library(Path(args.root))
    result = lib.export_labels(
        Path(args.manifest),
        scale=args.scale,
        out=Path(args.out),
        fallback=args.fallback,
    )
    print(
        f"exported {result.get('exported', 0)} labels, "
        f"{result.get('gaps', 0)} gaps; report: {result.get('report')}"
    )
    lib.close()
    return 0


def cmd_stats(args: argparse.Namespace) -> int:
    lib = Library(Path(args.root))
    s = lib.stats()
    mb = s["bytes"] / (1024 * 1024)
    print(f"library: {s['root']}")
    print(f"objects: {s['objects']}  unique size: {mb:.1f} MB  records: {s['records']}")
    print("system\tkind\tmedia\tscale\tcount")
    for r in s["rows"]:
        print(
            f"{r['system']}\t{r['kind']}\t{r['media_type']}\t{r['scale'] or ''}\t{r['c']}"
        )
    lib.close()
    return 0


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        prog="artlib",
        description="Private local art library for Slotted (labels, box art, raw media).",
    )
    p.add_argument(
        "--root",
        default=os.environ.get("ARTLIB_ROOT", str(DEFAULT_ROOT)),
        help="library root (default: $ARTLIB_ROOT or the External 4TB path)",
    )
    sub = p.add_subparsers(dest="cmd", required=True)

    s = sub.add_parser("init", help="create the library folder and database")
    s.set_defaults(func=cmd_init)

    s = sub.add_parser("ingest", help="copy art into the library (idempotent, never moves sources)")
    s.add_argument("paths", nargs="+", help="folders, a Labels/ tree, raw/fitted tree, or a .zip")
    s.add_argument("--source", required=True, help="short name for this source, e.g. cartart-test")
    s.add_argument(
        "--kind",
        choices=["label", "raw", "other"],
        default=None,
        help="force kind when the folder layout is unusual",
    )
    s.add_argument("--system", default=None, help="default Slot system folder if it isn't in the path")
    s.add_argument("--scale", default=None, help="default label scale (1x or 2x) if it can't be measured")
    s.set_defaults(func=cmd_ingest)

    s = sub.add_parser("export", help="write a device Labels/ tree from the library (offline)")
    s.add_argument("--manifest", required=True, help="TSV of system<TAB>rom-filename")
    s.add_argument("--scale", required=True, help="1x or 2x")
    s.add_argument("--out", required=True, help="Slot root to write Labels/ into")
    s.add_argument(
        "--fallback",
        action="store_true",
        help="if the requested scale is missing, copy another scale instead of leaving a gap",
    )
    s.set_defaults(func=cmd_export)

    s = sub.add_parser("stats", help="print counts by system and type")
    s.set_defaults(func=cmd_stats)
    return p


def main(argv: Optional[list[str]] = None) -> int:
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
