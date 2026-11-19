# Releasing Slot for this Brick

HOME → Online → Update only looks at [jfink22-design/slot-brick-plus](https://github.com/jfink22-design/slot-brick-plus) releases. It never asks any other project. A release it will install has two files:

- `slot` — the Brick binary, the same file that lives at `Slot/System/slot`
- `slot.sha256` — the checksum, one line from `sha256sum` (`<hex>  slot`)

The tag has to be a version, `vX.Y` or `vX.Y.Z`. Update offers it only when that version is newer than the one on the card. The release's title and its notes are what the Update screen shows. A draft is skipped. The release must not be a draft when you publish it.

Update checks the checksum before it replaces `Slot/System/slot`. The previous binary is kept as `Slot/System/slot.bak`. If the new one never shows a frame, the card puts `slot.bak` back and starts it.

## From a tag, on GitHub

Pushing a `vX.Y` tag on `main` runs `.github/workflows/brick-binary.yml`. That builds the Brick binary in Docker and publishes `slot` and `slot.sha256` on the release for that tag, with the changelog section for the tag as the notes.

Before the tag:

1. Put the version at the top of `brick/README.md`, in `brick/card/Apps/Slot/config.json`, `brick/card/Slot/System/launch.sh`, `brick/card/Slot/System/diagnose.sh`, and `PORT_VERSION` in `crates/slot-ui/src/sticker.rs`.
2. Move `## Unreleased` in `brick/CHANGELOG.md` under `## vX.Y`, and leave an empty `## Unreleased` above it. Those notes are the release text.
3. Merge that as a pull request named `Release vX.Y`.
4. `git tag vX.Y && git push origin vX.Y`, with the tag on `main`.

The workflow only runs in `jfink22-design/slot-brick-plus`.

To publish the same files for a tag that already exists, run the **Brick binary** workflow by hand and give it the tag. It uploads `slot` and `slot.sha256` onto that release.

## From this machine

Docker and the repo's build image are enough. From the repository root:

```sh
sh brick/publish-slot.sh dist-release
```

That writes `dist-release/slot` and `dist-release/slot.sha256`. Then, with the `gh` command logged into this repository:

```sh
notes=$(bash brick/release-notes.sh vX.Y)
gh release create vX.Y dist-release/slot dist-release/slot.sha256 --title "vX.Y" --notes "$notes"
```

If the release is already there, upload the two files onto it:

```sh
gh release upload vX.Y dist-release/slot dist-release/slot.sha256 --clobber
```

The title (`--title`) is the line the Update screen shows beside the version. The notes are the lines under it. Leave the release published, not a draft.
