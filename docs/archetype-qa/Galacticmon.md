# Archetype QA: Galacticmon
Date: 2026-03-14 (updated)
Total cards: 36

## Summary
- PASS: 24
- IMPLEMENTED: 6 (BT18-060, BT18-065, BT18-092, BT7-105, ST13-08, LM-048)
- QA-FAIL -> FIXED: 14 (prior pass) + 5 (this pass)
- BLOCKED: 0 (EX11-042 and P-094 redirect_attack now use game.switch_attack_target)

## Fixes Applied (2026-03-14 Pass)

### BT11-070 (Destromon) - FIXED (was completely wrong)
**Effect 1 (When Digivolving):** Was deleting opponent Digimon then adding to hand. Now correctly: reveal top 3, place 1 [Vemmon] as bottom digi-card, trash rest. Then if 5+ [Vemmon] in digi-cards, delete 1 opponent **Tamer** (not Digimon).

**Effect 2 (Inherited redirect attack):** Was a `pass` stub. Now fully implemented: checks for own [Galacticmon] with 2+ [Vemmon] in digi-cards, returns 2 [Vemmon] to deck bottom (firing OnDigivolutionCardReturnToDeckBottom timing), then calls `game.switch_attack_target()`. Added proper opponent's turn + Galacticmon availability condition checks.

### BT11-111 (Galacticmon) - FIXED (missing timing fire)
**Effect 2 (WhenRemoveField):** Was returning 4 [Vemmon] to deck bottom without firing `OnDigivolutionCardReturnToDeckBottom` timing. Now fires the timing for each returned card, enabling BT11-065/BT18-065/BT21-058 inherited effects to trigger correctly.

### BT21-058 (Snatchmon) - FIXED (auto-selection removed)
**_place_vemmon_from_trash:** Was auto-selecting the first Digimon and first Vemmon cards without player choice. Now provides proper selection flow: player selects up to 2 [Vemmon] from trash via `request_selection`, then selects which Digimon to place them under (auto-selects only when 1 Digimon on field, per C# reference).

### BT21-056 (Vemmon) - FIXED (missing condition)
**Inherited cost reduction:** Was reducing digivolution cost for ANY digivolve target as long as this Digimon had [Vemmon] in text. Now correctly also checks that the card being digivolved into has [Vemmon] in its text, matching C# `CardSourceCondition`.

### P-094 (Destromon) - FIXED (missing timing fire)
**Inherited redirect attack (process2):** Was returning 2 [Vemmon] from Galacticmon's digi-cards to deck bottom without firing `OnDigivolutionCardReturnToDeckBottom`. Now fires the timing for each returned card. Was previously marked BLOCKED for redirect_attack, but `game.switch_attack_target()` already existed and was being used - just the timing fire was missing.

## Spot-Checked Cards (10 clean scripts verified against C# reference)
| Card | Name | Verdict |
|------|------|---------|
| BT11-061 | Vemmon | PASS - Main reveal logic, inherited cost reduction correct |
| BT11-065 | Snatchmon | PASS - When Digi + inherited OnDigivolutionCardReturnToDeckBottom correct |
| BT18-060 | Vemmon | PASS - 2-pass reveal with player selection, inherited cost reduction |
| BT18-065 | Snatchmon | PASS - DigiXros, When Digi, End Turn digi, inherited unsuspend+Blocker |
| BT18-092 | Zenith | PASS - Start Main trash+draw+memory, On Attack return+de-digivolve |
| BT21-060 | Destromon | PASS - Fires OnDigivolutionCardReturnToDeckBottom correctly |
| BT21-062 | Galacticmon | PASS - Fires OnDigivolutionCardReturnToDeckBottom correctly |
| BT21-087 | Zenith | PASS - Memory set, reveal logic correct |
| EX11-046 | Galacticmon | PASS - Delete highest/keep, Blocker+immunity gate, end-turn digi |
| EX11-066 | Xeno | PASS - Trash cost, reveal/place Vemmon, suspend self |

## Previously Fixed Cards (2026-03-13)

### BT11 Batch (4 cards)
| Card | Name | Fixes |
|------|------|-------|
| BT11-061 | Vemmon | Main: rewritten (suspend self, reveal with name filters). Inherited: added Destromon/Galacticmon check + OPT |
| BT11-065 | Snatchmon | When Digi: place 2 Vemmon from trash + Fusionize return. Inherited: unsuspend self + GRANT_BLOCKER modifier |
| BT11-105 | Fusionize | Cost reduction: added Snatchmon check. Main: digivolve from trash. Security: reveal + play Vemmon |
| BT11-111 | Galacticmon | When Digi: place 4 Vemmon + 8-count gate. WhenRemoveField: implemented (return 4 Vemmon). Start Main: engine API for security |

### BT21 Batch (7 cards)
| Card | Name | Fixes |
|------|------|-------|
| BT21-006 | Tsumemon | Added 4+ Vemmon digi-card count gate for DP buff |
| BT21-056 | Vemmon | On Play: removed wrong condition, fixed filters for Vemmon text. Inherited: fixed target check |
| BT21-058 | Snatchmon | On Play/When Digi: proper reveal + Vemmon filter + trash-to-digi step. Inherited: added cost<=4 filter |
| BT21-060 | Destromon | When Digi: added IMMUNE_FROM_STACK_TRASHING, de-digivolve = Vemmon/2. WhenRemoveField: play from digi-cards. Inherited: implemented end-attack |
| BT21-062 | Galacticmon | When Digi: implemented (place 4 Vemmon-text + play Ragnarok). WhenRemoveField: implemented (return 4 Vemmon) |
| BT21-087 | Zenith | Removed wrong condition. Rewrote reveal logic (play Vemmon OR add Vemmon-text) |
| BT21-098 | Ragnarok Cannon | Lowest-cost targeting. Delay: implemented delete+security-trash. Security: Vemmon-text filter, fixed condition |

### EX11+P Batch (3 cards)
| Card | Name | Fixes |
|------|------|-------|
| EX11-046 | Galacticmon | Delete: keep highest, delete all others. 4+ Vemmon gate for Blocker+immunity. End-turn digi from hand/trash |
| EX11-066 | Xeno | Trash cost enforced (Vemmon-text). Trigger: reveal 2 not 4, place Vemmon as digi-cards. Suspend self not opponent |
| P-094 | Destromon | Budget-based multi-delete (cost 3 + Vemmon count). Inherited: redirect_attack now uses game.switch_attack_target() |

## Engine Gaps (Remaining)
| Card | Gap | Status |
|------|-----|--------|
| EX11-066 | "All Turns" trigger on other Digimon play/digivolve | Uses _is_play_observer + _is_digivolve_observer flags -- functional workaround |
| BT18-065 | OnDigivolutionCardReturnToDeckBottom not auto-fired | Scripts manually fire -- functional workaround |

## Smoke Test
- 50/50 mirror games completed (post prior fixes)
