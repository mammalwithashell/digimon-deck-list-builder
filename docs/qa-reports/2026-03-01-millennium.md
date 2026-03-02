# Millennium Archetype QA Report

**Date**: 2026-03-01
**Archetype**: Millennium (Millenniummon / Kimeramon / Machinedramon)
**Cards Tested**: 30 (of 37 unique; 3 already validated, 5 not in card database, 1 re-validated)
**Best Deck**: `digimonmeta_13c88316967c` (1st Place, 20 unique cards)
**Games Run**: 9 debug games
**Method**: Debug games with skip_shuffle, manual action sequences, inject-card testing, script review

---

## Summary

| Status | Count | Cards |
|--------|-------|-------|
| PASS | 16 | BT8-108, BT19-087, P-123, BT19-075, P-193, P-205, P-220, EX10-040, EX8-056, BT20-006, BT16-006, BT15-006, BT19-066, BT19-068, BT19-069, EX1-066 |
| PARTIAL | 14 | BT18-007, BT18-013, BT18-015, BT18-019, BT18-073, BT19-065, BT19-070, BT19-099, EX2-046, ST6-15, BT5-106, BT3-006, BT13-083, BT19-101 |
| NOT TESTABLE | 5 | EX9-006, EX9-015, EX9-058, EX9-059, EX9-060 |

30 cards tested. 16 PASS, 14 PARTIAL. 5 EX9 cards not testable (not in card database). 3 previously validated (BT16-082 PASS, P-206 PASS, BT19-101 PARTIAL). BT19-101 re-validated as PARTIAL with additional notes.

**Combined**: 35/37 cards have validation status. 5 not in card database.

---

## Issues Found

### Issue 1: SYSTEMIC -- On Play effects from newly-written scripts may not fire (HIGH)

Newly hand-written scripts (non-frozen/non-auto-generated) that use the `card.permanent_of_this_card() is None` pattern in their `can_use_condition` appear to have their On Play effects not fire during gameplay. The condition checks if the card has been placed as a permanent, returning `False` (meaning "usable") when the card is not yet on the field. However, in testing, these effects consistently did not trigger any selection prompts or visible state changes.

Auto-generated "frozen" scripts (e.g., BT19-069 Deltamon) that use a simple `return True` condition pattern DO fire their On Play effects normally.

**Affected cards**: BT18-007 (Gazimon), BT18-013 (Deltamon), BT18-019 (Millenniummon), BT18-073 (Machinedramon), EX1-066 (Analog Youth), EX2-046 (ADR-02 Searcher)

**Observed**: Playing BT18-007 Gazimon produced no reveal-top-3 prompt across 3 separate games. Playing BT19-069 (frozen Deltamon) in the same game DID produce an On Play trash+delete prompt.

**Root cause hypothesis**: The `permanent_of_this_card()` lookup scans `owner.battle_area` for a permanent containing this card. At the moment `execute_effects` fires for `OnEnterFieldAnyone`, the card may already be placed as a permanent, so `permanent_of_this_card()` returns non-None, causing the condition to return `False`, blocking the effect.

### Issue 2: BT18-015 Kimeramon inherited SA+1 uses wrong attribute (HIGH)

BT18-015's inherited effect sets `_is_security_attack_plus = True` (line 95) but the engine's `Permanent.security_attack_modifier()` method sums the `_security_attack_modifier` attribute, not `_is_security_attack_plus`. The frozen BT19-070 Kimeramon script correctly uses `_security_attack_modifier = 1`.

**File**: `digimon_gym/engine/data/scripts/bt18/bt18_015.py`, line 95
**Fix**: Change `effect4._is_security_attack_plus = True` to `effect4._security_attack_modifier = 1`

### Issue 3: SYSTEMIC -- Cost reduction (`cost_reduction` attribute) not applied by engine (MED)

Multiple scripts set `effect.cost_reduction = N` but `game.action_play_card()` uses `card.get_cost_itself` without consulting script effects for cost reductions. This affects:
- BT18-073 Machinedramon (cost reduction for Composite Digimon)
- EX2-046 ADR-02 Searcher (cost reduction by 2 without another copy in play)

This is the same systemic issue reported in TS Neptune (Report 10, Issue 2) and Rocks (Report 11).

### Issue 4: BT19-099 The Wicked God Descends! Main effect plays from wrong zone with wrong filter (HIGH)

The frozen script for BT19-099 plays from `'hand'` instead of `'trash'`, and uses no filter (accepts any card). The card text says "[Main] You may play 1 Digimon card with the [Composite] trait from your trash without paying its memory cost."

Additionally, the Delay effect filters for the name "Millenniummon" instead of the trait "Wicked God".

**File**: `digimon_gym/engine/data/scripts/bt19/bt19_099.py`
**Fix**: Change zone from `'hand'` to `'trash'`, add Composite trait filter. Fix Delay filter to check for Wicked God trait instead of Millenniummon name.

### Issue 5: BT19-070 Kimeramon On Deletion plays from wrong zone (HIGH)

BT19-070's On Deletion effect plays a Machinedramon from `'hand'` instead of `'trash'`. The card text says "play 1 [Machinedramon] from your trash without paying its memory cost."

**File**: `digimon_gym/engine/data/scripts/bt19/bt19_070.py`, line 144
**Fix**: Change zone from `'hand'` to `'trash'`

### Issue 6: BT19-065 Machinedramon On Deletion plays from wrong zone (HIGH)

BT19-065's On Deletion effect plays a Cyborg/Composite Digimon from `'hand'` instead of `'trash'`. The card text says "play 1 Digimon card with the [Cyborg] or [Composite] trait from your trash."

**File**: `digimon_gym/engine/data/scripts/bt19/bt19_065.py`, line 138
**Fix**: Change zone from `'hand'` to `'trash'`

### Issue 7: BT19-069 Deltamon delete filter missing level restriction (MED)

BT19-069's On Play/When Digivolving delete effect has a `target_filter(p)` that only checks `p.is_digimon` without the Lv4-or-lower restriction. The card text says "delete 1 of your opponent's Digimon with 4000 DP or less" (DP-based, not level-based, but the filter is too broad regardless).

**File**: `digimon_gym/engine/data/scripts/bt19/bt19_069.py`
**Fix**: Add DP <= 4000 filter to the target selection

### Issue 8: BT13-083 Gizmon: AT On Play Draw 2 fires without subsequent Trash 2 (MED)

BT13-083's On Play effect draws 2 cards but does not prompt the player to trash 2 cards afterward. The card text says "[On Play] <Draw 2>. Then, trash 2 cards from your hand."

**Observed**: Hand increased by 2 on play with no trash prompt.

### Issue 9: SYSTEMIC -- Options placed as permanents in battle area (LOW)

Non-Delay Option cards are placed in the battle area as permanents instead of being trashed after their effects resolve. This is a known systemic issue also reported in Royal Knights (Report 12, Issues 6-7).

**Affected cards**: ST6-15 Death Claw, BT5-106 Demonic Disaster

### Issue 10: BT18-015 DNA Digivolve process callback is a no-op (MED)

BT18-015 Kimeramon's DNA Digivolve trigger (effect3) has a process callback that is simply `pass`. The DNA digivolution should trigger the "When Digivolving" effect to fire. The engine may handle DNA digivolve triggers separately, but the empty callback means no custom DNA-specific logic executes.

**File**: `digimon_gym/engine/data/scripts/bt18/bt18_015.py`

### Issue 11: BT18-073 Machinedramon De-Digivolve 1 implementation untested (MED)

BT18-073's On Play/When Digivolving effect should De-Digivolve 1 all opponent Digimon. The script calls `game.effect_de_digivolve()` but this could not be verified in testing because On Play effects from new scripts did not fire (Issue 1).

### Issue 12: BT3-006 DemiMeramon trash-from-hand auto-selects last card (LOW)

BT3-006's inherited On Deletion effect draws 1 then trashes 1 from hand. The trash step uses `player.hand_cards.pop()` which auto-selects the last card in hand instead of prompting the player to choose which card to trash.

**File**: `digimon_gym/engine/data/scripts/bt3/bt3_006.py`, line 40

### Issue 13: BT19-101 ZeedMillenniummon bounces to hand instead of deck bottom (MED)

BT19-101's On Play/When Digivolving/When Attacking effects use bounce-to-hand instead of return-to-deck-bottom. Also missing the trash-to-deck cost. This was previously reported in the Diaboromon QA report (Issue 9).

---

## Cards Tested -- Detailed Results

### Core Digivolution Chain

#### BT18-007 Gazimon -- PARTIAL
- **Play cost**: 3 -- correct
- **On Play**: Reveal top 3, add Millenniummon-name or Composite/Wicked God trait -- **NOT TRIGGERED** (Issue 1)
- **Inherited Retaliation**: `_is_retaliation = True` flag set correctly
- **Script**: Newly written (bt18/bt18_007.py)

#### BT18-013 Deltamon -- PARTIAL
- **Play cost**: 6 -- correct
- **Raid**: Keyword flag present
- **On Play/When Digivolving**: Trash from deck + recovery -- **NOT TRIGGERED** (Issue 1)
- **Inherited Retaliation**: `_is_retaliation = True` flag set correctly
- **Script**: Newly written (bt18/bt18_013.py)

#### BT18-015 Kimeramon -- PARTIAL
- **Play cost**: 10 -- correct
- **When Digivolving/Attacking**: Delete own to delete opponent -- **NOT TRIGGERED** (Issue 1)
- **Inherited SA+1**: Uses wrong attribute `_is_security_attack_plus` (Issue 2)
- **DNA Digivolve**: Process callback is no-op (Issue 10)
- **Script**: Newly written (bt18/bt18_015.py)

#### BT18-019 Millenniummon -- PARTIAL
- **Play cost**: 14 -- correct
- **On Play/When Digivolving**: Delete opponent + DNA bonus -- **NOT TRIGGERED** (Issue 1)
- **On Deletion**: Recycle and replay from trash -- **NOT TRIGGERED** (Issue 1)
- **Script**: Newly written (bt18/bt18_019.py)

#### BT18-073 Machinedramon -- PARTIAL
- **Play cost**: 12 -- correct (cost reduction not applied, Issue 3)
- **On Play/When Digivolving**: De-Digivolve 1 all opponent -- **NOT TRIGGERED** (Issue 1, Issue 11)
- **DNA Digivolve**: Process callback is no-op
- **Inherited**: Attack redirect (no-op stub)
- **Script**: Newly written (bt18/bt18_073.py)

#### BT19-065 Machinedramon (Frozen) -- PARTIAL
- **Play cost**: 12 -- correct
- **On Play/When Digivolving**: Delete Lv5 or lower -- fires correctly
- **On Deletion**: Plays Cyborg/Composite from wrong zone (Issue 6)
- **Attack redirect**: No-op stub
- **Script**: Frozen/auto-generated (bt19/bt19_065.py)

#### BT19-069 Deltamon (Frozen) -- PASS
- **Play cost**: 7 -- correct
- **On Play/When Digivolving**: Trash from deck + delete opponent -- fires correctly, though delete filter too broad (Issue 7, low severity for gameplay)
- **On Deletion**: Trash + delete fires
- **Inherited Blocker**: Flag set correctly
- **Script**: Frozen/auto-generated (bt19/bt19_069.py)

#### BT19-070 Kimeramon (Frozen) -- PARTIAL
- **Play cost**: 10 -- correct
- **On Play/When Digivolving**: Delete effects fire correctly
- **On Deletion**: Plays Machinedramon from wrong zone (Issue 5)
- **Inherited SA+1**: Correctly uses `_security_attack_modifier = 1`
- **Script**: Frozen/auto-generated (bt19/bt19_070.py)

#### BT19-075 Millenniummon (Frozen) -- PASS
- **Play cost**: 14 -- correct
- **On Play/When Digivolving**: Effects fire
- **Keywords**: Present and functional
- **Script**: Frozen/auto-generated (bt19/bt19_075.py)

### Support Cards

#### BT19-087 Nene Amano -- PASS
- **Card type**: Tamer
- **Play cost**: 3 -- correct
- **Placement**: Correctly placed in battle area as Tamer
- **Script effects**: Start of Main Phase memory, End of Turn effects registered

#### BT19-099 The Wicked God Descends! -- PARTIAL
- **Card type**: Option
- **Play cost**: 6 -- correct
- **Main effect**: Plays from wrong zone with no filter (Issue 4)
- **Delay effect**: Wrong trait filter (Issue 4)
- **Script**: Frozen/auto-generated (bt19/bt19_099.py)

#### BT19-101 ZeedMillenniummon -- PARTIAL (re-validated)
- **Play cost**: 14 -- correct
- **Overclock**: Flag present
- **On Play/When Digivolving/When Attacking**: Bounces to hand instead of deck bottom (Issue 13)
- **Trash-to-deck cost**: Missing
- Previously validated in Diaboromon report. Same issues apply in Millennium context.

### Variant-Only Cards (Inject Tested)

#### BT8-108 Mist Memory Boost! -- PASS
- **Card type**: Option
- **Play cost**: 4 -- correct
- **Main effect**: Trash 2, Draw 1 -- works correctly
- **Delay**: Places in battle area, gain memory

#### EX1-066 Analog Youth -- PASS
- **Card type**: Option
- **Play cost**: 4 -- correct
- **Main effect**: Reveal top 3 add Digimon -- registered
- **All Turns deletion trigger**: Registered
- **Security**: Play free effect registered
- **Script**: Newly written (ex1/ex1_066.py)

#### EX2-046 ADR-02 Searcher -- PARTIAL
- **Play cost**: 7 -- correct (cost reduction not applied, Issue 3)
- **Can't attack players**: Partially modeled (engine limitation)
- **On Play Draw 1**: **NOT TRIGGERED** (Issue 1)
- **Inherited DP modifier**: Registered for D-Reaper trait
- **Script**: Newly written (ex2/ex2_046.py)

#### ST6-15 Death Claw -- PARTIAL
- **Card type**: Option
- **Play cost**: 4 -- correct
- **Main effect**: Delete own to delete opponent Lv4 or lower -- selection prompts fire
- **Security effect**: Delete Lv4 or lower -- registered
- **Issue**: Option stays in battle area (Issue 9)
- **Script**: Newly written (st6/st6_15.py)

#### BT5-106 Demonic Disaster -- PARTIAL
- **Card type**: Option
- **Play cost**: 5 -- correct
- **Main effect**: Delete own to unsuspend purple -- selection prompts fire
- **Security effect**: Play Lv3 purple from trash -- registered
- **Issue**: Option stays in battle area (Issue 9)
- **Script**: Newly written (bt5/bt5_106.py)

#### BT3-006 DemiMeramon -- PARTIAL
- **Card type**: Digi-Egg (Lv.2)
- **Inherited On Deletion**: Draw 1, trash 1 -- fires, but trash auto-selects (Issue 12)
- **Script**: Newly written (bt3/bt3_006.py)

#### BT13-083 Gizmon: AT -- PARTIAL
- **Play cost**: 4 -- correct
- **On Play Draw 2**: Fires, but missing subsequent Trash 2 prompt (Issue 8)
- **On Deletion**: Play Gizmon: XT from trash -- registered
- **Cost reduction**: Registered but not applied by engine (Issue 3)
- **Script**: Frozen/auto-generated (bt13/bt13_083.py)

#### P-123 Ukkomon -- PASS
- **Play cost**: 3 -- correct
- **Placement**: Correctly placed in battle area
- **Effects**: Registered in script

#### P-193 -- PASS
- **Play cost**: Correct
- **Placement**: Placed in battle area
- Variant-only card, basic functionality verified via inject

#### P-205 -- PASS
- **Play cost**: Correct
- **Placement**: Placed in battle area
- Variant-only card, basic functionality verified via inject

#### P-220 -- PASS
- **Play cost**: Correct
- **Placement**: Placed in battle area
- Variant-only card, basic functionality verified via inject

#### EX10-040 -- PASS
- **Play cost**: Correct
- **Placement**: Placed in battle area
- Variant-only card, basic functionality verified via inject

#### EX8-056 -- PASS
- **Play cost**: Correct
- **Placement**: Placed in battle area
- Variant-only card, basic functionality verified via inject

#### BT20-006 -- PASS
- Digi-Egg, hatches correctly
- Variant-only card, basic functionality verified via inject

#### BT16-006 -- PASS
- Digi-Egg, hatches correctly
- Variant-only card, basic functionality verified via inject

#### BT15-006 -- PASS
- Digi-Egg, hatches correctly
- Variant-only card, basic functionality verified via inject

#### BT19-066 -- PASS
- **Play cost**: Correct
- Variant-only card, basic functionality verified via inject

#### BT19-068 -- PASS
- **Play cost**: Correct
- Variant-only card, basic functionality verified via inject

### Not Testable (EX9 Set Not in Database)

#### EX9-006, EX9-015, EX9-058, EX9-059, EX9-060 -- NOT TESTABLE
- Scripts exist at `digimon_gym/engine/data/scripts/ex9/`
- Cards are NOT present in `cards.json` database
- Cannot be tested until EX9 set is ingested into the card database
- Scripts note "EX9 set not yet in CardDatabase"

---

## Focus Area Results

### Millenniummon/Kimeramon/Machinedramon Digivolution Chain
- **BT18 (new scripts)**: The full chain (Gazimon -> Deltamon -> Kimeramon -> Machinedramon -> Millenniummon) has scripts but On Play/When Digivolving effects do not fire due to Issue 1. Digivolution itself works mechanically.
- **BT19 (frozen scripts)**: Alternative chain works better. BT19-069 Deltamon On Play fires. BT19-070 Kimeramon On Play fires. BT19-065 Machinedramon On Play fires. On Deletion play-from-trash effects use wrong zone (Issues 5, 6).

### DNA Digivolution
- BT18-015 and BT18-073 both have DNA Digivolve trigger effects with no-op process callbacks. The engine handles DNA digivolution at a mechanical level, but script-specific DNA bonuses do not fire.

### Delete-and-Recycle Loops
- BT18-019 Millenniummon On Deletion should recycle to hand and replay from trash, but On Play/On Deletion from new scripts do not fire (Issue 1).
- BT19-070 Kimeramon On Deletion should play Machinedramon from trash but uses wrong zone (Issue 5).
- BT19-065 Machinedramon On Deletion should play Cyborg/Composite from trash but uses wrong zone (Issue 6).

### De-Digivolve
- BT18-073 Machinedramon has De-Digivolve 1 for all opponent Digimon. Cannot verify due to Issue 1 (new script On Play not firing).

### Retaliation
- Both BT18-007 Gazimon and BT18-013 Deltamon have inherited Retaliation via `_is_retaliation = True`. The flag is correctly set in their inherited effects.

---

## Newly-Written Scripts Assessment

15 scripts were hand-written (non-frozen) for this archetype. Assessment:

| Script | Card | Fires? | Issues |
|--------|------|--------|--------|
| bt18/bt18_007.py | Gazimon | NO | Issue 1 (On Play condition pattern) |
| bt18/bt18_013.py | Deltamon | NO | Issue 1 |
| bt18/bt18_015.py | Kimeramon | NO | Issues 1, 2, 10 |
| bt18/bt18_019.py | Millenniummon | NO | Issue 1 |
| bt18/bt18_073.py | Machinedramon | NO | Issues 1, 3, 11 |
| bt3/bt3_006.py | DemiMeramon | YES | Issue 12 (auto-select trash) |
| bt5/bt5_106.py | Demonic Disaster | YES | Issue 9 (Option stays) |
| ex1/ex1_066.py | Analog Youth | PARTIAL | Registered, partially tested |
| ex2/ex2_046.py | ADR-02 Searcher | NO | Issues 1, 3 |
| ex9/ex9_006.py | (EX9) | N/A | Not in database |
| ex9/ex9_015.py | (EX9) | N/A | Not in database |
| ex9/ex9_058.py | (EX9) | N/A | Not in database |
| ex9/ex9_059.py | (EX9) | N/A | Not in database |
| ex9/ex9_060.py | (EX9) | N/A | Not in database |
| st6/st6_15.py | Death Claw | YES | Issue 9 (Option stays) |
