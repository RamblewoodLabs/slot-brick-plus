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
