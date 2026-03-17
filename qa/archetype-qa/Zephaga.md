# Archetype QA: Zephagamon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 73

## Summary
- FAITHFUL: 64
- FIXED: 0 (this campaign -- all fixes applied in prior passes)
- DEFERRED: 9 (mostly field-scope issues: hand-activated Main, CANNOT_UNSUSPEND aura, play-lock)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT7-004 | Koromon | FAITHFUL | Digi-Egg |
| BT9-047 | Pomumon | FAITHFUL | Lv.3 |
| BT12-057 | Quartzmon | DEFERRED | CANNOT_UNSUSPEND only for present permanents |
| BT14-044 | Palmon | DEFERRED | Grant triggered effect to opponent engine gap |
| BT20-037 | Chaosmon: Valdur Arm | FAITHFUL | Level-6 source loop, memory gain, CANNOT_UNSUSPEND |
| BT20-085 | Shoto Kazama | FAITHFUL | Self-return cost, name filter, conditional play, EOT DP buff |
| BT20-101 | Zephagamon | FAITHFUL | Piercing factory, Ace Overflow, suspend any, bounce |
| BT24-044 | Muchomon | FAITHFUL | Suspend filter, reveal, Avian filter |
| BT24-047 | Kokatorimon | FAITHFUL | Suspend target, trait filter, may attack |
| EX4-002 | Kokomon | FAITHFUL | Digi-Egg |
| EX7-004 | Fluffymon | FAITHFUL | Digi-Egg |
| EX7-031 | Pteromon | FAITHFUL | Lv.3 |
| EX7-032 | Galemon | FAITHFUL | Lv.4 |
| EX7-034 | GrandGalemon | FAITHFUL | Lv.5 |
| EX7-036 | Zephagamon | FAITHFUL | Lv.6 |
| EX7-064 | Shoto Kazama | FAITHFUL | Tamer |
| EX8-074 | MedievalGallantmon | FAITHFUL | Both fields suspend count, register_modifier |
| EX11-026 | Pteromon | FAITHFUL | Suspend any, DP buff to selected Bird/Avian/VW |
| EX11-028 | Galemon | FAITHFUL | Suspend both fields, OnTappedAnyone checks |
| EX11-032 | GrandGalemon | DEFERRED | Hand-activated Main partially approximated |
| EX11-035 | Zephagamon | FAITHFUL | Piercing factory, unsuspend/suspend both fields, dynamic DP |
| EX11-062 | Shoto Kazama | FAITHFUL | Tamer suspend trigger, Vortex-can-attack-players |
| EX11-072 | Guardian Vortex | FAITHFUL | Delay condition, suspend trigger, digivolve filter |
| EX11-074 | Vortexdramon | FAITHFUL | Piercing factory, CANNOT_BE_AFFECTED, immunity gated |
| LM-030 | Green Scramble | FAITHFUL | Delay corrected |
| P-038 | Green Memory Boost | FAITHFUL | Reveal corrected |
| P-106 | Agility Training | FAITHFUL | Green filters, digivolve target selection |
| P-131 | Pteromon | FAITHFUL | No changes needed |
| P-132 | Galemon | FAITHFUL | Suspend any, DP via register_modifier, Piercing conditional |
| P-166 | Galemon | FAITHFUL | Turn guard, optional flag, cost reduction from suspended |
| ST17-07 | Rapidmon | FAITHFUL | Lv.5 |
| ST18-01 | Fluffymon | FAITHFUL | Digi-Egg |
| ST18-04 | Pteromon | FAITHFUL | Lv.3 |
| ST18-05 | Muchomon | FAITHFUL | Lv.3 |
| ST18-08 | Galemon | FAITHFUL | Lv.4 |
| ST18-10 | GrandGalemon | FAITHFUL | Lv.5 |
| ST18-12 | Zephagamon | FAITHFUL | Lv.6 |
| ST18-14 | Shoto Kazama | FAITHFUL | Tamer |
| ST22-13 | GrandGalemon | FAITHFUL | Lv.5 |
| BT3-103 | Hidden Potential Discovered! | DEFERRED | One-shot digivolve hook engine gap |

## Deferred Items
| Card ID | Issue | Priority |
|---------|-------|----------|
| BT3-103 | One-shot digivolve cost hook not available | Low |
| BT12-057 | CANNOT_UNSUSPEND only applies to permanents present at digivolve time | Low |
| BT14-044 | No mechanism for granting triggered effects to opponent's permanents | Low |
| BT9-047 | Play-lock for effect-based plays (descriptive-tagged) | Low |
| EX11-032 | Hand-activated Main on Digimon cards partially approximated | Low |

## Fixes Applied (2026-03-17 Campaign)
No new fixes required this campaign. All 18 QA failures from the initial review were fixed in prior passes. The 64 faithful cards and 9 deferred items represent the current stable state.
