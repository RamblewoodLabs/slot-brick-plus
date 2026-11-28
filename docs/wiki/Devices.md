# Devices

One Slot codebase builds for two kinds of handheld. Both draw the same 640×480 layout.

| | TrimUI Brick / Brick Hammer (and Brick Pro) | Anbernic RG35XX SP |
| :--- | :--- | :--- |
| SoC / platform | Cortex-A53 with PowerVR GE8300, platform name `tg5040` (the repo doesn't name the SoC; A133P is **needs confirmation**) | Allwinner H700 |
| Screen | 1024×768. The 640×480 layout is scaled up by exactly 8/5. | 640×480, drawn 1:1 |
| GPU path | PowerVR, null window, SDL joystick | Mali fbdev window, evdev pad |
| Operating system | TrimUI stock firmware, or **Slot OS** (experimental, flashed to the card) | **BaseOS 1.4.0** by pvaibhav on TF1 |
| Where Slot lives | One card: `Slot/` and `.slot-boot` at the root | TF2 (FAT32, MBR): `System/launch_frontend.sh` and `Slot/` |
| Cargo feature | `brick` | `rg35xxsp` (turns on the Brick software plus the H700 device path) |
| HOME → Online → Update | Yes, signed | **No.** That row downloads the Brick binary. To update the SP, copy a new `Slot/System` onto TF2. |
| Lid | none | Hall sensor. Lid sleep and wake are in [PR #24](https://github.com/jfink22-design/slot-brick-plus/pull/24). See [Lid, Sleep and Power](Lid-Sleep-and-Power.md). |
| Overlays | 1024×768 PNGs; KrutzOtrem's sets come bundled | 640×480 PNGs; none come bundled |
| LEDs | F1/F2, top bar, buttons and triggers, plus sticks on the Brick Pro | none |

Sources: `docs/rg35xxsp.md` and `README.md`.

## TrimUI Brick notes

- **Brick Pro.** Slot detects it automatically. The left stick works as the D-pad, and HOME → System → LEDs gains a
  **Sticks** row. On Slot OS the Pro needs its own image (`slot-os-brickpro-…`), which is experimental and so far
  reported working by one player.
- **Slot OS** starts in about 4 seconds, where stock firmware takes about 9. It runs TrimUI's boot chain, kernel and
  firmware 1.1.1 with Debian 12 packages. The licences for these are in `slot-os/licenses` on a Slot OS card.

## RG35XX SP notes

- TF1 holds BaseOS 1.4.0 (`baseos-rg35xxsp-1.4.0.img.zip`, sha256 `0d93edff…7b626d`).
  **Never format or reflash TF1** after its first boot. If Windows or macOS offers to format a partition on it,
  click **Cancel**.
- TF2 holds Slot and the library. A Brick card's `Slot` folder drops onto it with
  `rg35xxsp/copy-from-brick.sh`, which skips `System/`, `Overlays/`, logs and the `brick_*` config files.
- Save states load on either device, provided both run the same core builds (the pinned commits in
  [Systems and Cores](Systems-and-Cores.md)).
- Why this isn't based on pvaibhav's Slot 1.16 build: that build's default GBA core is armGBA, so the Brick's states
  wouldn't load, and it uses upstream Slot's card layout. See `docs/rg35xxsp.md`.
