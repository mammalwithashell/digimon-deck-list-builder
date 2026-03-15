# Gameplay QA Report — TS Neptune

## Test Setup
- **Date**: 2026-03-03
- **Archetype**: TS Neptune
- **Game IDs**: a1edc0db (primary), b7b85ef6, 9dbd2999 (supplemental), plus several setup games
- **Total Turns Played**: ~12 (focused card-by-card testing via API)
- **Focus Areas**: play costs, cost reduction, On Play effects, keywords, DP aura, game stability
- **Pre-game fixes**: BT24-030/040/041 duplicate cost reduction removed, BT24-030 On Play/When Digivolving callbacks added, BT24-030 unsuspend fixed

## Summary
- **Total Issues Found**: 12
- Critical: 2 | High: 4 | Medium: 4 | Low: 2
- **Pre-test fixes applied**: 3 (duplicate cost reduction x3, Neptunemon callbacks, Neptunemon unsuspend)

## Detailed Findings

### Issue 1: BT24-030/040/041 duplicate cost reduction (-10 instead of -5)
- **Card(s)**: BT24-030 Neptunemon, BT24-040 Venusmon, BT24-041 Minervamon
- **Severity**: high
- **Category**: play_cost
- **Expected**: Each card should have a single conditional -5 cost reduction
- **Actual**: Transpiler generated two identical BeforePayCost effects, applying -10 total
- **Fix**: Removed duplicate effect2 block from each script
- **Status**: **FIXED** (pre-test)

### Issue 2: BT24-030 On Play/When Digivolving missing process callbacks
- **Card(s)**: BT24-030 — Neptunemon
- **Severity**: high
- **Category**: effect
- **Expected**: On Play and When Digivolving should return all opponent Digimon with fewest digi cards to deck bottom
- **Actual**: Effects had timing/conditions but no `set_on_process_callback()` — effects fired as no-ops
- **Fix**: Added `_neptunemon_bottom_deck()` helper and process callbacks for both effects
- **Status**: **FIXED** (pre-test)

### Issue 3: BT24-030 unsuspend targets any permanent instead of self
- **Card(s)**: BT24-030 — Neptunemon
- **Severity**: medium
- **Category**: effect
- **Expected**: "When this Digimon suspends, it may unsuspend" — should unsuspend self
- **Actual**: Used `game.effect_select_own_permanent()` to let player pick any permanent
- **Fix**: Changed to `perm.unsuspend()` directly on self
- **Status**: **FIXED** (pre-test)

### Issue 4: BT24-031 Elecmon On Play trashes from hand instead of revealing from deck
- **Card(s)**: BT24-031 — Elecmon
- **Severity**: high
- **Category**: effect
- **Expected**: On Play should reveal top 3 cards of deck and add 1 Iliad or TS trait card to hand
- **Actual**: Prompts "Trash from hand" selection instead of deck reveal
- **Evidence**: Game a1edc0db — playing Elecmon shows wrong selection prompt
- **Status**: OUTSTANDING

### Issue 5: BT24-031 Elecmon inherited effect logic completely inverted
- **Card(s)**: BT24-031 — Elecmon
- **Severity**: medium
- **Category**: effect
- **Expected**: Inherited should allow returning a security card to hand (security-to-hand)
- **Actual**: Script logic is inverted — implementation details incorrect
- **Status**: OUTSTANDING

### Issue 6: BT24-029 Whamon On Play applies wrong effect
- **Card(s)**: BT24-029 — Whamon
- **Severity**: high
- **Category**: effect
- **Expected**: On Play should place a card from hand as bottom digivolution card on a TS Digimon, then that Digimon can't be affected by opponent effects until end of turn
- **Actual**: Applies CANNOT_BE_SELECTED_BY_EFFECT modifier on self instead of the tucking + protection mechanic
- **Evidence**: Game a1edc0db — Whamon On Play fires but applies wrong effect
- **Status**: OUTSTANDING

### Issue 7: BT24-102 Homeros EOT effect is a stub
- **Card(s)**: BT24-102 — Homeros
- **Severity**: medium
- **Category**: effect
- **Expected**: End of Turn should allow reactivating an Olympos XII Digimon's On Play or When Digivolving effect
- **Actual**: EOT callback only suspends Homeros, doesn't trigger any Olympos XII reactivation
- **Status**: OUTSTANDING

### Issue 8: BT24-090 Abyss Sanctuary security swap not implemented
- **Card(s)**: BT24-090 — Abyss Sanctuary: Throne Room
- **Severity**: medium
- **Category**: effect
- **Expected**: OptionSkill should allow swapping a card from hand with a face-up security card
- **Actual**: Security swap mechanic not implemented (stub only)
- **Evidence**: Game b7b85ef6 — option fired but no security swap prompt
- **Status**: OUTSTANDING

### Issue 9: BT24-088 Asuna Shiroki On Play crashes game
- **Card(s)**: BT24-088 — Asuna Shiroki
- **Severity**: critical
- **Category**: game_flow
- **Expected**: On Play should prompt to trash 1 TS/Three Musketeers card from hand then draw 2
- **Actual**: Game crashes during SelectHand phase — game becomes "Not Found" after action
- **Evidence**: Game b7b85ef6 — playing Asuna causes server to lose game state
- **Status**: OUTSTANDING

### Issue 10: BT3-093 Davis Motomiya On Play crashes game
- **Card(s)**: BT3-093 — Davis Motomiya
- **Severity**: critical
- **Category**: game_flow
- **Expected**: On Play should reveal top 3 cards and add blue/green Digimon to hand
- **Actual**: Game crashes immediately upon play — server returns empty response, game not found afterward
- **Evidence**: Game 9dbd2999 — playing Davis causes complete game loss
- **Status**: OUTSTANDING

### Issue 11: BT24-027/028/029 tucking cost not implemented
- **Card(s)**: BT24-027 Lanamon, BT24-028 Divermon, BT24-029 Whamon
- **Severity**: low
- **Category**: effect
- **Expected**: "By placing 1 card from hand as bottom digi card" should be required as a cost before granting keywords/effects
- **Actual**: Keywords/effects granted unconditionally without tucking cost step
- **Notes**: Requires `effect_place_from_hand_as_source()` engine helper (deferred)
- **Status**: OUTSTANDING (deferred — needs engine helper)

### Issue 12: BT24-028 Divermon inherited effect wrong filter and zone
- **Card(s)**: BT24-028 — Divermon
- **Severity**: low
- **Category**: effect
- **Expected**: Inherited should allow playing a Lv.4 or lower TS Digimon from this Digimon's digivolution cards
- **Actual**: Filter checks for "Neptunemon" name instead of Lv.4 TS trait, and targets hand zone instead of digivolution cards
- **Status**: OUTSTANDING

## Cards Tested Successfully

### Game 1 (a1edc0db) — Primary Test
| Card | Name | Result | Notes |
|------|------|--------|-------|
| BT24-031 | Elecmon | PARTIAL | Play cost 3 correct. On Play broken (wrong zone — trash instead of reveal). |
| BT24-027 | Lanamon | PARTIAL | Play cost 5 correct. Decode keyword present. On Play fires but no tucking cost required. |
| BT24-102 | Homeros | PARTIAL | Play cost 5 correct. DP aura not working (existing Issue 3 from Report 10). EOT is a stub. |
| BT24-029 | Whamon | PARTIAL | Play cost 7 correct. On Play fires but applies wrong effect (CANNOT_BE_SELECTED on self). |
| BT24-028 | Divermon | PARTIAL | Play cost 0 correct. Blocker granted unconditionally. DP=None in database. |
| BT24-051 | Merukimon | PASS | Play cost 7 (12-5 with 4 Digimon) correct. DP=17000 (12000+5000 buff). Rush keyword present. |
| BT24-041 | Minervamon | PARTIAL | Keywords (Blocker, Reboot) present. Appeared on field during Merukimon On Play interaction. Cost reduction not directly tested. |
| BT24-030 | Neptunemon | PARTIAL | Play cost 12 correct. On Play bottom-deck fires (fixed this session). Unsuspend fixed. Cost reduction with opponent 2+ Digimon not tested in gameplay. |

### Game 2 (b7b85ef6) — Supplemental
| Card | Name | Result | Notes |
|------|------|--------|-------|
| BT24-090 | Abyss Sanctuary | PARTIAL | Play cost 3 correct. Option trashes after resolving. Security swap not implemented. |
| BT24-088 | Asuna Shiroki | FAIL | Play cost 3 correct. Game crashes during SelectHand phase on On Play effect. |

### Game 3 (9dbd2999) — Supplemental
| Card | Name | Result | Notes |
|------|------|--------|-------|
| BT3-093 | Davis Motomiya | FAIL | Game crashes immediately upon play. |

### Not Tested in Gameplay
| Card | Name | Notes |
|------|------|-------|
| BT24-040 | Venusmon | Cost reduction code verified (<=3 security -> -5), duplicate fix applied. Not played in game. |
| BT24-091 | Tidal Stream | Not tested this session. Previous Report 10 noted Link mechanic issues. |
| LM-028 | Blue Scramble | Not tested this session. Previous Report 10 noted digivolve-from-hand issues. |

## Pre-Test Script Fixes Applied
1. **BT24-030 Neptunemon**: Removed duplicate cost reduction (effect2). Added `_neptunemon_bottom_deck()` helper with process callbacks for On Play and When Digivolving. Fixed unsuspend to target self (`perm.unsuspend()`).
2. **BT24-040 Venusmon**: Removed duplicate cost reduction (effect2).
3. **BT24-041 Minervamon**: Removed duplicate cost reduction (effect2).

## Areas Not Covered
- BT24-030 Neptunemon cost reduction: Could not set up opponent with 2+ Digimon to test -5 condition in live gameplay (code-verified only)
- BT24-040 Venusmon: Not played in any game (cost reduction code-verified)
- BT24-041 Minervamon cost reduction: Not directly tested (appeared during complex interaction)
- BT24-091 Tidal Stream: Link mechanic not retested
- LM-028 Blue Scramble: Digivolve-from-hand not retested
- BT24-030 WhenRemoveField: Completely wrong implementation (suspends opponent instead of protecting ally) — not tested, identified via code review
- Attack-time interactions, inherited effect chains: Not tested
