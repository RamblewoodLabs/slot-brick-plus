# Slot download relay

A small server for a Mac mini on the same Wi-Fi as the Brick. Slot asks it to
fetch a video or a track with yt-dlp. The Brick then copies the finished file,
checks its size and sha256, and tells the Mac to delete it. A download that is
never collected is deleted after 24 hours. Those files live in `downloads/`
inside the relay folder. The sweep does not touch `config.json` or
`update-ytdlp.sh`. If the Mac is off or the download
fails, Slot uses YouTube directly, as it did before.

The relay listens on port **8787** and expects a shared token.

## Install on the Mac mini

1. Install [Homebrew](https://brew.sh) if it is not there already.
2. In a clone of this repo, run:

```bash
bash tools/mac-relay/install.sh
```

That installs `ffmpeg` and `yt-dlp` with Homebrew, writes
`~/Library/Application Support/SlotRelay/config.json`, and loads two launchd
jobs: `com.slot.relay` (the server) and `com.slot.relay.ytdlp` (a daily
`brew upgrade yt-dlp`, or `yt-dlp -U` when Homebrew does not own the binary).

3. The script prints three lines. Put them in `Slot/Config/videos.txt` on the
   card:

| Key | Meaning |
| --- | --- |
| `relay_host` | The Mac's Bonjour name (`LocalHostName.local`) or a manual IP such as `192.168.1.20`. If this line is missing and a token is set, Slot uses `Mac-mini.local`. |
| `relay_port` | TCP port. Default `8787`. |
| `relay_token` | The same token as `config.json`. While this is empty, Slot does not try the relay. |

Example:

```text
relay_host=Mac-mini.local
relay_port=8787
relay_token=paste-the-token-from-install
```

4. Allow incoming connections if macOS asks. The Brick and the Mac must be on
   the same LAN.

To stop it:

```bash
bash tools/mac-relay/uninstall.sh
```

## What the Brick sees

Downloads says **Relay downloading…** while yt-dlp is fetching, then
**Transferring…** while the file is copied. `Logs/videos.log` records
`source relay` or `source direct`.

Video files are H.264 + AAC in an MP4, at most 360p (YouTube itag 18 when
yt-dlp can get it). Music files are AAC in an M4A.

## Checks

```bash
python3 -m unittest discover -s tools/mac-relay -v
```

## Menu bar switch

`menubar/` holds **Slot Relay.app**, a small menu bar item (no Dock icon) that
shows whether the relay is on and turns it on or off.

```bash
bash tools/mac-relay/menubar/install.sh     # build, copy to ~/Applications, start at login
bash tools/mac-relay/menubar/uninstall.sh   # remove it; the relay is left as it is
```

- The icon is a filled TV with a play mark when the relay answers on its port,
  an outline TV while launchd is starting it, and a crossed-out TV when it is off.
- The menu shows the state, the port (read from `config.json`), and the Mac's
  LAN address and `.local` name, then **Turn On** / **Turn Off**, the logs and
  relay folders, and Quit.
- **Turn Off** runs `launchctl disable` and `bootout` for `com.slot.relay`, so
  the relay stays off across restarts until **Turn On** (`enable`, `bootstrap`,
  `kickstart`). `com.slot.relay.ytdlp` is not touched.
- The app checks every 5 seconds with a request that has no token; the relay's
  401 answer is enough to know it is serving. It never reads the token.
- A LaunchAgent, `com.slot.relay.menubar`, starts it at login.
