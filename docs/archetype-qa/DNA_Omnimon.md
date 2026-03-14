# Archetype QA: DNA Omnimon
Date: 2026-03-14 (Phase 2 review)
Total cards: 47

## Summary
- All 47 scripts present, all compile
- No name aliasing needed (all Omnimon variants have "Omnimon" in their real name)
- 6 scripts fixed (auto-selections, leak guard bug)
- 10 scripts spot-checked clean
- BLOCKED: 0

## Fixes Applied

### BT22-008 (Agumon) - FIXED
- **Issue**: On Play trash recovery auto-selected `qualifying[0]` instead of agent choice
- **Fix**: Added `SelectTrash` phase with `game.request_selection()` to let agent choose which Greymon/Garurumon/Omnimon to return from trash

### BT17-007 (Agumon) - FIXED
- **Issue**: Start of Main Phase trash recovery auto-selected `matching[0]` instead of agent choice
- **Fix**: Added `SelectTrash` phase with `game.request_selection()` to let agent choose which card to return from trash

### EX9-066 (Tai Kamiya & Matt Ishida) - FIXED
- **Issue**: On Play trash recovery auto-selected `qualifying[0]` instead of agent choice
- **Fix**: Added `SelectTrash` phase with `game.request_selection()` to let agent choose. Draw-1 fallback still triggers when no qualifying cards exist.

### BT5-092 (Nokia Shiramine) - FIXED
- **Issue**: BeforePayCost leak guard `context.get('card_source') is not card` was inverted -- it only triggered when `card_source IS card` (i.e., when this tamer was being played), but the effect reduces cost for OTHER Digimon being digivolved into Garurumon/Omnimon/Greymon
- **Fix**: Removed self-only check. Now correctly triggers when any card with matching name is being digivolved, and blocks when `card_source is card` (self). Added owner check for own Digimon only.

### EX4-073 (Omnimon Alter-B) - FIXED
- **Issue 1**: "Delete up to 6 play cost total" auto-selected greedily instead of letting agent choose multiple targets with budget constraint
- **Fix**: Replaced greedy loop with iterative `effect_select_opponent_permanent()` calls that let the agent pick targets one by one within remaining budget, with `is_optional=True` to stop early
- **Issue 2**: When Attacking evo card trashing auto-selected first eligible cards without agent choice
- **Fix**: Added `SelectSource` phase selection for each evo card to trash, with `is_optional=True` allowing the agent to stop trashing early
- **Issue 3**: Docstring said "Omnimon Alter-S" but card is "Omnimon Alter-B"
- **Fix**: Corrected docstring

### EX9-021 (Omnimon Alter-S) - FIXED
- **Issue**: End of Attack auto-selected first Greymon/Ver.1 and first Garurumon/Ver.2 from evo cards without agent choice
- **Fix**: Added `SelectSource` phase for both Greymon/Ver.1 and Garurumon/Ver.2 selections using `game.request_selection()` with proper field-based action IDs

## Spot-Check Results (10 clean scripts)

| Card | Name | Verdict | Notes |
|------|------|---------|-------|
| BT17-015 | WarGreymon | PASS | Branch choice correct. Inherited Omnimon name check uses `contains_card_name` correctly |
| BT17-027 | MetalGarurumon | PASS | Mirror of BT17-015. Can't-suspend modifier and Agumon->WarGreymon digi both correct |
| BT17-078 | Omnimon | PASS | Blast DNA, Raid, Blocker all flagged. DNA-gated bottom-deck + unconditional delete matches C# |
| BT17-081 | Tai & Matt | PASS | Suspend-on-play/digi memory gain correct (checks Greymon + Garurumon separately). End-turn Omnimon attack via FORCE_ATTACK modifier |
| BT17-095 | Brave Tornado | PASS | Play Agumon/Gabumon free. Delay DNA protection with WhenPermanentWouldBeDeleted timing correct |
| BT17-102 | Greymon (Bond) | PASS | Name-change effect for Lv3- digi-cards. +3000 DP if Koromon. On Deletion play tamer or hatch with branch choice |
| BT22-013 | WarGreymon | PASS | Nokia warp-digi from hand. Branch: Gabumon->MG or delete lowest DP. Inherited Omnimon security trash |
| BT22-026 | MetalGarurumon | PASS | Nokia warp-digi from hand. Branch: Agumon->WG or bounce lowest level. Inherited Omnimon unsuspend |
| BT22-084 | Nokia Shiramine | PASS | Memory set to 3. Play Agumon/Gabumon if <=1 Digimon. DP +1000 for Greymon/Garurumon/Omnimon |
| BT5-093 | Tai & Matt | PASS | +2 memory if opp Lv6+. SA+1 for Omnimon. Security play free |

## Name Aliasing Verification
- No cards in this deck pool have "also treated as" in their card text
- All 4 Omnimon variants (BT17-078, BT22-015, EX4-073, EX9-021) have "Omnimon" in their actual card name
- `contains_card_name('Omnimon')` correctly matches all of them via substring matching
- "Omnimon (X Antibody)" is NOT in this deck pool but would be matched by `contains_card_name('Omnimon')` -- this is correct behavior since it contains the substring

## Implemented Cards

### BT17 Batch (9 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT17-007 | Agumon | Alt digi Koromon. Start Main: return Greymon/Garurumon/Omnimon from trash. Inherited: end-turn DNA digi |
| BT17-015 | WarGreymon | Alt digi Greymon. Cost -3 w/ Tai. On Play/Digi: delete 8000- OR digi Gabumon->MetalGarurumon. Inherited: trash security if Omnimon |
| BT17-019 | Gabumon | Alt digi Tsunomon. Start Main: draw if Matt. Inherited: end-turn DNA digi |
| BT17-027 | MetalGarurumon | Alt digi Garurumon. Cost -3 w/ Matt. On Play/Digi: can't-suspend 1 OR digi Agumon->WarGreymon. Inherited: unsuspend if Omnimon |
| BT17-078 | Omnimon | Raid+Blocker+Blast DNA. On Play/Digi: if DNA, bottom-deck all opp Digimon of chosen level + delete 1 |
| BT17-081 | Tai & Matt | Tamer. Suspend on play/digi for memory. End turn: Omnimon attacks. Security: play free |
| BT17-093 | Kari Kamiya | Tamer. Suspend on hatch +1 memory. End turn: return self, draw, play tamer |
| BT17-095 | Brave Tornado | Option. Play Agumon/Gabumon free. Delay: DNA digi protection. Security: play tamer |
| BT17-102 | Agumon -Bond- | Alt digi Agumon. When Digi: +3000 DP if Koromon, delete opp <=DP. On Deletion: play tamer/hatch |

### BT12 (1 card)
| Card | Name | Key Effects |
|------|------|-------------|
| BT12-059 | Agumon (Black) | Alt digi Koromon. On Play: reveal 4, add Greymon/Omnimon + Tai Kamiya. Inherited: +1000 DP if Greymon/Omnimon |

### BT22 Batch (10 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT22-005 | Tsumemon | Inherited: draw on Unidentified/CS play |
| BT22-008 | Agumon | On Play: return Greymon/Garurumon/Omnimon from trash (agent choice). Inherited: end-turn DNA digi |
| BT22-013 | WarGreymon | Nokia warp-digi. When Digi: Gabumon->MG or delete lowest DP. Inherited: trash security if Omnimon |
| BT22-015 | Omnimon | Blocker+Decode x2. On Play/Attack: delete lowest DP. When Digi: bottom-deck per 2 same-level, then attack |
| BT22-017 | Gabumon | On Play: reveal 3, add Omnimon-text + CS-trait. Inherited: end-turn DNA digi |
| BT22-026 | MetalGarurumon | Nokia warp-digi. When Digi: Agumon->WG or bounce lowest level. Inherited: unsuspend if Omnimon |
| BT22-084 | Nokia Shiramine | Tamer. Set memory 3. Play Agumon/Gabumon. DP +1000 Greymon/Garurumon/Omnimon |
| BT22-089 | Mirei Mikagura | Tamer. Start Main: return self, play 4+ Mirei/CS tamer. On Play: trash trait card, draw 2 |
| BT22-094 | Yuugo Kamishiro | Tamer. On Play: reveal 3, add CS. Your Turn: return self to reduce play cost by 2 |
| BT22-099 | Kuremi Detective Agency | Option. Reveal 3, add CS. Delay: +2 memory. Security: place in BA |

### EX4+EX9 Batch (6 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| EX4-038 | Agumon | On Play: reveal 3, add Greymon + Gabumon/Garurumon/Omnimon. Inherited: +1 memory on other digi |
| EX4-039 | Gabumon | On Play: reveal 3, add Garurumon + Agumon/Greymon/Omnimon. Inherited: +1 memory on other digi |
| EX4-061 | Tai & Matt | Tamer. On Play: play partner free. Suspend on digi. Security: play free |
| EX4-073 | Omnimon Alter-B | Alt digi Omnimon. When Digi: de-digi 3 + budget delete (agent choice). When Attacking: trash evo cards for deletes (agent choice) |
| EX9-021 | Omnimon Alter-S | DNA Blue+Red Lv6. When Digi: if DNA, immunity + delete highest level. End Attack: play from evo cards (agent choice) |
| EX9-066 | Tai & Matt | Tamer. On Play: return Greymon/Garurumon/Omnimon from trash (agent choice). Suspend: memory per Greymon+Garurumon |

### BT5+BT23+ST+LM+EX+P Batch (14 cards)
| Card | Name | Key Effects |
|------|------|-------------|
| BT1-090 | Gravity Crush | Option. +2 memory, -2 at end of turn |
| BT5-092 | Nokia Shiramine | Tamer. On Play: play Agumon/Gabumon. Main: digi cost -1 for Garurumon/Omnimon/Greymon (fixed leak guard) |
| BT5-093 | Tai & Matt | Tamer. Start turn: +2 memory if opp Lv6+. SA+1 for Omnimon |
| BT8-097 | Crimson Blaze | Option. Cost reduction per opp Digimon. Delete all opp 6000- DP |
| BT13-012 | GeoGreymon | When Digi: search security for tamer. Inherited: delete 3000- on tamer suspend |
| BT14-001 | Koromon | Inherited: draw on security break |
| BT15-101 | MetalGarurumon | Matt warp-digi. Evade. When Digi: 3 can't suspend. Unsuspend on suspend |
| BT16-082 | Ukkomon | Your Turn: reveal 3 on move from breeding, add Digimon/Tamer, hatch |
| BT21-102 | Tai Kamiya | Tamer. Set memory 3. Draw on attack. Main: play low-cost trait card |
| BT23-008 | Greymon | Raid. Main: stack-shift, play Gabumon/Nokia -2. Inherited: +2000 DP |
| BT23-018 | Garurumon | Jamming. Main: stack-shift, play Agumon/Nokia -2. Inherited: +2000 DP opp turn |
| EX1-021 | MetalGarurumon | When Digi: memory per 4 hand. When Attack: bottom-deck opp w/ On Deletion |
| EX10-010 | BlackWarGreymon | Blast digi. Raid+Reboot+Blocker. Delete cost 7-. Immunity if opp 13000+ |
| LM-034 | Wisteria Memory Boost! | Reveal 3, add blue/red Digimon. Delay +2 memory |
| P-123 | Ukkomon | Hatch on move from breeding, +1 memory |
| P-182 | WarGreymon | SA+1. Blocker. Delete opp <=DP. +1000 DP per color |
| P-206 | Digital Gate Open | Reveal 3, add Digimon+Tamer. Delay: play tamer -4. Security: play Digimon cost 3- free |
| ST2-13 | Hammer Spark | Option. +1 memory. Security: +2 memory |
| ST20-10 | Agumon | Warp digi into WarGreymon. Inherited: Reboot |
| ST20-11 | WarGreymon | Blast digi. On Play/Digi: immunity. When Digi/Attack: delete lowest DP |
| ST20-15 | Island of Adventure | Security field spell. +2000 DP Lv3+. Main: swap top security. Security: play tamer |
