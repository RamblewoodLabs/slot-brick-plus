#!/usr/bin/env python3
"""Write a 32x32 RGBA PNG icon (no third-party deps) for the Slotted hello OPK."""
from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path


def chunk(tag: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(tag + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)


def main() -> None:
    dest = Path(sys.argv[1] if len(sys.argv) > 1 else "slotted-hello.png")
    w = h = 32
    rows = []
    for y in range(h):
        row = b"\x00"  # filter none
        for x in range(w):
            edge = x < 2 or y < 2 or x >= w - 2 or y >= h - 2
            mid = 10 <= x <= 21 and 8 <= y <= 23
            if edge:
                r, g, b, a = (240, 220, 80, 255)
            elif mid:
                r, g, b, a = (30, 40, 50, 255)
            else:
                r, g, b, a = (20, 140, 150, 255)
            row += bytes((r, g, b, a))
        rows.append(row)
    raw = b"".join(rows)
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    dest.write_bytes(png)


if __name__ == "__main__":
    main()
