# Archetype QA: Chaos Control
Date: 2026-03-14
Total cards: 25

## Summary
- PASS: 1 (existing frozen, QA-verified)
- IMPLEMENTED: 9 total
  - CLEAN (first pass): 7 (EX7-056, EX7-060, BT7-107, ST10-15, EX4-011, EX4-074, ST6-14)
  - SIMPLE-FIX (applied): 2 (ST16-14, EX1-066)
- QA-FAIL: 16 (existing frozen scripts with issues)
- BLOCKED: 0

## Smoke Test
- 50/50 mirror-match games completed successfully (random actions, 500 step limit)

## Implementation Results

### New Scripts (5 cards — no prior script)
| Card ID | Name | Verdict |
|---------|------|---------|
| EX7-056 | Orochimon | IMPLEMENTED |
| EX7-060 | Nidhoggmon | IMPLEMENTED |
| EX4-011 | ChaosGallantmon | IMPLEMENTED |
| BT7-107 | Calling From the Darkness | IMPLEMENTED |
| ST10-15 | Darkness Wave | IMPLEMENTED |

### Existing Scripts Reviewed+Fixed (4 cards — had scripts, not frozen)
| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| EX4-074 | ShineGreymon: Ruin Mode | IMPLEMENTED | Rewritten — added missing alt-digi |
| ST16-14 | Matt Ishida | FIXED | Simplified OnDiscardHand condition |
| ST6-14 | Matt Ishida | PASS | Correct as-is |
| EX1-066 | Analog Youth | FIXED | Fixed reveal flow, deletion condition, security timing |

### Fixes Applied to ST16-14
- Simplified OnDiscardHand condition to use `event_player` ownership check instead of broken `source_effect`/`card` context key checks

### Fixes Applied to EX1-066
1. On Play reveal: replaced manual list ops with `game.effect_reveal_and_select_multi()`
2. OnDestroyedAnyone condition: added missing checks for own Digimon, level 5+, digivolution cards
3. Security effect: added missing `set_timing(EffectTiming.SecuritySkill)` and process callback

## QA Failures in Frozen Scripts

### QA Batch 1: Gizmon Package (5 cards)

#### BT13-006 Kapurimon
- Process order reversed: delete happens before trash (should trash hand card as COST first, then delete lv3)
- Delete filter missing level 3 check

#### BT16-006 Cupimon
- Memory gain executes unconditionally — should only happen if card was actually trashed ("By trashing" = cost)

#### BT13-080 ProtoGizmon
- BeforePayCost process does wrong actions (plays from hand + grants immunity instead of deleting Lv2 from breeding)
- BeforePayCost condition doesn't check for Lv2 in breeding area
- Phantom effect1 with fabricated "effect immunity" not in card text
- Missing "can't digivolve" static effect
- On Deletion order reversed + wrong targets (selects opponent permanent instead of own trash cards)
- Play filter accepts any Gizmon instead of specifically "Gizmon: AT"

#### BT13-083 Gizmon: AT
- Missing "can't digivolve" static effect
- On Deletion same issues as BT13-080 (wrong order, wrong targets, wrong filter)
- Play filter accepts any Gizmon instead of specifically "Gizmon: XT"

#### BT13-086 Gizmon: XT
- BeforePayCost process does wrong actions (plays from hand + grants immunity)
- BeforePayCost condition doesn't check for Lv4 Digimon
- Phantom effect1 with fabricated "effect immunity"
- Missing "can't digivolve" static effect
- On Play filter accepts any card instead of "Akihiro Kurata"
- On Deletion filter accepts any card instead of "ProtoGizmon"

### QA Batch 2: BT24 Core (5 cards)

#### BT24-066 Guilmon
- Reveal filter wrong: requires Tamer AND purple AND traits — should be trait-match OR purple Tamer
- Process order inverted (trashes hand first, then reveal)
- Single reveal selection instead of 2 passes (add 1 AND trash 1)
- Inherited attack delete filter missing level 3 check

#### BT24-070 Growlmon
- Missing hand count <= 4 condition check
- `get_cost_itself` used via `getattr()` but is a `@property`
- Inherited attack delete filter missing level 3 check

#### BT24-076 WarGrowlmon
- Trash Main condition missing card-in-trash and hand count checks
- Trash Main process plays wrong card from wrong zone (should play self from trash with -2 cost)
- Delete filters missing level <= 4 check
- Inherited On Deletion plays from 'hand' instead of 'trash'

#### BT24-080 Megidramon
- Trash EOT condition missing card-in-trash and hand count checks
- Trash EOT uses effect_digivolve_from_hand (should be from trash), filter always returns True
- Delete effects select 1 instead of deleting ALL at lowest level

#### BT24-088 Asuna Shiroki
- "Three Musketeers" check only in traits, should also check card_text
- Security effect missing process callback

### QA Batch 3: Misc Frozen (6 cards)

#### EX11-047 Impmon
- Process order reversed: gains memory before trashing (should trash first, then gain)

#### P-123 Ukkomon
- Missing hatch action entirely (only gains memory)

#### BT20-069 Punkmon
- Blocker+Retaliation granted to self instead of player-selected Digimon
- Keywords granted permanently instead of until end of opponent's turn

#### P-205 Insane Synthetic Monster
- OptionSkill main effect has no process callback (stub)
- SecuritySkill has no process callback (stub)
- Delay plays from 'hand' instead of 'trash'
- Delay plays free instead of with cost reduced by 3
- Delay missing "delete own Digimon cost 7-" cost
- Delay missing trash-self-option (Delay cost)

#### BT20-096 Black Sabbath
- Trash Main missing card-in-trash and hand count checks
- Trash Main doesn't pay 6 cost
- Trash Main doesn't return card to deck bottom (tries to return opponent permanent instead)
- Trash Main delete filter doesn't check unsuspended
- OptionSkill process order reversed (delete before trash)
- OptionSkill hand filter incorrectly checks level

#### BT21-100 The Digimon I Designed
- Delay process doesn't select own Guilmon/Growlmon (uses option's permanent instead)
- Uses effect_digivolve_from_hand but card says from trash
- Missing "by effect" check in condition
- Missing trash-self-option (Delay cost)
- digi_filter doesn't check is_digimon

## New/Modified Files
- `digimon_gym/engine/data/scripts/ex7/ex7_056.py` (new)
- `digimon_gym/engine/data/scripts/ex7/ex7_060.py` (new)
- `digimon_gym/engine/data/scripts/ex4/ex4_011.py` (new)
- `digimon_gym/engine/data/scripts/ex4/ex4_074.py` (rewritten)
- `digimon_gym/engine/data/scripts/bt7/bt7_107.py` (new)
- `digimon_gym/engine/data/scripts/st10/st10_15.py` (new)
- `digimon_gym/engine/data/scripts/st16/st16_14.py` (fixed)
- `digimon_gym/engine/data/scripts/ex1/ex1_066.py` (fixed)
- `docs/archetype-qa/chaos_control.md` (this file)

## Engine Gaps Found
None — all cards implementable with current engine.
