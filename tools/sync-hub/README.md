# Slot sync hub

A small always-on store for continue-anywhere saves. It runs on the Mac mini and
holds the newest save RAM, resume state, and last-played record for the Brick and
the RG35XX SP. See `docs/sync.md` for the rules.

```sh
bash tools/sync-hub/install.sh
```

That writes `~/Library/Application Support/SlotSync/config.json` (token, port 8791),
loads the LaunchAgent `com.slot.sync`, and builds the menu bar app
`~/Applications/Slot Sync.app` (LaunchAgent `com.slot.sync.menubar`). The icon shows
whether the hub is running. Turn On and Turn Off affect `com.slot.sync` only. The app
does not read the token. It prints the `host`, `port`, and `token` lines to put in
`Slot/Config/sync.txt` on each handheld. Leave `device` blank.

```sh
bash tools/sync-hub/uninstall.sh
```

Both launchd jobs go away, and `~/Applications/Slot Sync.app` is removed. The save
store stays, so a later iOS or Mac app can still read `files/` and `index.json`.

```sh
python3 -m unittest discover -s tools/sync-hub -v
```
