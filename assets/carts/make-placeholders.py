#!/usr/bin/env python3
"""Write placeholder cartridge shells under assets/carts/<sys>/.

These stand in until a Blender render replaces shell.png (and mask.png, when
the label is not a plain rectangle). meta.toml is the label window. See
docs/cart-shells.md.
"""
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent


def png(w, h, rgba):
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    raw = b"".join(b"\x00" + bytes(rgba[y * w * 4 : (y + 1) * w * 4]) for y in range(h))
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def inside(x, y, w, h, r, inset=0):
    px, py = x + 0.5, y + 0.5
    x0, y0, x1, y1 = inset, inset, w - inset, h - inset
    if px < x0 or py < y0 or px >= x1 or py >= y1:
        return False
    cx = min(max(px, x0 + r), x1 - r)
    cy = min(max(py, y0 + r), y1 - r)
    dx, dy = px - cx, py - cy
    return dx * dx + dy * dy <= r * r


def shell(w, h, body, well, label, radius=18, well_radius=8):
    buf = bytearray(w * h * 4)
    lx, ly, lw, lh = label
    for y in range(h):
        for x in range(w):
            i = (y * w + x) * 4
            if not inside(x, y, w, h, radius):
                continue
            colour = body
            if inside(x - lx, y - ly, lw, lh, well_radius):
                colour = well
            buf[i : i + 4] = bytes((*colour, 255))
    return buf


def mask(w, h, label, radius):
    buf = bytearray(w * h * 4)
    lx, ly, lw, lh = label
    for y in range(h):
        for x in range(w):
            if inside(x - lx, y - ly, lw, lh, radius):
                i = (y * w + x) * 4
                buf[i : i + 4] = bytes((255, 255, 255, 255))
    return buf


def write(name, w, h, body, well, label, mask_radius=None, shell_radius=18):
    d = ROOT / name
    d.mkdir(parents=True, exist_ok=True)
    lx, ly, lw, lh = label
    aspect = round(lw / lh, 4)
    (d / "shell.png").write_bytes(png(w, h, shell(w, h, body, well, label, shell_radius)))
    lines = [
        f"# {name} placeholder. Replace shell.png with the Blender render.",
        f"# Shell is {w}x{h} RGBA. Label rect is in those pixels, origin top-left, y down.",
        f"x = {lx}",
        f"y = {ly}",
        f"w = {lw}",
        f"h = {lh}",
        "rotation = 0.0",
        f"aspect = {aspect}",
        "",
    ]
    if mask_radius is not None:
        (d / "mask.png").write_bytes(png(w, h, mask(w, h, label, mask_radius)))
        lines.insert(-1, "# mask.png: label alpha. 0 hides the label (the shell shows through).")
    else:
        stale = d / "mask.png"
        if stale.exists():
            stale.unlink()
    (d / "meta.toml").write_text("\n".join(lines))


def main():
    write("md", 360, 200, (0x1A, 0x1A, 0x1C), (0xC8, 0xB8, 0x78), (40, 28, 280, 128), shell_radius=16)
    write("sms", 240, 320, (0x1C, 0x28, 0x38), (0xD8, 0xD2, 0xC4), (32, 48, 176, 200), shell_radius=16)
    write("gg", 200, 280, (0x20, 0x20, 0x24), (0x3A, 0x6E, 0xA8), (28, 64, 144, 144), shell_radius=14)
    write("arcade", 360, 140, (0x3A, 0x12, 0x18), (0xF2, 0xE2, 0xC0), (18, 16, 324, 108), mask_radius=22, shell_radius=12)
    write("pce", 320, 180, (0xF2, 0xF0, 0xE8), (0xE4, 0x3A, 0x3A), (28, 22, 264, 124), shell_radius=10)
    write("a2600", 220, 340, (0x8A, 0x58, 0x2C), (0xF0, 0xE6, 0xC8), (36, 70, 148, 180), mask_radius=14, shell_radius=14)
    write("intv", 240, 320, (0xC4, 0xA3, 0x5A), (0xF6, 0xF1, 0xE4), (30, 40, 180, 210), shell_radius=16)
    write("lynx", 280, 160, (0x4A, 0x20, 0x60), (0xE8, 0xD8, 0xF0), (24, 22, 232, 108), shell_radius=12)
    write("a7800", 240, 300, (0x2A, 0x2A, 0x2E), (0xD0, 0xD4, 0xDC), (28, 36, 184, 200), shell_radius=14)


if __name__ == "__main__":
    main()
