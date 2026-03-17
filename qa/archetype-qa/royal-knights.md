# Archetype QA: Royal Knights
Date: 2026-03-17 (faithfulness campaign)
Total cards: 61

## Summary
- FAITHFUL: 47
- FIXED: 11 (this campaign)
- DEFERRED: 3 (engine gaps: attack redirect, decoy color, suppress On Play)
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT6-082 | Sistermon Blanc | FAITHFUL | Aura Blocker grant |
| BT6-084 | Sistermon Ciel | FAITHFUL | +2000 DP aura for RK/Huckmon |
| BT8-094 | Digimon Emperor | FAITHFUL | OnDestroyedAnyone timing |
| BT8-097 | Crimson Blaze | FAITHFUL | Modifier arg order + leak guard |
| BT9-092 | Cool Boy | FAITHFUL | Correct docstring |
| BT9-103 | Kongou | FAITHFUL | register_modifier loop |
| BT13-007 | King Drasil_7D6 | FAITHFUL | BeforePayCost leak guard |
| BT13-012 | GeoGreymon | FAITHFUL | Search security for tamer |
| BT13-019 | Gankoomon | FAITHFUL | Royal Knight from breeding branch |
| BT13-030 | UlforceVeedramon | FAITHFUL | Opponent targeting, count calculation |
| BT13-040 | Magnamon | FIXED | Branch choice: Veemon filter, digi-source play path, self-trigger |
| BT13-075 | Alphamon | FIXED | Selection: trash-to-digi-stack, blanket CANNOT_ATTACK, WhenRemoveField |
| BT13-087 | Dynasmon | FAITHFUL | Reveal+select multi |
| BT13-093 | Omekamon | FAITHFUL | No changes needed |
| BT13-095 | Marcus Damon | FAITHFUL | OnStartTurn timing, correct suspend |
| BT13-102 | Keenan Crier | FAITHFUL | Removed incorrect is_on_play |
| BT13-110 | Royal Knights of the Purge | FAITHFUL | Delay digi-source iteration, Rush via modifier |
| BT13-111 | Gallantmon | FAITHFUL | Trash count cost reduction with leak guard |
| BT13-112 | Omnimon | FIXED | Selection: Royal Knight from breeding play logic implemented |
| BT14-009 | Gotsumon | FAITHFUL | CANNOT_PLAY_CARD modifier correct |
| BT15-084 | Kari Kamiya | FIXED | Remove OPT: Security A -1 process, suspend-as-cost |
| BT15-092 | Revelation of Light | FAITHFUL | Security search/play, shuffle |
| BT17-018 | Gallantmon: Crimson Mode | FAITHFUL | Alt-digi, When Attacking timing, security trash |
| BT17-077 | Imperialdramon: PM | FAITHFUL | Trash-return + memory gain |
| BT18-087 | Owen Dreadnought | FAITHFUL | OnStartTurn timing, security loss condition |
| BT19-072 | LordKnightmon | DEFERRED | Attack redirect stubbed (engine gap) |
| BT19-093 | Queen Device | FIXED | Selection: DP -3000 and color bypass; When Digivolving disable is engine gap |
| BT20-017 | Jesmon | FAITHFUL | Token play, delete, FORCE_ATTACK |
| BT20-021 | Jesmon GX | FAITHFUL | Process callbacks, unsuspend+security trash |
| BT20-045 | Examon | FIXED | Piercing+self-unsuspend: condition missing Digimon check and self-exclusion |
| BT20-056 | Alphamon | FIXED | Selection: DP mod via register_modifier, breeding digivolve with trash fallback |
| BT20-060 | Alphamon: Ouryuken | FAITHFUL | DNA check, blast DNA names |
| BT20-083 | Omekamon | FIXED | Inherited ref: name collision 'Omnimon' -> 'Omnimon (X Antibody)' |
| BT20-091 | Cool Boy | FAITHFUL | Play/digivolve observers |
| BT20-100 | The Last Guardian | FAITHFUL | WhenRemoveField prevention + Delay guard |
| BT20-102 | Omnimon (X Antibody) | FAITHFUL | X Antibody trait check |
| BT21-086 | Marcus Damon | FAITHFUL | Piercing grant with turn expiry |
| BT22-009 | Effecmon | FAITHFUL | On Play / When Digivolving delete |
| BT22-017 | Gabumon | FAITHFUL | Reveal + select multi, DNA digivolve |
| BT22-025 | UlforceVeedramon | FAITHFUL | Branch choice + blast digivolve + OPT unsuspend |
| BT22-041 | Kentaurosmon | FIXED | Guard: cost reduction, security placement, suspend trigger |
| BT22-052 | Leopardmon | FAITHFUL | DP filter, Blocker grant to Lv3+ |
| BT23-013 | Jesmon | FAITHFUL | Alt-digi conditions, branch choice token vs Sistermon |
| BT23-014 | Gallantmon | FAITHFUL | Trash play block + DP-scaled delete |
| BT23-035 | Dynasmon | FAITHFUL | Security trash, DP mod, trigger conditions |
| BT23-047 | Examon | DEFERRED | Suspend auto-selection fixed, FORCE_ATTACK added; optional aspect engine gap |
| BT23-054 | Magnamon | FAITHFUL | Modifier call, empty-target guard |
| BT23-057 | Gankoomon | FAITHFUL | Cost reduction with trash return, deck placement |
| BT23-058 | Craniamon | FAITHFUL | WhenRemoveField ownership check |
| BT23-072 | King Drasil_7D6 | FAITHFUL | Hand/Main stub filled |
| BT23-077 | Sistermon Ciel | FAITHFUL | also_treated_as_names, Blocker, delete, De-Digivolve |
| EX4-065 | Trident Gaia | FAITHFUL | Created and correct |
| EX8-073 | Gallantmon (X Antibody) | FAITHFUL | Source check, delete-or-trash, immunity |
| EX8-074 | MedievalGallantmon | FAITHFUL | BeforePayCost process uses player selection |
| EX10-068 | Digimon Emperor | FAITHFUL | Memory gain, delete filter, execution order |
| EX11-053 | Omekamon | FIXED | Selection: On Deletion rewritten with hand+King Drasil search, place-under callback |
| EX11-071 | Cool Boy | FAITHFUL | Reveal multi, tamer return |
| P-186 | Gallantmon | FAITHFUL | Delete targets both fields, alt-digi |
| P-206 | Digital Gate Open | FAITHFUL | Reveal 3, delay play, security |
| ST12-12 | Sistermon Blanc | DEFERRED | Decoy color restriction engine gap |
| ST20-11 | WarGreymon | FAITHFUL | Timing fixed (earlier fix) |

## Fixes Applied (2026-03-17 Campaign)
### BT20-045 Examon
- Added Piercing grant; unsuspend condition now checks is_digimon and excludes self

### BT13-040 Magnamon
- Added branch choice for Veemon filter, digi-source play path, self-trigger correction

### BT13-075 Alphamon
- Complete rework: trash-to-digi-stack selection, blanket CANNOT_ATTACK, WhenRemoveField

### BT20-056 Alphamon
- DP mod via register_modifier; breeding digivolve with trash fallback selection

### BT19-093 Queen Device
- Selection: DP -3000 target and color bypass corrected

### BT13-112 Omnimon
- Royal Knight from breeding play logic implemented (was stub)

### BT20-083 Omekamon
- Name collision fix: 'Omnimon' -> 'Omnimon (X Antibody)' for inherited reference

### BT22-041 Kentaurosmon
- Cost reduction, security placement, and suspend trigger all corrected

### EX11-053 Omekamon
- On Deletion rewritten: hand+King Drasil search, place-under callback with selection

### BT15-084 Kari Kamiya
- Removed OPT restriction; Security A -1 process implemented; suspend-as-cost fixed

### ST20-11 WarGreymon
- Timing fix applied in earlier pass (confirmed correct)
