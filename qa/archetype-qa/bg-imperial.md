# Archetype QA: BG Imperial
Date: 2026-04-05
Total cards: 27 (25 original + 2 AD1 additions)
Pipeline: batch-fix-cards

## Summary
- FAITHFUL: 7
- IMPLEMENTED: 3 (AD1-011, AD1-024 new scripts; BT12-021/BT12-047 DNA digivolve unblocked)
- FIXED: 17
- PARTIAL: 0
- BLOCKED: 0

## Per-Card Verdicts
| Card ID | Name | Verdict | Tests | Notes |
|---------|------|---------|-------|-------|
| BT12-002 | DemiVeemon | FIXED | 6 | Empty deck guard, CardColor enum |
| BT3-002 | DemiVeemon | FAITHFUL | 6 | Minor cleanup |
| BT12-021 | Veemon | FIXED | 11 | Added inherited DNA digivolve, fixed Free trait |
| P-117 | Veemon | FIXED | 10 | Timing fix, permanent match, Free trait |
| BT12-047 | Wormmon | FIXED | 8 | Added inherited DNA digivolve, fixed Free trait |
| BT16-040 | Wormmon | FIXED | 12 | Free trait, trash selection offset |
| EX1-014 | ExVeemon | FIXED | 6 | Free trait in inherited Jamming |
| BT12-022 | ExVeemon | FIXED | 8 | Free trait, removed spurious EOT DNA digivolve |
| BT12-050 | Stingmon | FIXED | 8 | Free trait, removed spurious EOT DNA digivolve |
| ST9-09 | Stingmon | FAITHFUL | 10 | Correct |
| BT21-037 | Lighdramon | FIXED | 10 | Engine CHANGE_DP fix |
| ST9-05 | Paildramon | FAITHFUL | 13 | Correct |
| BT12-028 | Paildramon | FIXED | 9 | Selection chaining, modifier conditions, Free trait |
| BT16-025 | Paildramon | FIXED | 13 | Modifier conditions, WA async race |
| AD1-011 | Paildramon | IMPLEMENTED | 13 | Partition, battle immunity, WA digivolve |
| BT16-028 | Imperialdramon DM | FIXED | 20 | Selection chain overwrite |
| ST9-06 | Imperialdramon DM | FIXED | 7 | SelectSource→SelectTarget |
| BT12-031 | Imperialdramon FM | FIXED | 10 | DP modifier pattern |
| BT16-027 | Imperialdramon FM | FAITHFUL | 13 | Correct |
| BT20-020 | Imperialdramon FM | FIXED | 14 | CANNOT_PLAY_BY_EFFECT |
| AD1-024 | Imperialdramon FM | IMPLEMENTED | 14 | SA+1, Blocker, reactive suspend |
| BT17-077 | Imperialdramon PM | FAITHFUL | 16 | Correct |
| BT16-085 | Davis & Ken | FIXED | 18 | game.opponent→player.enemy crash |
| BT3-093 | Davis Motomiya | FAITHFUL | 16 | Correct |
| BT3-103 | Hidden Potential | FAITHFUL | 8 | Correct |
| BT17-097 | Return to Primogenitor | FIXED | 16 | Free trait (attribute_eng) |
| LM-030 | Green Scramble | FIXED | 16 | Delay condition after engine trash |

## Systemic Issues Found

### Free trait (attribute_eng vs card_traits)
`card_traits` returns `type_eng` only; "Free" is in `attribute_eng`. Fixed in 10+ scripts.

### Engine: CHANGE_DP modifiers
`Permanent.dp` now queries modifier registry for CHANGE_DP (engine fix in permanent.py).

### Spurious EOT DNA effects
BT12-022 and BT12-050 had incorrect inherited DNA digivolve effects. Removed.

### Selection overwrite
Multiple `effect_select_opponent_permanent` calls must be chained in callbacks.

## Test Coverage
27 test files, 308 total test methods, all passing.
