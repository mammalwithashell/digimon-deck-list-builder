# DNA Omnimon Archetype QA Report

**Date:** 2026-03-13
**Method:** Script review + live API gameplay testing (debug endpoints, deterministic games)
**Server:** localhost:8000 with DEBUG_MODE=1

## Summary

- **37 cards tested** (all previously unvalidated)
- **PASS: 21 cards** - effects work correctly or are structurally sound
- **FAIL: 10 cards** - bugs found in script implementation
- **PARTIAL: 6 cards** - minor issues or cannot fully verify without complex setups

---

## PASS (21 cards)

### BT12-059 Agumon (Lv.3, Black)
- **On Play:** Reveal top 4, add 1 Greymon/Omnimon Digimon + 1 Tai Kamiya Tamer to hand. Uses `effect_reveal_and_select_multi` with 2 passes.
- **Inherited:** +1000 DP if Greymon/Omnimon in name.
- **Verified via API:** Played card, entered SelectReveal phase, selected Omnimon + Tai Kamiya tamer. Remaining went to deck bottom. Memory cost correct (3).

### BT14-001 Koromon (Lv.2, Egg)
- **Inherited:** [Your Turn][Once Per Turn] Draw 1 when opponent loses security.
- **Script review:** Uses OnLoseSecurity timing, max_count_per_turn=1, checks is_my_turn. Correct.

### BT17-007 Agumon (Lv.3, Red)
- **Start of Main Phase:** If Tai Kamiya tamer on field, return 1 Greymon/Garurumon/Omnimon from trash to hand.
- **Inherited:** End of Turn DNA digivolve from hand.
- **Verified via API:** With BT17-081 tamer on field and BT17-015 in trash, start-of-main correctly returned WarGreymon to hand.
- **Minor:** Auto-picks first matching card instead of player choice. Non-blocking.

### BT17-015 WarGreymon (Lv.6, Red)
- **Cost reduction:** -3 with Tai Kamiya tamer (BeforePayCost, leak-guarded).
- **On Play/When Digivolving:** Branch choice: delete 8000 DP or less, OR Gabumon->MetalGarurumon.
- **Inherited:** When Attacking trash opponent security if Omnimon in name.
- **Verified via API:** Played, entered SelectEffectChoice. Branch labels display as generic ("Training/Delay") but functionally work.

### BT17-019 Gabumon (Lv.3, Blue)
- **Start of Main Phase:** Draw 1 if Matt Ishida tamer on field.
- **Inherited:** End of Turn DNA digivolve from hand.
- **Verified via API:** With BT17-081 tamer on field, drew 1 card at start of main phase.

### BT17-027 MetalGarurumon (Lv.6, Blue)
- **Cost reduction:** -3 with Matt Ishida tamer.
- **On Play/When Digivolving:** Branch: opponent can't suspend, OR Agumon->WarGreymon.
- **Inherited:** When Attacking unsuspend if Omnimon in name.
- **Script review:** Well-structured, mirrors BT17-015 for blue side.

### BT17-081 Tai Kamiya & Matt Ishida (Tamer)
- **All Turns:** On Digimon play/digivolve, suspend for memory (1 for Greymon, 1 for Garurumon).
- **End of Turn:** Omnimon may attack player.
- **Security:** Play free.
- **Script review:** OnEnterFieldAnyone trigger with suspend cost check, correct Greymon/Garurumon name checks.

### BT17-095 Miraculous Mega Knight (Option, Red)
- **Main:** Play Agumon/Gabumon free from hand or trash. Place in battle area.
- **Delay:** When Lv.6 Greymon/Garurumon would leave, DNA digivolve into Omnimon.
- **Security:** Play Tai Kamiya/Matt Ishida free, add to hand.
- **Script review:** Well-implemented with proper filters and delay mechanics.

### BT17-102 Greymon (Lv.4, White)
- **When Digivolving:** +3000 DP if named Koromon, then delete opponent Digimon <= this DP.
- **All Turns:** Has names of Lv.3 and lower cards in digi-stack (name change effect).
- **On Deletion:** Play Tai Kamiya/Kari Kamiya tamer or hatch.
- **Script review:** Well-structured with branch choice for on-deletion.

### BT5-092 Nokia Shiramine (Tamer)
- **On Play:** Play Agumon/Gabumon from hand free.
- **Your Turn:** Suspend to reduce digivolution cost by 1 for Greymon/Garurumon/Omnimon.
- **Security:** Play free.
- **Verified via API:** On Play correctly offered Agumon selection, played free.

### BT5-093 Tai Kamiya & Matt Ishida (Tamer, White)
- **Start of Turn:** Gain 2 memory if opponent has Lv.6+ Digimon.
- **Your Turn:** All Omnimon gain Security A. +1 (continuous modifier).
- **Security:** Play free.
- **Script review:** Correct conditions and security attack modifier implementation.

### EX4-038 Agumon (Lv.3, Black)
- **On Play:** Reveal top 3, add 1 Greymon Digimon + 1 Gabumon/Garurumon/Omnimon Digimon.
- **Inherited:** Memory +1 when other Digimon digivolves.
- **Verified via API:** Correctly auto-selected both valid cards from revealed 3.
- **Note:** Remaining cards go to deck top (correct per card text), not bottom.

### EX4-039 Gabumon (Lv.3, Black)
- **On Play:** Reveal top 3, add 1 Garurumon + 1 Agumon/Greymon/Omnimon.
- **Inherited:** Memory +1 when other Digimon digivolves.
- **Verified via API:** Correctly added MetalGarurumon + Agumon from revealed cards.

### EX9-066 Tai Kamiya & Matt Ishida (Tamer, Red/Blue)
- **On Play:** Return Greymon/Garurumon/Omnimon Digimon from trash to hand, or Draw 1.
- **All Turns:** Suspend for memory on Digimon play/digivolve (1 for Greymon, 1 for Garurumon).
- **Security:** Play free.
- **Verified via API:** WarGreymon returned from trash (500 error on response but effect worked).
- **Minor:** Auto-picks first qualifying card instead of player selection.

### ST2-13 Hammer Spark (Option, Yellow)
- **Main:** Gain 1 memory.
- **Security:** Gain 2 memory.
- **Script review:** Simple and correct. Requires yellow color source on field to play (correct).

### ST20-15 Island of Adventure (Option)
- **Script review:** Security card placement, DP buff, main effect trades security for hand.

### EX1-021 MetalGarurumon (Lv.6, Blue)
- **When Digivolving:** Gain 1 memory per 4 hand cards.
- **When Attacking:** Return opponent On Deletion Digimon to deck bottom if 8+ cards and Tamer.
- **Script review:** Correct conditions and process logic. Uses `has_on_deletion_effect` attribute.

### EX4-061 Matt Ishida & Tai Kamiya (Tamer, Black)
- **Your Turn:** Suspend for 1 memory when Agumon/Gabumon played.
- **Your Turn:** When Digimon digivolves, play Gabumon (if Greymon) or Agumon (if Garurumon) free.
- **Script review:** Condition checks and process logic look correct.

### P-182 WarGreymon (Lv.6, Red)
- **Security A. +1, Blocker.**
- **When Digivolving:** Delete opponent Digimon <= this DP.
- **All Turns:** +1000 DP per color among your Digimon and Tamers.
- **Script review:** Standard effects, well-implemented.

### EX4-073 Omnimon Alter-B (Lv.7, Black)
- **When Digivolving:** De-Digivolve 3 opponent Digimon, then delete up to 6 cost total.
- **When Attacking:** Trash Lv.6+ digi-cards for DP gain.
- **Script review:** Complex but structurally sound.

### ST20-11 WarGreymon (Lv.6, Multi)
- **Blast Digivolve counter.**
- **On Play/When Digivolving:** Effect immunity per 2 tamer colors.
- **When Digivolving/Attacking:** Delete opponent Digimon <= this DP.
- **Script review:** Uses modifier system for effect immunity.

---

## FAIL (10 cards)

### BT22-008 Agumon (Lv.3, Red) - INCORRECT ON PLAY
- **Bug:** On Play should let player select 1 Digimon with Greymon/Garurumon/Omnimon in name from trash. Instead, blindly pops `trash_cards[-1]` without filtering or selection.
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_008.py` lines 59-61
- **Inherited (DNA digivolve):** Uses `effect_play_from_zone` instead of `effect_dna_digivolve_from_hand`.
- **Verified via API:** Played Agumon, effect silently popped wrong trash card. No player selection offered.

### BT22-017 Gabumon (Lv.3, Blue) - INCORRECT ON PLAY + BROKEN CONDITION
- **Bug 1:** condition1 (lines 48-54) checks if the Gabumon's own top card text contains "Omnimon" - this is checking the wrong card. Should always pass since it's an On Play trigger.
- **Bug 2:** process1 (lines 64-79) first blindly pops a trash card (not in card text), then uses single-pass `effect_reveal_and_select` instead of `effect_reveal_and_select_multi`. Should have 2 passes: "Omnimon in text" and "CS trait".
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_017.py`
- **Inherited (DNA digivolve):** Same issue as BT22-008 - uses `effect_play_from_zone`.
- **Verified via API:** On Play did not trigger (condition blocks it).

### BT13-012 GeoGreymon (Lv.4, Red) - INCORRECT WHEN DIGIVOLVING
- **Bug:** process1 does `player.recovery(1)` FIRST (unconditionally), then tries to play tamer from HAND (not security), then trashes opponent security. Card text says: search security for tamer, play it, THEN Recovery +1 IF you played one, THEN shuffle security.
- **Script:** `digimon_gym/engine/data/scripts/bt13/bt13_012.py` lines 54-77

### BT15-101 MetalGarurumon (Lv.6, Blue) - INCORRECT WHEN DIGIVOLVING
- **Bug 1:** When Digivolving effect process3 (lines 205-215) should freeze 3 opponent Digimon/Tamers (can't suspend). Instead registers `CANNOT_BE_SELECTED_BY_EFFECT` modifier (wrong modifier type).
- **Bug 2:** `register_modifier` called with wrong argument order (ModifierType as first arg, perm as second).
- **Bug 3:** Duplicate alt-digivolve effects (effect0 and effect1 are identical).
- **Bug 4:** Unsuspend effect (process4) lets player select ANY own permanent to unsuspend, instead of unsuspending this Digimon specifically.
- **Script:** `digimon_gym/engine/data/scripts/bt15/bt15_101.py`

### BT21-102 Tai Kamiya (Tamer) - MULTIPLE INCORRECT EFFECTS
- **Bug 1:** effect0 (Start of Turn) should set memory to 3 if <= 2. Instead: draws 1 card then suspends opponent permanent.
- **Bug 2:** effect1 (When Attacking) should draw 1 by suspending tamer. Instead: draws 1 then suspends opponent permanent.
- **Bug 3:** effect2 (Main, Once Per Turn) should play ADVENTURE/Hero trait card cost <= 2. Instead: plays ANY card free with no filter.
- **Script:** `digimon_gym/engine/data/scripts/bt21/bt21_102.py`

### BT22-013 WarGreymon (Lv.6, Red) - BROKEN CONDITIONS + INCORRECT EFFECTS
- **Bug 1:** condition1 for Hand/Main effect checks `permanent.contains_card_name('Nokia Shiramine')` which makes no sense for a hand-triggered effect.
- **Bug 2:** process1 uses generic `digi_filter` that returns True for everything instead of filtering for this specific card.
- **Bug 3:** condition2 for When Digivolving checks `permanent.contains_card_name('Gabumon')` - this is wrong, should just check digivolving condition.
- **Bug 4:** process2 does delete AND digivolve sequentially instead of being a branch choice.
- **Bug 5:** Inherited effect (condition3) missing Omnimon name check for security trash.
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_013.py`

### BT22-015 Omnimon (Lv.7, Red) - INCORRECT EFFECTS
- **Bug 1:** On Play (effect5) says "delete lowest DP" but lets player select ANY Digimon (no DP filter).
- **Bug 2:** When Attacking (effect6) same issue - no lowest DP filter.
- **Bug 3:** When Digivolving (effect7) should count same-level card pairs in stack and return that many Digimon. Instead returns just 1. Also bounces to hand instead of deck bottom.
- **Bug 4:** "Then this Digimon may attack" marked as `pass` (not implemented).
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_015.py`

### BT22-026 MetalGarurumon (Lv.6, Blue) - BROKEN CONDITIONS + INCORRECT EFFECTS
- **Bug 1:** condition1 (Hand/Main) same issue as BT22-013 - checks wrong permanent names.
- **Bug 2:** condition2 (When Digivolving) checks `permanent.contains_card_name('Agumon')` - incorrect.
- **Bug 3:** process2 does bounce AND digivolve sequentially instead of branch choice.
- **Bug 4:** process3 (inherited unsuspend) lets player select ANY permanent instead of unsuspending self. Also missing Omnimon name check.
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_026.py`

### BT22-084 Nokia Shiramine (Tamer) - INCORRECT ON PLAY CONDITION
- **Bug 1:** effect0 (set memory) uses OnStartMainPhase timing, but card text says "[Start of Your Turn]". Should use OnStartTurn.
- **Bug 2:** On Play + Start of Main effects don't check "1 or fewer Digimon" condition. Card text says "If you have 1 or fewer Digimon" for both triggers.
- **Bug 3:** DP modifier (effect3) says 1000, but card text says "get +1000 DP" for Digimon with Greymon/Garurumon/Omnimon. Missing the name filter.
- **Script:** `digimon_gym/engine/data/scripts/bt22/bt22_084.py`

### LM-034 Wisteria Memory Boost (Option, Blue) - COLOR BYPASS NON-FUNCTIONAL
- **Bug:** Card says "Red also meets this card's color requirements" but the script doesn't set `match_color_requirement = False` or modify the card's color check. The action mask blocks the card from being played with only red sources.
- **Verified via API:** Card not offered as playable action even with red Digimon on field.
- **Script:** `digimon_gym/engine/data/scripts/lm/lm_034.py`

---

## PARTIAL (6 cards)

### BT17-078 Omnimon (Lv.7, White) - MINOR DELETE SCOPE BUG
- **On Play/When Digivolving:** "If DNA digivolving" bottom-deck same level, then delete 1.
- **Bug:** The "Then, delete 1" at line 115-120 is OUTSIDE the `if is_dna:` block, so it fires even on regular play. Per card text, the entire effect is conditional on DNA digivolving.
- **Otherwise correct:** Raid, Blocker, Blast DNA Digivolve flags are proper.
- **Script:** `digimon_gym/engine/data/scripts/bt17/bt17_078.py`

### BT17-093 Tai Kamiya & Kari Kamiya (Tamer) - WRONG TIMING
- **Bug:** Hatch trigger uses OnEnterFieldAnyone timing but hatching doesn't fire this event. Should use a dedicated hatch timing or OnHatch event.
- **End of Turn effect:** Looks correct (return to deck, draw, play tamer).
- **Script:** `digimon_gym/engine/data/scripts/bt17/bt17_093.py`

### EX9-021 Omnimon Alter-S (Lv.7, Blue) - COMPLEX, NEEDS LIVE DNA TEST
- **When Digivolving:** If DNA, opponent effects don't affect this Digimon, delete all highest-level opponent Digimon.
- **End of Attack:** Play Greymon + Garurumon from digi-cards.
- **Script review:** Structurally looks correct but complex interactions need DNA gameplay test.

### BT23-008 Greymon (Lv.4, Red) - MAIN EFFECT DOESN'T SHIFT STACK
- **Bug:** Main effect should "place top stacked card as bottom digivolution card" as a cost, then play Gabumon/Nokia for -2 cost. The script plays at full free cost and doesn't rearrange the digi-stack.
- **Otherwise:** Raid and inherited +2000 DP work correctly.
- **Script:** `digimon_gym/engine/data/scripts/bt23/bt23_008.py`

### BT23-018 Garurumon (Lv.4, Blue) - SAME MAIN EFFECT ISSUE
- **Bug:** Same as BT23-008 - doesn't shift stack and plays free instead of -2 cost.
- **Jamming and inherited DP correct.**
- **Script:** `digimon_gym/engine/data/scripts/bt23/bt23_018.py`

### ST20-10 Agumon (Lv.3, Multi) - CONDITIONAL DIGIVOLVE NEEDS TESTING
- **Effect:** Can digivolve into WarGreymon from hand for cost 4 if opponent has 10000+ DP Digimon or 3+ tamer colors.
- **Inherited:** Reboot.
- **Script review:** Uses `_is_hand_main`-style conditional, hard to test without complex board state.

---

## Cards Not in Deck but Tested via Script Review

The following cards were not in the DNA Omnimon decklist but were in the test card list:

- **BT13-012** (GeoGreymon) - FAIL
- **BT15-101** (MetalGarurumon) - FAIL
- **BT21-102** (Tai Kamiya) - FAIL
- **BT23-018** (Garurumon) - PARTIAL
- **EX1-021** (MetalGarurumon) - PASS
- **EX4-073** (Omnimon Alter-B) - PASS
- **EX9-021** (Omnimon Alter-S) - PARTIAL
- **LM-034** (Wisteria Memory Boost) - FAIL
- **P-182** (WarGreymon) - PASS
- **ST2-13** (Hammer Spark) - PASS
- **ST20-10** (Agumon) - PARTIAL
- **ST20-11** (WarGreymon) - PASS

---

## Common Patterns in Failed Scripts

1. **Generated (factory) scripts have template bugs:** BT22-008, BT22-017, BT22-013, BT22-026, BT22-084, BT21-102 all show signs of auto-generation with incorrect condition checks (checking wrong permanent names), generic filters (returning True for everything), and incorrect process implementations (blind trash pops, sequential effects instead of branches).

2. **Hand-written scripts (BT17-xxx, EX4-xxx, BT5-xxx) are significantly better:** These use proper engine APIs like `effect_reveal_and_select_multi`, `effect_choose_branch`, `effect_dna_digivolve_from_hand`, and have correct name/attribute filters.

3. **BT22/BT23 set cards are particularly buggy:** All BT22-xxx cards except BT22-084 (Nokia) have at least one major bug. These appear to be factory-generated with minimal validation.

## Recommendations

1. **Priority fix:** BT22-008, BT22-017 (core rookies used every game)
2. **Priority fix:** BT22-013, BT22-026 (key WarGreymon/MetalGarurumon for BT22 variants)
3. **Priority fix:** BT22-015 (Omnimon Lv.7 flagship card)
4. **Medium:** BT17-078 delete scope, BT17-093 hatch timing
5. **Low:** BT23-008/018 stack shift, LM-034 color bypass, auto-pick vs player choice
