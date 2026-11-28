# Lid, Sleep and Power

## TrimUI Brick (on main)

| Action | What happens |
| :--- | :--- |
| Press Power | Sleep or wake (deep sleep, with low battery drain) |
| Hold Power | Shut down, or exit to the stock OS. On Slot OS it shuts down. |
| Idle | Auto-sleep after 5 minutes in menus or 15 minutes in a game. To turn it off, create `Slot/DISABLE_AUTO_SLEEP`. |
| Low battery | F1/F2 turn red at ≤15% and blink at ≤10%. At the **Low Battery Shutdown** level (5% by default), Slot saves and shuts down. |

Sleep is based on the pinned NextUI `tg5040` suspend reference (see Credits). Slot's own helper makes one attempt,
rolls back services, and falls back to standby.

## RG35XX SP

- **On main:** closing the lid asks the kernel for suspend-to-RAM. Holding Power opens a menu to shut down or restart.
  There's no stock launcher to exit to.
- **In PR #24 (the lid wakes the SP):** BaseOS 1.4.0 sets `axp2202-battery/os_sleep` to `16` (Super Standby) at boot.
  That mode wakes on Power but ignores the hall sensor, so opening the lid used to come back to a black screen.
  The SP's own `Slot/System/sleep.sh` clears that setting for the sleep, arms the hall, lid and Power wakeup files,
  and asks for `mem` only when one of those was armed.
  - Closing the lid sleeps the SP, in a game or on the shelf. Opening the lid, or pressing Power, comes back to the
    same place.
  - If deep sleep can't be woken by the lid, the screen goes dark and the game stays paused until the lid opens or
    Power is pressed (standby, exit code 2).
  - Each cycle is logged to `Slot/Logs/sleep.log` (`mode=mem` or `mode=standby`).
  - **Needs confirmation:** PR #24 hasn't been confirmed on SP hardware in these notes. Check `sleep.log` after testing.

After installing a new build on either device, **power fully off and cold-boot**. Waking from sleep doesn't load the
new binary.
