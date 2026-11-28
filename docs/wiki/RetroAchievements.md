# RetroAchievements

Slot uses [rcheevos](https://github.com/RetroAchievements/rcheevos) v12.5.0 (MIT), vendored in
`crates/slot-cheevos`.

## Logging in (on main)

**HOME → Online → RetroAchievements** logs in to your [retroachievements.org](https://retroachievements.org) account
and tests the connection. Your password is sent once, over HTTPS, and never stored. Slot keeps only the login token,
in `Slot/Config/retroachievements.txt`. To forget it, choose **LOG OUT** or delete that file.

- **Softcore only.** Save states, rewind and fast-forward stay allowed. Hardcore mode isn't available.
- A toast in the top-right corner shows how many achievements a game has when it starts, and each unlock with its
  badge. Badges are cached in `Slot/System/Cache/Badges`.
- PlayStation `.chd` games are hashed through libchdr.
- **VIEW ACHIEVEMENTS** lists a game's set, even offline.

## Offline play and the set cache

- **On main:** **CACHE ACHIEVEMENT SETS** (needs Wi-Fi) saves **every** game's set and badges into
  `Slot/Config/Achievements`. Away from Wi-Fi, cached sets still unlock. Unlocks are stored with their time and sent
  when Wi-Fi returns, even after a power-off. If the queue file is torn, a spare copy is used. A hardcore unlock is
  never queued, because the server doesn't accept those from an offline session.
- **In PR #27, the cache covers only favorites and the last 20 played:**
  - The cache holds every starred game plus the **20 most recently played** (`RECENT_SET_CACHE = 20`), with badges.
  - It can be resumed: sets already on the card are skipped.
  - Starring a game or starting one queues its set in the background, and the queue runs off the UI thread when Wi-Fi
    is up.
  - Unstarring a game stops prioritizing it but leaves its saved set on the card.

## Encore mode (in PR #27)

**HOME → Online → Encore Mode** is on by default (`ra_encore=1` in `slot.state`). With it on, achievements you've
already earned pop and unlock again when you replay them (`rc_client_set_encore_mode_enabled`). It's still softcore:
the server doesn't count a second award.

## Unlock popup (in PR #27)

An unlock toast has a short burst around the badge (700 ms) and stays up 5.5 s, where it was 4.5 s.
