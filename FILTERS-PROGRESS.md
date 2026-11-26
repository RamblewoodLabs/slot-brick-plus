# Shelf filters (MGT-16) — progress
Worktree ~/sbp-filters, branch cursor/shelf-filters off origin/main f7537404.
Don't touch ~/sbp-perf, ~/sbp-arcade, any card, E:. Don't merge. Never print tokens.
## Steps
- [x] explore code (sort modes, slot.state, platforms, select handling)
- [x] ranking data sources + generator script (brick/make-rankings.py -> crates/slot-store/rankings/*.tsv)
- [ ] implementation + tests
- [ ] previews -> box /workspace/cart-renders/filters-preview/
- [ ] PR + CI green
- [ ] builds + staged /Users/jeff/Shared-Space/sp-update-filters-<sha>
## Notes

- ROM list of the card: ~/Projects/slot-rankings/romlist.tsv (from EVO D:\Temp\rg35xxsp\pr25\plan.csv + D:\Emulation\ROMS GB/GBC/GBA/NES/snes/PSX). Sources in ~/Projects/slot-rankings/src (LaunchBox Metadata.zip, libretro-database sparse, wiki/*.html).
- Regenerate: python3 brick/make-rankings.py --src ~/Projects/slot-rankings/src --romlist ~/Projects/slot-rankings/romlist.tsv --coverage crates/slot-store/rankings/COVERAGE.md
- Design: per-shelf view `view_<key>=` in slot.state (unset = HOME Sort By). Cycle system: A–Z, LAST PLAYED, MOST PLAYED, TOP 100, POPULAR, BEST SELLING, FIRST PARTY (ranked ones skipped when empty; FIRST PARTY skipped when = all). Favorites: ADDED, LAST PLAYED, MOST PLAYED. Pill top-left shows current view always on shelf. Ranked views dedupe same game (same rank) and cap 100; title card gets "#N ".
- Data embedded with include_str! (lazy parse per platform). rank_key mirrored in crates/slot-store/src/rankings.rs.
- First party also from filename tags (GW "(Nintendo, ...)", INTV "(Mattel)").
