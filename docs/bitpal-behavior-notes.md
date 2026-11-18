# Companion behavior notes

These notes describe how the companion in PakUI behaves, rewritten from reading its scripts. They are not a copy of that project. Slot's companion uses its own name, pictures, and sentences.

## What it stores

The companion record itself is one text file:

| Field | Meaning |
| --- | --- |
| name | Display name |
| level | Current level |
| xp | XP still inside this level, not the lifetime total |
| xp_next | XP required to reach the next level |
| mood | happy, excited, neutral, sad, angry, or surprised |
| last_visit | Unix time the screen was last opened |
| missions_completed | How many missions have ever been finished |

Each active mission is a separate file, one slot, up to five. A line holds: the sentence shown to the player, a short game name, the kind of mission, the length in minutes, the XP reward, the unix time it was accepted, the game path, and the seconds counted so far. A marker file beside it means the mission already reached its time and should be paid on the next open.

Finished missions go in one append-only list. Each line is the sentence, the unix time it started, the unix time it finished, and the XP paid.

A session file holds the game path and the seconds of a launch that never got a clean ending. A one-line file holds the length of the latest finished launch.

The play log, shared with the time tracker, is one row per game: a label, the game path, the system, how many times it was launched, the total seconds, a short time string, a marker that it is a launch row, and the unix time of the last session. Rows are kept sorted by total time.

A streak file holds the last day played, the current run of days, the longest run, how many days have had play, and the day the current run started.

Those files are rewritten in place. A power loss in the middle of a rewrite can leave a partial file, and there is no second copy.

## What Slot stores

Slot keeps one versioned file, `Config/companion.txt`, plus `Config/companion.txt.bak`. The header is `slot-companion 2`. A checksum line rejects a torn or edited file. A good file is copied to the backup before the next save. The save itself is a new temp file, flushed, then renamed, and the folder is flushed too. If the main file fails the checksum, the backup is read and written back. A version 1 file is still accepted and saved as version 2.

The file holds every fact above, and more:

| Record | Fields |
| --- | --- |
| Companion | switches, lifetime XP, next mission id, mood, last visit day, missions finished |
| Per game | system, name, launch count, total seconds, last day, last session length, favorite |
| Per system | launch count, total seconds, last day, how many games have time |
| Streak | last day, current run, longest run, days with play, day the run started |
| Session | system, game, seconds, day, including one left open by a crash |
| Mission | id, kind, words, system, game, seconds required, seconds counted, XP, done, extra |
| History | words, day, XP, system, game. The list is trimmed; the finished count is not |
| Also | first-play awards, systems the player turned off, the suggested game |

Favorites and per-system totals are not separate files in the other project. Slot keeps them in this record so a crash cannot lose them independently of the play log.

## Missions

The player asks for a mission and is offered a short menu, then picks one. The menu is built like this:

- If the play log has more than a couple of launches, one offer is a game from that log. Fewer than three recorded launches makes it a shorter return visit (about 5 to 14 minutes). Three or more makes it a longer return (about 10 to 24 minutes).
- One offer is a system chosen at random. The player then picks any game on that system. Length is about 5 to 19 minutes.
- One offer is one game chosen at random. Length is about 5 to 19 minutes.
- One offer is a surprise game chosen for the player. Length is about 10 to 24 minutes.
- If nothing else could be built, the offer is simply to play any game for about 5 to 19 minutes.

The companion's own folder and a few utility folders are not treated as systems. Time counts only while the mission's game is the one running. A system mission counts any game on that system. When the counted seconds reach the minutes, the mission is finished, its XP is granted, and the slot is freed. Progress is kept if you leave and come back. A crash is recovered from the session file on the next open.

Rewards, per minute of the mission:

- a game from the play log, or "any game": 5 XP a minute
- a system: 6 XP a minute
- a named random game: 7 XP a minute
- a surprise game: 8 XP a minute

## Levels

A new companion is level 1 and needs 100 XP. When the XP in the level reaches the cost, that cost is spent, the level goes up by one, and the next cost becomes `level * 50 + 50` using the new level. So the steps are 100, then 150, then 200, and so on. Crossing a level shows a separate notice after the mission notice.

## Mood

The face has happy, excited, neutral, sad, angry, and surprised. Finishing a mission walks the mood upward: sad becomes neutral, neutral or surprised becomes happy, angry becomes neutral, and happy sometimes becomes excited. A level-up is excited. Cancelling a mission is usually sad and sometimes angry. Leaving the screen, unless a mission was finished in the last five minutes, picks sad, angry, or surprised and asks you to stay; staying softens that to neutral or surprised. Three days away sets neutral. Opening the screen while already sad or angry has a chance to soften it.

## Suggestions, sessions, screens

There is no separate "recommended game" beside the missions. The offers are the suggestions. Play time is tracked per game for the mission and also in a general play log. The home screen shows the face, a greeting, and sometimes a trivia line, then a menu: status, start a mission, history. Status shows level, XP toward the next level, mood, finished count, and the active missions. History lists what was finished. You can look at one mission's progress or cancel it. Two small extra games exist on that menu; they are not part of the mission rules.

## What Slot does not copy

The other project's pictures, greetings, trivia lines, and pleading exit lines are its own writing and art. Slot keeps an original face and its own short sentences. Slot also has no separate game-switcher switch to turn off during a mission, and it does not include those two extra games.
