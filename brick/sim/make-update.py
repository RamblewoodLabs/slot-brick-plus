"""A stand-in release for the simulator's Update screen: a Brick `slot` binary, its
sha256, and the releases list. SLOT_UPDATE_URL points at this folder.
Usage: python3 make-update.py RELEASES_DIR
"""
import hashlib
import os
import sys

root = os.path.abspath(sys.argv[1])
os.makedirs(root, exist_ok=True)
# The simulator reads the card through Docker, where the repo is mounted at /src.
rel = os.path.relpath(root, os.getcwd()).replace(os.sep, "/")
base = "file:///src/" + rel
binary = b"#!/bin/sh\nexit 0\n"
digest = hashlib.sha256(binary).hexdigest()
with open(os.path.join(root, "slot"), "wb") as f:
    f.write(binary)
with open(os.path.join(root, "slot.sha256"), "w") as f:
    f.write(f"{digest}  slot\n")
notes = [
    "- Something new on the shelf, with a line long enough that it has to be wrapped onto a second one.",
    "- A fix for something that was broken.",
    *[f"- Change number {n}, one of many, so the notes run past the bottom of the screen." for n in range(3, 13)],
]
body = "\\n".join(notes)
# The title matches the version, so the screen's first line stays "v9.9 is out".
listing = (
    '[{"tag_name":"v9.9","name":"v9.9","draft":false,"body":"' + body + '","assets":['
    f'{{"name":"slot","size":{len(binary)},"browser_download_url":"{base}/slot"}},'
    f'{{"name":"slot.sha256","size":70,"browser_download_url":"{base}/slot.sha256"}}'
    "]}]"
)
with open(os.path.join(root, "releases.json"), "w") as f:
    f.write(listing)
