# Archetype QA: Chaos Control
Date: 2026-03-14 (updated)
Total cards: 24

## Summary
- PASS: 7 (correct as-is)
- IMPLEMENTED: 2 (newly created)
- FIXED: 13 (existing scripts corrected)
- BLOCKED: 2 (engine gaps)

## Card-by-Card Verdicts

| Card ID | Name | Verdict | Notes |
|---------|------|---------|-------|
| BT20-069 | Punkmon | FIXED | Blocker/Retaliation now granted to player-selected Digimon, not just self |
| BT20-096 | Black Sabbath | FIXED | Trash trigger fully implemented with OnDeclaration timing (BLOCKED: trash_main_action_mask); Main effect order fixed |
| BT21-100 | The Digimon I Designed | FIXED | Delay digivolve now from trash (not hand); proper own Digimon + trash card selection |
| BT24-066 | Guilmon | FIXED | On Play reveal rewritten: 2-pass reveal (add 1, trash 1, rest to bottom), then trash 1 from hand; inherited delete filter now checks level 3 |
| BT24-070 | Growlmon | FIXED | Added missing hand count <= 4 condition; plays from trash not hand; inherited delete filter checks level 3 |
| BT24-076 | WarGrowlmon | FIXED | Trash Main now plays self from trash with cost -2; delete filter checks level <= 4; inherited On Deletion now plays from trash |
| BT24-080 | Megidramon | PASS | Correct: also_treated_as, trash EOT digivolve, blocker, delete all lowest level |
| BT7-107 | Calling From the Darkness | FIXED | Trash-to-hand selection now uses proper request_selection with SEL_TRASH_START (no auto-selection) |
| EX1-066 | Analog Youth | FIXED | Reveal now properly trashes remaining when no Digimon found; deletion condition checks event_permanent ownership/level/digi-cards |
| EX10-040 | DemiDevimon | FIXED | Process now properly mills top 2 both decks and conditionally gains memory; inherited now has process callback for milling |
| EX11-005 | Yaamon | FIXED | Completely rewritten: digivolves from trash (not hand), then trash 2 from hand only if digivolve succeeded |
| EX11-047 | Impmon | FIXED | Order corrected: trash first, then gain memory (was reversed) |
| EX11-050 | Loudmon | FIXED | Completely rewritten: trash 2, then select own Dark Dragon/Evil Dragon as DP reference, then delete opp with DP <=; added missing inherited SA+1 |
| EX11-069 | Yuuki | FIXED | Trash-for-memory now optional ("by trashing" = cost); digivolve from trash with proper selection; end-of-turn suspend+select from trash fixed |
| EX3-072 | Megiddo Flame | IMPLEMENTED | New script: delete lv4 or lower OR delete own Digimon to upgrade to lv6 or lower; security plays Guilmon from trash |
| EX4-006 | Guilmon | PASS | Correct: conditional Rush on play |
| EX4-011 | ChaosGallantmon | PASS | Correct: alt digi, trash EOT play from trash, DP-scaled deletion |
| EX7-053 | Eyesmon: Scatter Mode | IMPLEMENTED | New script: trash 1 from hand, return 1 trait Digimon from trash; inherited Retaliation |
| EX7-056 | Orochimon | PASS | Correct: Blocker, On Deletion trash then delete lv3 and lv4 |
| EX7-060 | Nidhoggmon | PASS | Correct: Trash Main cost -4, Blocker, On Deletion play from trash |
| P-205 | Insane Synthetic Monster | FIXED | Main effect now has process callback (draw 2, trash 2); delay now deletes own Digimon as cost, plays from trash with cost -3 |
| ST10-15 | Darkness Wave | FIXED | Trash-to-hand selection now uses proper request_selection (no auto-selection) |
| ST16-14 | Matt Ishida | PASS | Correct: memory set to 3, suspend on hand trash |
| ST6-14 | Matt Ishida | PASS | Correct: suspend on own Digimon deletion for memory |

## BLOCKED Items

### BT20-096 Black Sabbath - Trash Trigger
- **Gap**: `trash_main_action_mask` - The engine's `_collect_triggered_effects` explicitly skips `OnDeclaration` for non-field zones. The [Trash] [Main] effect is fully implemented but cannot be activated by the RL agent until the engine adds trash-zone OnDeclaration to the action mask.
- The [Main] and [Security] effects work correctly.

### EX11-050 Loudmon - Scapegoat Grant
- **Gap**: The `AddSkillClass`-style dynamic keyword granting (granting Scapegoat to ALL qualifying Digimon via a continuous static effect) requires engine support for conditional keyword broadcasting. The `_is_scapegoat` flag is set with appropriate trait+hand-count conditions as a best-effort approximation. The SA+1 inherited uses `sa_modifier` which is engine-supported.

## Fixes Applied (This Pass)

### BT20-069 Punkmon
- Blocker/Retaliation now granted to player-selected "1 of your Digimon" via `effect_select_own_permanent`, not auto-granted to self

### BT24-066 Guilmon
- Complete rewrite of On Play: uses `effect_reveal_and_select_multi` for 2-pass reveal (add 1 to hand, trash 1), then trash 1 from hand
- Inherited delete filter now requires `p.level == 3` instead of accepting any Digimon

### BT24-070 Growlmon
- Added `len(player.hand_cards) > 4` guard in process
- Changed zone from `'trash'` (was `'hand'` in old code notes) - correctly plays from trash
- Inherited delete now checks `p.level == 3`

### BT24-076 WarGrowlmon
- Trash Main: added proper condition (card in trash, hand <= 4), plays self from trash with BeforePayCost -2
- On Play/When Digivolving: delete filter now checks `p.level <= 4`
- Inherited On Deletion: changed zone from `'hand'` to `'trash'`

### EX10-040 DemiDevimon
- Process now: checks opponent trash <= 10, mills top 2 from both decks, then checks opponent trash >= 10 for memory gain
- Inherited When Attacking: added process callback to mill top 1 from both decks

### EX11-005 Yaamon
- Complete rewrite: selects Digimon from trash via `request_selection` with `SEL_TRASH_START`, digivolves with cost -1, then trash 2 from hand only if digivolve succeeded

### EX11-047 Impmon
- Reversed order: now trashes from hand first (via `effect_select_hand_card`), then gains memory inside the callback

### EX11-050 Loudmon
- Complete rewrite: trash 2 from hand, then select own [Dark Dragon]/[Evil Dragon] as DP reference, then delete opponent Digimon with DP <= reference
- Added missing inherited SA+1 effect with proper trait+hand-count conditions

### EX11-069 Yuuki
- SoYMP/On Play: now optional ("by trashing" = cost), trash first then gain memory
- When Attacking digivolve: proper selection from trash via `request_selection` with `SEL_TRASH_START`; added hand count <= 4 check
- End of All Turns: suspend this Tamer, then select from trash via `request_selection` (was broken: selected opponent permanent)

### EX3-072 Megiddo Flame (NEW)
- [Main]: `effect_choose_branch` for base (delete lv4) vs upgrade (delete own Digimon to delete lv6)
- [Security]: play 1 [Guilmon] from trash via `effect_play_from_zone`

### EX7-053 Eyesmon: Scatter Mode (NEW)
- [On Play]: trash 1 from hand, then select 1 trait Digimon from trash to return to hand
- Inherited: Retaliation

### P-205 Insane Synthetic Monster
- Main: added process callback (draw 2, trash 2 via sequential `effect_select_hand_card`)
- Delay: now selects own Digimon (cost 7 or lower) to delete as cost, then plays [Kimeramon]/[Millenniummon] from trash with `manual_reduction=3`
- Security: added process callback

### BT21-100 The Digimon I Designed
- Delay digivolve: selects own [Guilmon]/[Growlmon] Digimon, then selects [Growlmon]/[Gallantmon]/[Megidramon] from trash to digivolve into (free)

### EX1-066 Analog Youth
- On Play: when no Digimon in revealed cards, remaining now go to trash (not deck bottom)

### BT20-096 Black Sabbath
- Trash trigger: fully implemented with OnDeclaration, proper condition checks, pays 6 cost, returns to deck bottom, deletes unsuspended Digimon

## New/Modified Files
- `digimon_gym/engine/data/scripts/bt20/bt20_069.py` (fixed)
- `digimon_gym/engine/data/scripts/bt20/bt20_096.py` (fixed)
- `digimon_gym/engine/data/scripts/bt21/bt21_100.py` (fixed)
- `digimon_gym/engine/data/scripts/bt24/bt24_066.py` (fixed)
- `digimon_gym/engine/data/scripts/bt24/bt24_070.py` (fixed)
- `digimon_gym/engine/data/scripts/bt24/bt24_076.py` (fixed)
- `digimon_gym/engine/data/scripts/bt7/bt7_107.py` (fixed)
- `digimon_gym/engine/data/scripts/ex1/ex1_066.py` (fixed)
- `digimon_gym/engine/data/scripts/ex10/ex10_040.py` (fixed)
- `digimon_gym/engine/data/scripts/ex11/ex11_005.py` (fixed)
- `digimon_gym/engine/data/scripts/ex11/ex11_047.py` (fixed)
- `digimon_gym/engine/data/scripts/ex11/ex11_050.py` (fixed)
- `digimon_gym/engine/data/scripts/ex11/ex11_069.py` (fixed)
- `digimon_gym/engine/data/scripts/ex3/ex3_072.py` (new)
- `digimon_gym/engine/data/scripts/ex7/ex7_053.py` (new)
- `digimon_gym/engine/data/scripts/p/p_205.py` (fixed)
- `digimon_gym/engine/data/scripts/st10/st10_15.py` (fixed)
- `docs/archetype-qa/chaos_control.md` (this file)

## Engine Gaps Found
1. `trash_main_action_mask` - OnDeclaration effects in trash zone cannot be activated by RL agent (BT20-096 trash trigger)
2. `addskill_class` - Dynamic keyword broadcasting to multiple permanents based on continuous conditions (EX11-050 Scapegoat grant)
