"""Slot OS's boot logo, which u-boot shows until Slot's first frame.

The picture is bootlogo-slotted.png: the original pixel "slot" letters with a
red graffiti "ted" tagged on. Written as 1024x768, 32-bit BMP with a V5
header, rows bottom-up (the format TrimUI's u-boot shows). A 1024x768 picture
is 3,145,866 bytes.

Usage:
  python3 bootlogo.py <out.bmp>
  python3 bootlogo.py --check
"""
import struct
import sys
import tempfile
from pathlib import Path

from PIL import Image

W, H = 1024, 768
HEADER = 14 + 124
PIXELS = W * H * 4
SIZE = HEADER + PIXELS  # 3,145,866
SRC = Path(__file__).with_name("bootlogo-slotted.png")


def encode_v5(screen: Image.Image) -> bytes:
    if screen.size != (W, H):
        raise SystemExit(f"boot logo must be {W}x{H}, got {screen.size}")
    screen = screen.convert("RGB")
    pixels = b"".join(
        b"".join(
            bytes((b, g, r, 255))
            for r, g, b in (screen.getpixel((x, y)) for x in range(W))
        )
        for y in range(H - 1, -1, -1)
    )
    if len(pixels) != PIXELS:
        raise SystemExit(f"pixel data is {len(pixels)} bytes, expected {PIXELS}")
    v5 = struct.pack("<IiiHHIIiiII", 124, W, H, 1, 32, 3, len(pixels), 2835, 2835, 0, 0)
    v5 += struct.pack("<IIII", 0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000)
    v5 += b"BGRs" + bytes(36) + struct.pack("<III", 0, 0, 0) + struct.pack("<IIII", 4, 0, 0, 0)
    head = b"BM" + struct.pack("<IHHI", HEADER + len(pixels), 0, 0, HEADER)
    out = head + v5 + pixels
    if len(out) != SIZE:
        raise SystemExit(f"BMP is {len(out)} bytes, expected {SIZE}")
    return out


def render() -> bytes:
    if not SRC.is_file():
        raise SystemExit(f"missing {SRC}")
    return encode_v5(Image.open(SRC))


def check(data: bytes) -> None:
    if data[:2] != b"BM":
        raise SystemExit("not a BMP")
    size, offset = struct.unpack_from("<IHHI", data, 2)[0], struct.unpack_from("<IHHI", data, 2)[3]
    dib, width, height, planes, bpp, compression = struct.unpack_from("<IiiHHI", data, 14)
    if (size, offset, dib, width, height, planes, bpp, compression, len(data)) != (
        SIZE,
        HEADER,
        124,
        W,
        H,
        1,
        32,
        3,
        SIZE,
    ):
        raise SystemExit(
            f"bad header: size={size} off={offset} dib={dib} {width}x{height} "
            f"planes={planes} bpp={bpp} comp={compression} bytes={len(data)}"
        )
    red, green, blue, alpha = struct.unpack_from("<IIII", data, 54)
    if (red, green, blue, alpha) != (0x00FF0000, 0x0000FF00, 0x000000FF, 0xFF000000):
        raise SystemExit(f"bad masks: {red:#x} {green:#x} {blue:#x} {alpha:#x}")


def main(argv: list[str]) -> None:
    if len(argv) != 2 or argv[1] in ("-h", "--help"):
        raise SystemExit("usage: python3 bootlogo.py <out.bmp> | python3 bootlogo.py --check")
    data = render()
    check(data)
    if argv[1] == "--check":
        with tempfile.NamedTemporaryFile(suffix=".bmp") as tmp:
            tmp.write(data)
            tmp.flush()
            check(Path(tmp.name).read_bytes())
        print(f"ok: {SIZE} byte 32-bit V5 {W}x{H} BMP")
        return
    Path(argv[1]).write_bytes(data)


if __name__ == "__main__":
    main(sys.argv)
