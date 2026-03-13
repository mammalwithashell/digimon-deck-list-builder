# Archetype QA: Galacticmon
Date: 2026-03-13
Total cards: 36

## Summary
- PASS: 2 (P-206, LM-031)
- IMPLEMENTED: 6 (BT18-060, BT18-065, BT18-092, BT7-105, ST13-08, LM-048)
- QA-FAIL → FIXED: 14
- BLOCKED: 2 engine gap effects (EX11-042 redirect_attack, P-094 redirect_attack)
- Shared with ExMaquinamon: EX11 cards fixed in ExMaquinamon pass also benefit Galacticmon

## Implemented Cards (new scripts)
| Card | Name | Description |
|------|------|-------------|
| BT18-060 | Vemmon | On Play: 2-pass reveal (hand + digi-card placement). Inherited: digi cost -1 for Vemmon-text |
| BT18-065 | Snatchmon | DigiXros -1/4 Vemmon. When Digi: place 2 Vemmon from trash. End Turn: digivolve if 4+ digi-cards. Inherited: unsuspend + Blocker on Vemmon return |
| BT18-092 | Zenith | Start Main: trash Vemmon → draw + memory. On Attack: suspend + return 2 Vemmon → de-digivolve. Security: play free |
| BT7-105 | Pride Memory Boost! | Reveal 3, play black cost<=4 free, trash rest. Delay +2. Security: place in BA |
| ST13-08 | Chikurimon | [All Turns] Players can't reduce play costs |
| LM-048 | Chrome Memory Boost! | Reveal 3, add green/black Digimon. Delay +2. Security: place in BA |

## Fixed Cards

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
| P-094 | Destromon | Budget-based multi-delete (cost 3 + Vemmon count). Removed orphan effect. **BLOCKED: redirect_attack** |

## Engine Gaps
| Card | Gap | Status |
|------|-----|--------|
| EX11-042 | redirect_attack | Stub — no engine API |
| P-094 | redirect_attack | Stub — no engine API |
| EX11-066 | "All Turns" trigger on other Digimon play/digivolve | Cannot dispatch to Tamer — documented |
| BT18-065 | OnDigivolutionCardReturnToDeckBottom not auto-fired | Scripts manually fire — functional workaround |

## Smoke Test
- 50/50 mirror games completed (post all fixes)
