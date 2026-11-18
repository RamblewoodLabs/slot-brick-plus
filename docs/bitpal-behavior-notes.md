# Companion behavior notes

These notes describe how the companion in PakUI behaves, rewritten from reading its scripts. They are not a copy of that project. Slot's companion uses its own name, pictures, and sentences.

## What it stores

A small text record holds the companion's level, the XP sitting inside the current level, how much XP the next level costs, a mood name, the last time the screen was opened, and how many missions have been finished. Each active mission is its own one-line record: a description, a game or system, a kind, a length in minutes, an XP reward, when it was accepted, the game file it points at, and how many seconds have been counted so far. Finished missions are appended to a history list. Up to five missions can be active at once. A separate session file remembers a game that was still running if the machine stopped, so the next open can add that time.

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
