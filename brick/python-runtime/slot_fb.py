"""Shows a pygame program's screen on the Brick's framebuffer (/dev/fb0).

Slot's Python runtime (Slot/Apps/python) runs apps with SDL's dummy video driver: the Brick has
no SDL video driver Slot OS could use (Slot itself draws with the GPU's EGL straight onto the
framebuffer). pygame still draws into its display surface as usual; this wraps
pygame.display.flip and pygame.display.update to copy that surface to /dev/fb0 each frame,
scaled to the framebuffer's size if they differ, in its pixel order, at the buffer that is
showing (Slot may have left the display panned to its second buffer).

sitecustomize.py installs it when SLOT_FB is set (run-app.sh sets it to the framebuffer path).
"""
import fcntl
import mmap
import os
import struct

FBIOGET_VSCREENINFO = 0x4600
FBIOGET_FSCREENINFO = 0x4602
FBIOPAN_DISPLAY = 0x4606


class Framebuffer:
    def __init__(self, path):
        self.fd = os.open(path, os.O_RDWR)
        var = bytearray(160)
        fcntl.ioctl(self.fd, FBIOGET_VSCREENINFO, var)
        self.var = var
        (self.w, self.h, self.vw, self.vh, self.xoff, self.yoff, self.bpp) = struct.unpack_from("7I", var, 0)
        red_offset = struct.unpack_from("I", var, 32)[0]
        fix = bytearray(80)
        fcntl.ioctl(self.fd, FBIOGET_FSCREENINFO, fix)
        smem_len = struct.unpack_from("I", fix, 24)[0]
        self.stride = struct.unpack_from("I", fix, 48)[0] or self.w * self.bpp // 8
        self.map = mmap.mmap(self.fd, smem_len, mmap.MAP_SHARED, mmap.PROT_READ | mmap.PROT_WRITE)
        # 32 bpp only (the Brick's): red at bit 16 is BGRA in memory, red at bit 0 is RGBA.
        self.order = "BGRA" if red_offset == 16 else "RGBA"
        # Show the first buffer, and draw there.
        if self.yoff:
            struct.pack_into("2I", var, 16, 0, 0)
            try:
                fcntl.ioctl(self.fd, FBIOPAN_DISPLAY, var)
                self.yoff = 0
            except OSError:
                pass

    def show(self, pygame, surface):
        if surface.get_size() != (self.w, self.h):
            surface = pygame.transform.scale(surface, (self.w, self.h))
        data = pygame.image.tobytes(surface, self.order)
        row = self.w * 4
        base = self.yoff * self.stride
        if self.stride == row:
            self.map[base:base + len(data)] = data
        else:
            for y in range(self.h):
                o = base + y * self.stride
                self.map[o:o + row] = data[y * row:(y + 1) * row]


def install(path):
    import pygame

    state = {"fb": None, "failed": False}
    flip, update = pygame.display.flip, pygame.display.update

    def present():
        surface = pygame.display.get_surface()
        if surface is None or state["failed"]:
            return
        try:
            if state["fb"] is None:
                state["fb"] = Framebuffer(path)
            state["fb"].show(pygame, surface)
        except Exception as e:  # never take the app down for the screen
            state["failed"] = True
            print("slot_fb: cannot draw to %s: %s" % (path, e), flush=True)

    def slot_flip(*a, **k):
        r = flip(*a, **k)
        present()
        return r

    def slot_update(*a, **k):
        r = update(*a, **k)
        present()
        return r

    pygame.display.flip = slot_flip
    pygame.display.update = slot_update
