# Continue anywhere

Saves follow Jeff's handhelds around the house. A Brick and, later, an Anbernic RG35XX SP
running this same Slot tree play the same game. The Mac mini stays on and holds the newest
copy. Take a device out, play, come home, and when Wi-Fi is back the device uploads. The
other device pulls that copy the next time it connects or a game is launched.

This is our own hub and client. It is not Syncthing, and it is not the old download relay.
Nothing here reuses that relay's names, port, or folder.

The hub listens on the home LAN only. Do not forward its port to the internet.

## What syncs

Paths are relative to the `Slot/` folder on the card, and the same relative paths inside
the hub's store. A name maps across devices because both cards use these folders:

| What | Path | Why |
| :--- | :--- | :--- |
| Save RAM | `Saves/<system>/<game>.sav` and `.srm` | The in-game save. `.sav` is what Slot writes. `.srm` is read if a `.sav` is not there yet. |
| Resume | `States/<system>/<core>/<game>/resume.state` | What Continue loads. |
| Latest save state | `States/<system>/<core>/<game>/<stamp>.state` and its `.png` | The newest stamped slot only. The rest of the ten-slot ring stays on the device that made it. |
| Last played | `Config/library.txt` | Play time, last played, and favorites. One line a game. |
| Which game to highlight | `Config/continue.txt` | Written when a game is ejected or the lid closes. The other device selects that game on the shelf. It does not start the game by itself. |
| Core choice | `Config/selected_core.ini` | So both devices load a game with the same core. |
| RetroAchievements progress | `Config/Achievements/queue.txt`, `sets/<id>.json`, `sets/<id>.list`, `sessions/<id>.json` | Unlocks waiting to be sent, and the sets and sessions already saved on the card. |

`<system>` is `GB`, `GBC`, `GBA`, `NES`, `SNES`, or `PS1`. `<core>` is `mgba`, `gpsp`,
`fceumm`, `snes9x`, or `pcsx_rearmed`. `<game>` is the cart's stem, the same text on both
cards when the ROM file has the same name.

Not synced:

- `Config/sync.txt` (it holds the token).
- `Config/slot.state` (volume, brightness, clock, and the rest of that device's own settings).
- `Config/Achievements/login.json` (the RetroAchievements token). Each handheld logs in on its own.
- Anything ending in `.bak`, and any path that climbs out of `Slot/` with `..`.

## Conflict rules

The newest modification time wins. Times are compared in nanoseconds. If the bytes are
identical, nothing is written.

If the times are equal and the bytes differ, the greater device id wins, compared as text.
The same device id sending again replaces its own tie.

The copy that loses is renamed to the same path with `.bak` added, once. The next conflict
replaces that `.bak`. `.bak` files are not uploaded.

Each device keeps an id of its own in `Config/sync.txt` (`device=`). The install notes tell
you to copy the host, port, and token, and to leave `device` empty so the handheld fills in
its own id the first time it reads the file.

## Discovery

`Config/sync.txt` may name the hub:

```
host=Jeffs-Mac-Mini-2.local
port=8791
token=<shared token>
device=
```

`port` defaults to 8791. `host` may be a `.local` name or `192.168.7.99`.

If `host` is empty, the handheld asks the LAN, once, for `_slotsync._tcp.local` and uses
the first answer. The hub advertises that name while it is running. The query gives up
after a third of a second.

## Auth

Every request sends `Authorization: Bearer` with the token from `sync.txt`. The hub
compares it in constant time. A wrong token is refused. The token is never written to a
log, a status line, or an error message. Logs name the request path only.

The token is a shared secret for Jeff's own LAN. It is not an account.

## When it runs

- Wi-Fi comes up.
- A game is ejected.
- Slot boots.
- The lid closes, or Slot is about to sleep.

Each of those starts a short background pass (about three seconds). It does not wait on
the menu.

Launching a game pulls that game's save RAM and `resume.state` first, then starts the
core. Connecting gives up after about a third of a second, and the whole pull gives up
after about a second. If the hub is off, asleep, or off the LAN, the game starts from the
copy already on the card.

## Offline

No hub means no change on the card. Play, save, and sleep work as they do now. The next
time the hub answers, the handheld uploads what it did while it was away and downloads
anything newer.

System → Continue is missing until `Config/sync.txt` exists and has a token. With the file
in place the row reads Waiting, Looking, Hub not found, Up to date, Sent N, got M, or
Refused. Refused means the token did not match. The row does not show the token.

## Cores and save names

A save state from mGBA will not load in gpSP, and a state from one mGBA build can fail on
another. The Brick and the RG35XX SP have to ship the same core files and the same Slot
state format. The path already carries the core name, so a state made with `mgba` is only
offered back to `mgba`.

Save RAM is the `.sav` beside the game's stem. Both devices must spell the ROM's name the
same way (`Emerald.gba` on both cards becomes `Saves/GBA/Emerald.sav` and
`States/GBA/mgba/Emerald/resume.state`). Renaming a ROM on one card and not the other
makes them look like two games.

`Config/selected_core.ini` is synced so the core Slot picks for a stem stays the same on
both devices. Keep that file's core, and keep the core binaries in lockstep when the SP
build is made.

## The store

The hub keeps files at:

`~/Library/Application Support/SlotSync/files/<the same relative path>`

`index.json` next to `files/` lists each path with its time, device id, sha256, and size.
A later iOS or Mac app can read that folder and that index without talking to Slot. The
HTTP API is only how a handheld reaches the same bytes.

`config.json` in that folder holds the token, port, and bind address. It is not inside
`files/`, and it is not copied to a card.

## Menu bar

`install.sh` also builds a small menu-bar app with `swiftc` and puts it at
`~/Applications/Slot Sync.app`. LaunchAgent `com.slot.sync.menubar` opens it at
login. The icon is a monochrome sync arrow: filled while the hub is running, dimmed
and slashed when it is off. It does not show in the Dock (`LSUIElement`).

The menu says whether the hub is on, the port (8791), and the Mac's `.local` name.
Under that, each handheld's device id, the game it last synced, and the time. Turn On
and Turn Off use `launchctl` on `com.slot.sync` only. Open Sync Folder opens the store.
Quit closes the menu bar app and leaves the hub running.

The app never opens `config.json` and never sends or logs the token. While the hub is
up it reads `http://127.0.0.1:8791/status`, which needs no token and does not return
one. If the hub is off, it reads `index.json` in the store, which also has no token.
