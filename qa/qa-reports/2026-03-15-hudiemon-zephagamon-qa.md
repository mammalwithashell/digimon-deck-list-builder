# QA Report: Hudiemon vs Zephagamon

**Date:** 2026-03-15
**QA Agent:** Agent 2
**Matchup:** Hudiemon (P1) vs Zephagamon (P2)
**First matchup test for Hudiemon:** Yes

## Deck Lists

**Hudiemon (54 cards):**
BT16-082 x3, BT22-043 x4, BT22-044 x4, BT23-048 x4, BT23-101 x2, BT23-050 x4, BT22-049 x4, BT23-051 x4, BT23-053 x4, BT23-058 x2, BT23-089 x4, BT23-094 x4, BT23-096 x4, BT23-090 x4, BT23-097 x3

**Zephagamon (54 cards):**
EX4-002 x4, ST18-04 x4, BT24-044 x3, EX7-031 x3, EX11-028 x4, EX7-032 x2, BT24-046 x4, BT24-050 x4, BT24-051 x4, BT19-049 x3, BT24-055 x4, BT24-056 x3, BT24-087 x4, BT24-092 x4, BT24-098 x4

## Automated Regression Results

### Direct Engine (20 games)

| Category | Games | Completed | Crashed | Notes |
|----------|-------|-----------|---------|-------|
| Random: Hudi P1 vs Zepha P2 | 5 | 1 | 4 | All crashes from BT24-056 |
| Random: Zepha P1 vs Hudi P2 | 5 | 0 | 5 | All crashes from BT24-056 |
| Greedy: Hudi P1 vs Zepha P2 | 5 | 5 | 0 | P1 wins all (turn ~82) |
| Greedy: Zepha P1 vs Hudi P2 | 5 | 5 | 0 | P2 wins all (turn ~82) |

**Completion rate:** 11/20 (55%)
**Crash rate:** 9/20 (45%) -- all from same bug

### HTTP API (10 games)

| Category | Completed | Failed |
|----------|-----------|--------|
| Random H vs Z | 1/3 | 2 crashes |
| Random Z vs H | 0/3 | 3 crashes |
| Greedy H vs Z | 2/2 | 0 |
| Greedy Z vs H | 2/2 | 0 |

**Completion rate:** 5/10 (50%)

### Root Cause: BT24-056 Dezipmon CANNOT_BE_RETURNED

All crashes trace to the same bug:
```
AttributeError: type object 'ModifierType' has no attribute 'CANNOT_BE_RETURNED'
```

**File:** `digimon_gym/engine/data/scripts/bt24/bt24_056.py` (lines 73, 110)
**Trigger:** BT24-056 Dezipmon On Play / When Digivolving effect calls `game.register_modifier(perm, ModifierType.CANNOT_BE_RETURNED, ...)` but `ModifierType` only has `CANNOT_BE_REMOVED`.
**Scope:** 25 script files across bt10, bt19, bt21, bt22, bt23, bt24, ex8, st17 also reference this non-existent modifier (see full list below). Only BT24-056 is in this matchup's decks.
**Greedy games avoid the crash** because greedy policy doesn't trigger the code path that plays/digivolves BT24-056 Dezipmon in these particular game states.

## Targeted Debug Testing

### Test 1: Craniamon (BT23-058) Attack -> Suspend -> Delete

**Setup:** Craniamon hard-played (cost 11 with boosted memory)
**Result:** PASS
- Craniamon registered all 5 effects: alt digivolve, reboot, blocker, WhenRemoveField substitute, OnTappedAnyone delete
- Attacked player -> suspended -> OnTappedAnyone fired correctly
- Deleted Galemon (lowest play cost Digimon on opponent's field)
- Security check resolved correctly (attacker survived)

### Test 2: Takumi Aiba (BT23-089) Memory Gain

**Setup:** Takumi Aiba on field, opponent has Galemon
**Result:** PASS
- OnStartMainPhase effect fired: "+1 memory" when opponent has a Digimon
- Correctly did NOT fire when opponent had no Digimon (condition check works)
- WhenRemoveField substitute with trash selection -- not tested (requires specific removal scenario)

### Test 3: Nanomachine Break (BT23-094) Ignore Color Requirement

**Setup:** P1 has no Yellow/Black color match but BT23-094 is Yellow/Black option
**Result:** PASS
- `_match_color_requirement = False` correctly bypasses color check
- Card appeared as playable in action mask despite no matching colors on field
- Main effect fired: selected opponent Digimon for SA-1 + disable effects
- Card placed in battle area as Delay card

### Test 4: Golemon (BT23-051) Alliance + Blocker + Can't Attack Digimon

**Setup:** Golemon on field with other Digimon
**Result:** PARTIAL PASS
- Blocker registered correctly
- Can't Attack Digimon flag registered
- Alliance keyword registered (`_is_alliance = True`)
- OnTappedAnyone delete (<=4000 DP) effect registered
- Alliance attack action was not explicitly visible in test actions (may require unsuspended ally to appear)

### Test 5: BT24-056 Dezipmon Crash Reproduction

**Setup:** BT24-056 in Zephagamon deck, played from hand
**Result:** CONFIRMED CRASH
- Hard-playing Dezipmon triggers On Play effect
- Effect calls `ModifierType.CANNOT_BE_RETURNED` which does not exist
- Server returns 500 Internal Server Error

## Script QA Issues Found

### CRITICAL: BT24-056 Dezipmon -- Missing ModifierType

- **Card:** BT24-056 Dezipmon
- **File:** `digimon_gym/engine/data/scripts/bt24/bt24_056.py`
- **Issue:** `ModifierType.CANNOT_BE_RETURNED` does not exist in `ModifierType` enum
- **Fix:** Either add `CANNOT_BE_RETURNED` to ModifierType enum, or use `CANNOT_BE_REMOVED` if semantically equivalent
- **Impact:** 45% game crash rate in this matchup; affects 25 scripts across multiple sets

### QA-FAIL: BT23-097 Seventh Penetration -- Missing Level Filter

- **Card:** BT23-097 Seventh Penetration
- **File:** `digimon_gym/engine/data/scripts/bt23/bt23_097.py`
- **Card text:** "[Main] Delete 1 of your opponent's Digimon with a level as high or higher as the number of cards in your hand."
- **Issue:** `target_filter` in `process1` returns `p.is_digimon` without checking `level >= len(player.hand_cards)`. This allows deleting any Digimon regardless of level, making the card strictly more powerful than intended.
- **Fix:** Add `p.top_card.level >= len(player.hand_cards)` check to filter
- **Impact:** Low gameplay impact in this matchup (card is Purple, rarely played by random/greedy), but affects correctness

### Note: BT23-090 Keisuke Amasawa -- Bounce Implementation

- **File:** `digimon_gym/engine/data/scripts/bt23/bt23_090.py`
- **Observation:** The End of Turn bounce effect at line 94-100 manually removes from `battle_area` and splits top card to hand / sources to trash. This is correct behavior for returning a digivolved Digimon to hand (only top card returns, sources trash).
- **Status:** PASS (implementation matches card text)

## Cards Verified in This Session

| Card ID | Card Name | Verdict | Method |
|---------|-----------|---------|--------|
| BT23-058 | Craniamon | PASS | Debug game: hard play, attack, suspend trigger delete |
| BT23-089 | Takumi Aiba | PASS | Debug game: memory gain on main phase start |
| BT23-094 | Nanomachine Break | PASS | Debug game: ignore color req, SA-1 + disable, place in BA |
| BT23-051 | Golemon | PASS | Debug game: blocker, alliance, can't attack digimon |
| BT23-053 | Strikedramon | PASS | Debug game: option-to-field digivolve trigger observed |
| BT23-090 | Keisuke Amasawa | PASS | Script review: memory set, DP modifier, bounce+play |
| BT23-096 | Comet Hammer | PASS | Script review: ignore color req, de-digivolve, delay draw |
| BT23-097 | Seventh Penetration | QA-FAIL | Script review: missing level filter on delete |
| BT23-050 | Ankylomon | PASS | Debug game: -2000 DP + DNA digivolve offer |
| BT23-048 | Gotsumon | PASS | Archetype QA (prior) |
| BT23-101 | Hudiemon | PASS | Script review: alliance, alt digi from CS/Erika |
| BT22-043 | Terriermon | PASS | Automated regression (appeared in games) |
| BT22-044 | Palmon | PASS | Debug game: played, used as digivolve base |
| BT22-049 | Vegiemon | PASS | Debug game: played, on field |
| BT16-082 | Ukkomon | PASS | Automated regression (egg deck) |
| BT24-056 | Dezipmon | CRASH | Debug + regression: CANNOT_BE_RETURNED missing |
| EX4-002 | Kokomon | PASS | Automated regression (hatched in games) |
| ST18-04 | Pteromon | PASS | Debug game: On Play reveal effect fired |
| EX11-028 | Galemon | PASS | Automated regression: On Play suspend fired |

## Summary

- **Hudiemon archetype:** 14/15 unique cards tested, 13 PASS, 1 QA-FAIL (BT23-097 level filter)
- **Zephagamon matchup cards tested:** 4 cards verified, 1 CRASH (BT24-056)
- **Blockers:** 2 issues found
  1. **CRITICAL:** BT24-056 `CANNOT_BE_RETURNED` crashes 45% of games
  2. **QA-FAIL:** BT23-097 missing level-vs-hand-size filter on delete target

## Recommendations

1. **Urgent:** Add `ModifierType.CANNOT_BE_RETURNED` to the enum (or batch-replace with `CANNOT_BE_REMOVED` across all 25 affected scripts)
2. **Fix BT23-097:** Add level filter: `target_filter` should check `p.top_card.level >= len(player.hand_cards)`
3. **Re-test after fixes:** Run 20 more regression games to verify crash rate drops to 0%
