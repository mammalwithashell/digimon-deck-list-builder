# Archetype QA: ExMaquinamon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 16

## Summary
- FAITHFUL: 12
- FIXED: 2 (this campaign)
- DEFERRED: 2 (engine gap: hand-activated [Main] on Digimon)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT3-103 | Hidden Potential Discovered! | FAITHFUL | Cost reduction green Digimon only |
| EX6-072 | Mega Digimon Assembly! | FAITHFUL | Security trash-to-hand with player selection |
| EX11-006 | Flickmon | FAITHFUL | Linked-with-Maquinamon check, digivolve correct |
| EX11-027 | Maquinamon | FAITHFUL | 3-way zone choice, WhenRemoveField link selection |
| EX11-029 | Turbomon | FAITHFUL | Player selection for digi card source, zone choice |
| EX11-033 | Maneuvermon | FAITHFUL | Source from hand or link cards, zone choice |
| EX11-036 | Dalphomon | FAITHFUL | Suspends 2 Digimon/Tamers, FORCE_ATTACK modifier |
| EX11-040 | Mulemon | FAITHFUL | Player selection for digi card, zone choice |
| EX11-042 | MockingBirdmon | FAITHFUL | Hand or link cards source, redirect_attack |
| EX11-045 | Metatromon | FAITHFUL | De-digivolve, cannot-digivolve, EOT digi other |
| EX11-062 | Shoto Kazama | FAITHFUL | register_modifier arg order corrected |
| EX11-070 | Unchained | FAITHFUL | Selection from digi cards, DP floor, immunity |
| EX11-071 | Cool Boy | FAITHFUL | Reveal+select and Main deck-bounce correct |
| EX11-073 | ExMaquinamon | FIXED | Security pop: full zone-choice loop (hand/trash/digi) with selection |
| EX11-045 | Metatromon | FIXED | Condition: corrected de-digivolve condition check |
| LM-048 | Chrome Memory Boost! | FAITHFUL | Reveal with player choice |
| P-151 | Digimon Liberator | FAITHFUL | Security activates Main effects |

## Fixes Applied (2026-03-17 Campaign)
### EX11-073 ExMaquinamon
- Fixed security pop handling: link-up-to-3 now uses full zone-choice loop (hand/trash/digi) with proper selection per C# reference

### EX11-045 Metatromon
- Corrected de-digivolve condition check
