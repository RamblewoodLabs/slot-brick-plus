# Slot Brick Plus wiki

**Slot Brick Plus** ([jfink22-design/slot-brick-plus](https://github.com/jfink22-design/slot-brick-plus)) is a fork of
[LordNordyx/Slot-Brick](https://github.com/LordNordyx/Slot-Brick). That project ports
[Slot.](https://github.com/slot-cfw/slot), Brandon T. Kowalski's Game Boy frontend for the Anbernic RG SP, to the
TrimUI Brick. This fork adds more systems and shelves, 3D carts, and a build for the **Anbernic RG35XX SP**.

The project's README and `brick/README.md` (the README that goes on the card) explain installation and controls.
This wiki collects the same facts by topic, adds the features still in open pull requests, and lists the people
and projects whose work Slot uses.

## Status labels used on these pages

| Label | Meaning |
| :--- | :--- |
| **On main** | Merged. The feature is in `main` (f753740 when this was written) and in the Dev build. |
| **In PR #N** | Built and in review in that pull request, not merged yet. The wording comes from the PR's changelog line. |
| **Needs confirmation** | Couldn't be confirmed from the code, licences or git history. Don't rely on it until someone checks. |
| **Not found** | Asked about, but not in `main` or any open branch. |

## Pages

- [Devices](Devices.md): TrimUI Brick (tg5040) and RG35XX SP (H700)
- [Installation and Updating](Installation-and-Updating.md)
- [Systems and Cores](Systems-and-Cores.md): every shelf, its core, BIOS files, and the curated N64/DS/PSP lists
- [Shelf UI](Shelf-UI.md): carts, inserting, Game & Watch, Vectrex, filters, favorites
- [Cart Art, Arcade Names and Intellivision Labels](Cart-Art.md)
- [RetroAchievements](RetroAchievements.md)
- [Performance](Performance.md)
- [Settings Reference](Settings-Reference.md)
- [Lid, Sleep and Power](Lid-Sleep-and-Power.md)
- [Developer Notes](Developer-Notes.md): resetting settings and starting fresh
- [Building, Simulator and CI](Building-Simulator-and-CI.md)
- [Credits and Attributions](Credits-and-Attributions.md)

## Open pull requests covered here

| PR | What it adds | Pages |
| :--- | :--- | :--- |
| [#24](https://github.com/jfink22-design/slot-brick-plus/pull/24) | Closing the RG35XX SP's lid sleeps it, and opening the lid wakes it | Lid, Sleep and Power |
| [#27](https://github.com/jfink22-design/slot-brick-plus/pull/27) | Faster All/Favorites switching and menus; achievement cache limited to favorites and the last 20 played; Encore mode; unlock burst | Performance, RetroAchievements |
| [#28](https://github.com/jfink22-design/slot-brick-plus/pull/28) | Real arcade titles from an FBNeo/MAME name table | Cart Art |
| [#29](https://github.com/jfink22-design/slot-brick-plus/pull/29) | SELECT cycles Top 100, Popular, Best Selling and First Party | Shelf UI |
| [#30](https://github.com/jfink22-design/slot-brick-plus/pull/30) | Intellivision labels show the whole end label | Cart Art |
| [#31](https://github.com/jfink22-design/slot-brick-plus/pull/31) | ColecoVision, 5200, Atari 800, Vectrex, 32X, DS, N64 and PSP shelves | Systems and Cores |
| [#32](https://github.com/jfink22-design/slot-brick-plus/pull/32) | Every cart turns at the PSP UMD's pace (~2.6 s); real 3D models for the classic carts; Vectrex powers on. Includes #31's shelves. | Shelf UI |
| [#11](https://github.com/jfink22-design/slot-brick-plus/pull/11) | Companion missions and streaks. Companion is switched off on main. | (not covered) |

These pages cover what each PR does at its current head. Once a PR is merged, change its label to **On main**.
