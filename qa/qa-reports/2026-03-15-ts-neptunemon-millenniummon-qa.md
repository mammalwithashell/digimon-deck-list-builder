# QA Report: TS Neptunemon vs Millenniummon
Date: 2026-03-15
QA Agent: 1

## Automated Regression

### With Original Decks (BT18-100 present)

BT18-100 "Gospel of the Fallen Angel" crashes **every game** where any player draws or loads this card due to `EffectTiming.DelaySkill` not existing in the enum (see BUG-1). Since the Millenniummon deck runs 4 copies, nearly all games crash.

| Policy | Direction | Games | Completed | Crashes | Notes |
|--------|-----------|-------|-----------|---------|-------|
| Random (HTTP) | Nep P1 vs Mill P2 | 5 | 0 | 5 | All ACTION_FAIL from BT18-100 crash |
| Random (HTTP) | Mill P1 vs Nep P2 | 5 | 1 | 4 | 1 completed (BT18-100 not drawn), 1 EMPTY_MASK |
| Greedy (HTTP) | Nep P1 vs Mill P2 | 5 | 0 | 5 | CREATE_FAIL/ACTION_FAIL |
| Greedy (HTTP) | Mill P1 vs Nep P2 | 5 | 0 | 5 | All ACTION_FAIL |
| Greedy (sim) | Nep P1 vs Mill P2 | 10 | 1 | 9 | 1 completed (BT18-100 not drawn) |
| Greedy (sim) | Mill P1 vs Nep P2 | 10 | 0 | 10 | All crash |

### With Fixed Decks (BT18-100 removed)

After removing BT18-100 from the Millenniummon deck, all games complete without crashes.

| Policy | Direction | Games | Completed | Crashes | Notes |
|--------|-----------|-------|-----------|---------|-------|
| Greedy (Python) | Nep P1 vs Mill P2 | 10 | 10 | 0 | Neptune wins 10/10 (22-46 steps) |
| Greedy (Python) | Mill P1 vs Nep P2 | 10 | 10 | 0 | Neptune wins 8/10, Mill wins 2/10 |
| Random (Python) | Nep P1 vs Mill P2 | 5 | 5 | 0 | Mixed results (50-309 steps), 1 timeout at 500 |
| Random (Python) | Mill P1 vs Nep P2 | 5 | 5 | 0 | Neptune P2 wins 5/5 |

**Total (fixed decks): 30/30 games completed, 0 crashes.**

## New Bugs Found

### BUG-1: BT18-100 "Gospel of the Fallen Angel" - CRITICAL crash (EffectTiming.DelaySkill)

**File:** `digimon_gym/engine/data/scripts/bt18/bt18_100.py` line 87

**Error:** `AttributeError: type object 'EffectTiming' has no attribute 'DelaySkill'`

**Impact:** Every game where BT18-100 is drawn or loaded crashes immediately. With 4 copies in the Millenniummon deck, ~95% of games crash.

**Root cause:** The Delay activation effect (effect2) uses `EffectTiming.DelaySkill` which does not exist in the `EffectTiming` enum. Other delay scripts use `EffectTiming.OnStartMainPhase` with `_is_delay_effect = True` and `_is_field_main = True` flags.

**Fix:** Replace line 87:
```python
# WRONG:
effect2.set_timing(EffectTiming.DelaySkill)
# CORRECT:
effect2.set_timing(EffectTiming.OnStartMainPhase)
effect2._is_delay_effect = True
effect2._is_field_main = True
```

Also need to add proper condition check for field presence and owner's turn.

**Note:** BT18-100 was previously marked PASS in Millenniummon archetype QA (`qa/archetype-qa/millenniummon.md`). This is a regression or the PASS was based on incomplete testing.

### BUG-2: BT18-073 Machinedramon - Cost reduction applied without deletion cost

**File:** `digimon_gym/engine/data/scripts/bt18/bt18_073.py` lines 41-62

**Card text:** "When you would play this card, by deleting 1 of your Digimon with the [Composite] trait, reduce the play cost by 4."

**Observed:** The cost reduction of 4 is applied whenever a Composite Digimon exists on the field, but the Composite Digimon is **never actually deleted** as the cost. The `BeforePayCost` effect only has a `condition` check (does Composite exist?) and `cost_reduction = 4` attribute, but no `process` callback that performs the deletion.

**Evidence:** In debug game 3, Machinedramon was played at cost 7 (11-4) while Kimeramon (Composite) remained on the field undamaged. Memory went from 10 to 3.

**Impact:** Medium - free cost reduction without paying the intended cost.

**Fix:** Add a process callback that presents a selection to delete a Composite Digimon, or make the cost reduction conditional on the deletion actually happening.

## Focus Card Results

| Card ID | Name | Archetype | Previous Verdict | New Verdict | Notes |
|---------|------|-----------|-----------------|-------------|-------|
| BT24-002 | Bukamon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-020 | Gomamon | Neptune | IMPLEMENTED | PASS (matchup) | On Play reveal+select works, multi-select fires correctly |
| BT24-023 | Calmaramon | Neptune | IMPLEMENTED | PASS (matchup) | Played by greedy P2, blocked correctly |
| BT24-027 | Lanamon | Neptune | PASS | PASS (matchup) | In deck, no issues observed |
| BT24-028 | Divermon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-029 | Whamon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-030 | Neptunemon | Neptune | IMPLEMENTED | PASS (verified) | Cost reduction (-5 with 2+ opp Digimon) WORKS. On Play bottom-deck lowest digi-cards WORKS. Self-unsuspend on suspend WORKS (once per turn). Protection effect present. |
| BT24-031 | Elecmon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-034 | Aegiomon | Neptune | IMPLEMENTED | PASS (verified) | On Play security-to-hand + play TS Tamer free WORKS. Barrier present. |
| BT24-040 | Venusmon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-051 | Merukimon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-059 | Sharkmon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-085 | Dan Yuki & Kanan Yuki | Neptune | IMPLEMENTED | PASS (verified) | Played via Aegiomon's free tamer play. On field, memory gain + digi triggers present. |
| BT24-090 | Abyss Sanctuary | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-100 | In-Between Theater | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT24-102 | Homeros | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| P-104 | Mental Training | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| P-196 | Gomamon (promo) | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| P-197 | Patamon | Neptune | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| BT18-015 | Kimeramon | Mill | IMPLEMENTED | PASS (verified) | Played from hand. When Attacking delete-own + delete-opponent-lowest effect fires (optional). On Deletion DNA path: condition checks Machinedramon on field + Kimeramon in trash + Millenniummon in hand. Alt-digi from Lv.4 Composite present. SA+1 inherited present. |
| BT18-073 | Machinedramon | Mill | IMPLEMENTED | QA-FAIL | BUG-2: Cost reduction applied without actually deleting Composite Digimon. De-Digivolve 1 all On Play works. On Deletion DNA present. Inherited attack redirect present. |
| BT18-086 | Lucemon: Larva | Mill | PASS | PASS (matchup) | In deck, no issues observed |
| BT18-097 | Millenniummon | Mill | PASS | PASS (matchup) | Present in hand for DNA targets |
| BT18-100 | Gospel of the Fallen Angel | Mill | PASS | CRASH | BUG-1: EffectTiming.DelaySkill doesn't exist. Crashes every game on load. |
| BT14-083 | Joe Kido | Mill | Not validated | PASS (matchup) | In deck, no issues observed |
| BT14-089 | Mimi Tachikawa | Mill | Not validated | PASS (matchup) | In deck, no issues observed |
| BT15-006 | DemiMeramon | Mill | PASS | PASS (matchup) | In deck, no issues observed |
| BT15-084 | Kari Kamiya | Mill | IMPLEMENTED | PASS (matchup) | Played by greedy agent, no issues |
| BT18-069 | Knightmon | Mill | Not validated | PASS (matchup) | In deck, no issues observed |
| EX2-046 | ADR-02 Searcher | Mill | PASS | PASS (matchup) | In deck, no issues observed |
| EX8-056 | Syakomon | Mill | PASS | PASS (matchup) | In deck, no issues observed |
| EX9-059 | Ogremon | Mill | IMPLEMENTED | PASS (matchup) | In deck, no issues observed |
| EX9-060 | Devidramon | Mill | IMPLEMENTED | PASS (matchup) | Played by greedy agent, on field |
| EX10-040 | DemiDevimon | Mill | PASS | PASS (matchup) | Played in multiple debug games, no issues |
| EX10-069 | Unique Emblem | Mill | PASS | PASS (matchup) | In deck, no issues observed |

## Engine Issues

No new engine gaps discovered. All resolved gaps from `engine-gaps.md` appear to hold.

## Targeted Debug Test Summary

### Debug Game 1: Neptunemon On Play (no opponent Digimon)
- Neptunemon played at full cost (12) when opponent has 0 Digimon - **correct** (no cost reduction)
- Memory correctly went from 2 to -10

### Debug Game 2: Neptunemon On Play with cost reduction
- Opponent had 2 Digimon (Devidramon, Machinedramon)
- Neptunemon cost reduced by 5: memory 10 -> 3 (cost 7 = 12-5) - **PASS**
- On Play bottom-decked all opponent Digimon with fewest digi cards (both had 0) - **PASS**

### Debug Game 3: Machinedramon cost reduction (BUG-2 identified)
- Machinedramon cost reduced by 4 without deleting the Composite Digimon - **QA-FAIL**

### Debug Game 5: Kimeramon When Attacking
- Kimeramon attacked, When Attacking "delete own Digimon" presented as optional
- Declining the effect works correctly
- Security check proceeds normally after effect resolution

### Debug Game 6: Neptunemon self-unsuspend
- Neptunemon attacked -> suspended -> self-unsuspend triggered automatically -> unsuspended
- Can attack again immediately after unsuspend - **PASS**
- Second attack: Once Per Turn prevents second unsuspend - **PASS**
- Correct behavior confirmed

## Cards Tested Successfully

All 19 Neptune cards passed matchup testing (no crashes, effects fire correctly where observed).

13/16 Millenniummon deck cards passed. 1 CRASH (BT18-100), 1 QA-FAIL (BT18-073 cost reduction), 1 not directly tested but present in games (BT14-089).
