# Slotted on the Anbernic RG280V

The user-facing name on this device is **Slotted**. The Brick and RG35XX SP builds stay Slot. See [docs/rg280v/PLAN.md](../docs/rg280v/PLAN.md) for the port plan.

This folder is the OpenDingux (gcw0 / JZ4770) build. It does not change the Brick or SP toolchains, and it does not flash a card or download BIOS, keys, or firmware.

## Build the hello OPK

On the Mac mini, with Docker Desktop running:

```sh
sh rg280v/docker-build.sh
```

The result is `dist-rg280v/slotted-hello.opk`, also copied to `/Users/jeff/Shared-Space/rg280v/` when that folder exists.

The hello OPK tries SDL2 with an OpenGL ES 2 context first (KMS / Etnaviv on OpenDingux Beta). If that fails, it falls back to SDL’s software renderer. Face buttons, shoulders, and the D-pad are drawn on screen. Press **Start and Select** together to quit.

## Putting it on the device later

Do not flash a card until Jeff says yes. When there is a bootable Adam v2.1 card in TF1, copy `slotted-hello.opk` into the `apps` folder on TF2 and launch it from GMenu2X or SimpleMenu. Nothing in these scripts writes to a microSD card.

The Rust target is `mipsel-gcw0-linux-uclibc` (custom JSON in this folder). It is MIPS32r2, uClibc, and hard-float **FPXX**, matching the official gcc 11 toolchain. Nightly plus `-Z build-std` is used only inside the Docker image; the repo-root 1.96 pin for Brick and SP is unchanged.

The hello OPK has been cross-compiled but not run on the handheld. GLES2 versus the software fallback, screen orientation, and the button map (especially L2/R2) still need a bootable Adam card in TF1.



## Build the Slotted shell OPK

```sh
sh rg280v/docker-build.sh shell
```

This produces `dist-rg280v/slotted.opk`, also copied to `/Users/jeff/Shared-Space/rg280v/` when that folder exists. The on-device name is **Slotted**. The 640×480 layout is shown as a sharp 320×240 picture, letterboxed on the 320×480 (or 480×320) panel. Orientation is set at runtime: `SLOTTED_ORIENTATION` or `Slot/Config/orientation` (`auto`, `portrait`, `landscape`, `portrait-flip`, `landscape-flip`, `rotate-90`, `rotate-270`) until the device confirms the framebuffer.

N64, DS and PSP shelves are hidden. Starting a game looks for an OpenDingux standalone OPK (or a RetroArch core) using `launchers.ini`. Override that table with `Slot/Config/launchers.ini` on TF2. The insert-animation frames are stored at half resolution in memory, and only the selected cart's sequence is kept, for a 512 MB budget.

The shell needs OpenGL ES 2 (KMS / Etnaviv). The hello OPK is still the place to test the software-renderer fallback. Nothing here flashes a card or downloads BIOS, keys, or firmware.
