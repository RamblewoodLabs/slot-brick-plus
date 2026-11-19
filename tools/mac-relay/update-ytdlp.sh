#!/bin/bash
# Daily: Homebrew's yt-dlp, or yt-dlp -U when the binary updates itself.
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
if command -v brew >/dev/null 2>&1 && brew --prefix yt-dlp >/dev/null 2>&1; then
    brew upgrade yt-dlp
else
    yt-dlp -U
fi
