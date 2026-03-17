# Archetype QA: Medusamon
Date: 2026-03-17 (faithfulness campaign)
Total cards: 53

## Summary
- FAITHFUL: 33
- FIXED: 20 (this campaign)
- DEFERRED: 0
- ENGINE GAP: 0

## Card-by-Card Verdicts
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT5-008 | Gaossmon | FIXED | Self-exclusion on DP aura; evo cost prevention partial |
| BT8-097 | Crimson Blaze | FAITHFUL | Cost reduction per opp Digimon, delete all 6000- DP |
| BT18-087 | Owen Dreadnought | FAITHFUL | Opponent security check + tamer suspend check |
| BT20-102 | Omnimon (X Antibody) | FAITHFUL | Board wipe, Rush + unsuspend attack |
| BT21-001 | Arresterdramon | FIXED | Digivolve target selection with cost_reduction=1 |
| BT21-008 | Agumon | FIXED | Reveal select: two-step sequential selection (Reptile/Dragonkin first, then LIBERATOR) |
| BT21-013 | Guilmon | FIXED | Placement correction for card positioning |
| BT21-017 | Tyrannomon | FIXED | Tamer gate condition corrected |
| BT21-025 | Greymon | FIXED | DP filter on attack target change; added ESS turn guard |
| BT21-029 | Medusamon | FIXED | Shared hash for OPT; WhenDigivolving + EndOfAttack delete callbacks implemented |
| BT21-072 | Arresterdramon: Superior Mode | FIXED | Alt-digi condition corrected; DP bonus counts digivolution cards |
| BT21-081 | MetalGreymon | FAITHFUL | Opponent gate, Reptile/Dragonkin filter, FORCE_ATTACK |
| BT21-093 | Raging Serpentine | FIXED | Delay condition checks opponent security owner |
| BT23-005 | Reptiledramon | FIXED | Leak guard: added Reptile/Dragonkin trait filter to BeforePayCost |
| BT23-014 | Gallantmon | FAITHFUL | Trash play block + DP-scaled delete |
| BT24-001 | Gigimon | FAITHFUL | Correct as-is |
| BT24-008 | Koromon | FAITHFUL | Correct as-is |
| BT24-011 | Guilmon | FAITHFUL | Correct as-is |
| BT24-012 | Growlmon | FAITHFUL | WhenRemoveField with is_opponent_effect + trait check |
| BT24-016 | Lamiamon | FAITHFUL | ESS play filter with 5000 DP limit |
| BT24-017 | Megidramon | FIXED | Trash cost+gating+duration corrected; On Attack +2000 DP added |
| BT24-018 | Styracomon | FIXED | Optional actions: Piercing, WhenRemoveField delete-to-prevent, conditional security trash |
| BT24-082 | Owen Dreadnought | FIXED | Optional attack: changed from unsuspend to FORCE_ATTACK modifier |
| BT24-089 | Unique Emblem: Blazing Conductor | FAITHFUL | Delay condition corrected (removed incorrect trait check) |
| EX8-074 | MedievalGallantmon | FAITHFUL | Suspend step offers both own and opponent Digimon |
| EX9-013 | BlitzGreymon | FAITHFUL | DNA digivolve + FORCE_ATTACK grant implemented |
| EX10-010 | BlackWarGreymon | FAITHFUL | Conditional +3000 DP and effect immunity |
| EX11-008 | Tyrannomon | FAITHFUL | Raid grant with turn expiry |
| EX11-012 | Medusamon | FIXED | Token cost: DP-capped delete, token play, token-death-prevention |
| EX11-054 | Gallantmon | FAITHFUL | Reptile/Dragonkin trait check + Progress DP grant |
| LM-021 | Red Scramble | FIXED | Iterative delete pattern for trash selection |
| LM-027 | Magenta Memory Boost! | FIXED | Select: proper selection with cost_reduction=3 |
| P-035 | Gaia Force | FAITHFUL | SecuritySkill effect correct |
| P-103 | Crimson Blaze | FAITHFUL | Red filter on reveal + delay digivolve |
| P-189 | Growlmon | FAITHFUL | Correct as-is |
| P-206 | Digital Gate Open | FAITHFUL | Ignore color requirements, security add-to-hand |
| ST22-08 | Growlmon | FIXED | Select: proper player selection |
| BT5-086 | Omnimon | FAITHFUL | Blitz, unsuspend, deletion prevention |
| BT5-093 | Tai & Matt | FAITHFUL | +2 memory if opp Lv6+, SA+1 for Omnimon |
| EX1-021 | MetalGarurumon | FAITHFUL | Memory per 4 hand, bottom-deck On Deletion |
| EX4-038 | Agumon | FAITHFUL | Reveal 3, add Greymon + partner |
| EX4-061 | Tai & Matt | FAITHFUL | Play partner free, suspend on digi |
| EX9-066 | Tai Kamiya & Matt Ishida | FAITHFUL | Return from trash with agent choice, draw-1 fallback |
| ST16-14 | Matt Ishida | FAITHFUL | Memory 3, suspend on hand trash |
| BT12-059 | Agumon (Black) | FAITHFUL | Alt digi, reveal 4, inherited +1000 DP |
| BT14-001 | Koromon | FAITHFUL | Inherited draw on security break |
| EX4-039 | Gabumon | FAITHFUL | Reveal 3, add Garurumon + partner |
| BT13-012 | GeoGreymon | FAITHFUL | Search security for tamer, inherited delete on tamer suspend |
| BT16-082 | Ukkomon | FAITHFUL | Reveal 3 on move from breeding, add Digimon/Tamer |
| P-123 | Ukkomon | FAITHFUL | Hatch on move from breeding |
| P-182 | WarGreymon | FAITHFUL | SA+1, Blocker, delete opp <=DP |
| ST20-11 | WarGreymon | FAITHFUL | Blast digi, immunity, delete lowest DP |
| ST20-10 | Agumon | FAITHFUL | Warp digi, inherited Reboot |

## Fixes Applied (2026-03-17 Campaign)
### BT5-008 Gaossmon
- Added self-exclusion to DP aura; fixed evo cost prevention filter

### BT21-017 Tyrannomon
- Corrected tamer gate condition for effect activation

### BT21-025 Greymon
- Added DP filter on attack target change; added ESS turn guard for inherited effect

### BT21-029 Medusamon
- Shared hash for once-per-turn; implemented WhenDigivolving and EndOfAttack delete callbacks

### BT24-017 Megidramon
- Corrected trash cost, gating, and duration; added missing On Attack +2000 DP effect

### EX11-012 Medusamon
- Fixed token cost: DP-capped delete targeting, token play, token-death-prevention

### BT24-082 Owen Dreadnought
- Changed "may attack" from unsuspend to FORCE_ATTACK modifier

### BT21-001 Arresterdramon
- Added cost_reduction=1 to effect_digivolve_from_hand; fixed target selection

### BT21-008 Agumon
- Replaced single reveal selection with two-step sequential selection (Reptile/Dragonkin first, then LIBERATOR)

### BT21-013 Guilmon
- Fixed card placement positioning

### BT24-018 Styracomon
- Made security trash conditional; added Piercing keyword; implemented WhenRemoveField delete-to-prevent

### LM-021 Red Scramble
- Replaced auto-selection with iterative delete pattern for trash selection

### LM-027 Magenta Memory Boost!
- Added proper selection with cost_reduction=3

### ST22-08 Growlmon
- Replaced auto-selection with proper player selection

### BT23-005 Reptiledramon
- Added Reptile/Dragonkin trait filter to BeforePayCost leak guard

### BT21-072 Arresterdramon: Superior Mode
- Fixed alt-digi condition; changed DP bonus to count digivolution cards under top

### BT21-093 Raging Serpentine
- Added opponent security check to delay condition
