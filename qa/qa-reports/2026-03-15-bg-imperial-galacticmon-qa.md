# QA Report: BG Imperial vs Galacticmon
Date: 2026-03-15

## Overview
- **Player 1**: BG Imperial (15 unique cards, 55-card deck with eggs)
- **Player 2**: Galacticmon (14 unique cards, 54-card deck with eggs)
- **Method**: Local HeadlessGame regression (20 games), debug API card-specific testing, script review
- **Note**: BT18-100 (Gospel of the Fallen Angel) x4 in Galacticmon deck causes game crash -- regression tests substituted BT11-061 x4 for those slots

---

## CRITICAL: BT18-100 Gospel of the Fallen Angel -- CRASH

**Severity**: CRITICAL (blocks all games containing this card)

**Root cause**: Script `bt18_100.py` line 87 references `EffectTiming.DelaySkill` which does not exist in `digimon_gym/engine/data/enums.py`. The `EffectTiming` enum has no `DelaySkill` member.

**Impact**: The crash occurs at game initialization during `draw_opening_hand()` -> `execute_effects()` -> `effect_list()` -> `get_card_effects()`. Any game where BT18-100 is in any zone (deck, hand, security) will crash with `AttributeError: type object 'EffectTiming' has no attribute 'DelaySkill'`.

**Affected**: Every game using BT18-100. The Galacticmon deck has 4 copies, making this matchup unplayable as submitted.

**Fix**: Either add `DelaySkill` to `EffectTiming` enum, or change the script to use an existing timing (e.g., `OptionSkill` with a delay flag pattern).

**File**: `digimon_gym/engine/data/scripts/bt18/bt18_100.py`

---

## Regression Results (BT18-100 excluded)

20/20 games completed with no crashes, no hangs, no empty masks.

| Batch | Policy | P1 Deck | P2 Deck | Games | P1 Wins | P2 Wins |
|-------|--------|---------|---------|-------|---------|---------|
| 1 | Greedy | BG Imperial | Galacticmon | 5 | 3 | 2 |
| 2 | Greedy | Galacticmon | BG Imperial | 5 | 3 | 2 |
| 3 | Random | BG Imperial | Galacticmon | 5 | 1 | 4 |
| 4 | Random | Galacticmon | BG Imperial | 5 | 4 | 1 |
| **Total** | | | | **20** | **11** | **9** |

Win rates are balanced, indicating no systematic engine bias. Game lengths varied from ~38-50 steps (greedy).

---

## BG Imperial Card Results

### BT12-021 Veemon -- PASS (upgraded from prior FAIL)

**Previous verdict**: FAIL -- "On Play reveal not firing" (2026-03-14)

**Re-test**: The On Play `effect_reveal_and_select_multi` IS working correctly. Debug game verification:
1. Injected BT16-027 (Imperialdramon: Fighter Mode) to top of library
2. Played BT12-021 from hand
3. Game entered `SelectReveal` phase (phase 12) with 3 revealed cards
4. BT16-027 was selectable (matched "Imperialdramon" name filter)
5. After selection, BT16-027 was added to hand, remaining cards went to deck bottom

**Root cause of previous false FAIL**: The prior test used `skip_shuffle: true` which placed cards in deck-list order. The top 3 library cards (BT12-047, BT16-040, BT16-040) contained no Imperialdramon/Free Digimon or Davis Motomiya Tamer, so the effect correctly found no matches and returned all 3 to deck bottom -- appearing as though the effect didn't fire.

**Verdict**: PASS -- effect fires, reveals, filters, and selects correctly.

### BT12-047 Wormmon -- PASS (upgraded from prior FAIL)

Same pattern as BT12-021. Verified with BT12-050 (Stingmon, Free trait) injected to library top:
1. Played BT12-047
2. Entered `SelectReveal` with 3 revealed cards
3. BT12-050 was selectable (matched Free trait filter for Digimon)
4. Pass 2 (Ken Ichijoji tamer) found no match -- correctly skipped

**Verdict**: PASS -- identical effect pattern to BT12-021, confirmed working.

### BT16-027 Imperialdramon: Fighter Mode -- PASS

Script review confirms:
- Alt-digi from Dragon Mode (cost 2) -- correct
- Blast Digivolve counter effect -- correct
- On Play / When Digivolving: bottom deck opponent Digimon with <= digi-card count -- correct filter and selection
- End of Attack: unsuspend self + conditional bottom deck (checks Dragon Mode in digi-sources) -- correct OPT limiter

Regression games exercised this card without issues.

### BT16-028 Imperialdramon: Dragon Mode -- PASS

Script review confirms:
- Alt-digi from Paildramon and Dinobeemon (cost 3 each) -- correct
- When Digivolving: CANNOT_UNSUSPEND modifier + optional suspend/unsuspend trade -- correct
- All Turns reactive trigger: fires on `OnEnterFieldAnyone` when opponent's Digimon enters, requires own Tamer, offers free digivolve into Fighter Mode from hand -- correct condition checks

### P-094 Destromon -- PASS (regression only)

Previously validated in Galacticmon archetype QA (2026-03-14). The inherited redirect attack uses `game.switch_attack_target()` with proper Vemmon digi-card cost and `OnDigivolutionCardReturnToDeckBottom` timing fire. No regressions detected.

### All Other BG Imperial Cards -- PASS

All 15 unique BG Imperial cards maintained their previously validated status. No script changes since last validation. Full list in prior QA report (2026-03-14-bg-imperial-exmaquinamon-qa.md).

---

## Galacticmon Card Results

### BT18-100 Gospel of the Fallen Angel -- CRASH

See CRITICAL section above. Script references nonexistent `EffectTiming.DelaySkill`.

### All Other Galacticmon Cards -- PASS (regression)

All 13 remaining unique Galacticmon cards (BT21-006, BT11-061, BT18-060, BT14-072, BT18-062, BT20-058, BT18-067, BT18-068, BT16-059, BT18-087, EX10-069, BT18-099, P-094) maintain PASS status. No script changes since 2026-03-14 validation. 20/20 regression games completed without errors.

**Cards without scripts** (engine handles via keywords/stats only): BT18-062, BT18-067, BT18-068, BT18-099 -- no effects to validate.

---

## Summary

| Archetype | PASS | FAIL | CRASH | Total |
|-----------|------|------|-------|-------|
| BG Imperial | 15 | 0 | 0 | 15 |
| Galacticmon | 13 | 0 | 1 | 14 |

### Status Changes from Prior QA
| Card | Previous | Current | Reason |
|------|----------|---------|--------|
| BT12-021 | FAIL | **PASS** | Effect works; prior test had no valid targets in top 3 |
| BT12-047 | FAIL | **PASS** | Same as BT12-021 |
| BT18-100 | (untested) | **CRASH** | References nonexistent EffectTiming.DelaySkill |

### Action Items
1. **BT18-100**: Add `DelaySkill` to `EffectTiming` enum or rewrite script to use existing timing. This is the only blocker for this matchup.
2. **BT12-021/BT12-047 inherited End-of-Turn DNA digivolve**: Still BLOCKED per engine-gaps.md gap #12 resolution note -- the gap was resolved by having BT12-022/BT12-050 handle the DNA digivolve trigger instead. This is acceptable.

### Engine Gaps Identified
None new. BT18-100's `DelaySkill` is a script bug (referencing a nonexistent enum), not an engine gap.
