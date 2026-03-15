# QA Report: Medusa Regression Testing (Post-Fix Verification)

**Date**: 2026-03-09
**Archetype**: Medusa
**Games**: 1 debug game (Game ID: 8aead0e0-8a4e-484c-bf90-7269128a1362)
**Focus**: Verify fixes for Issues 46-51 from prior report, test Medusamon WhenDigivolving, Lamiamon targeting, SelectHand descriptions
**Decklist**: Medusa (digimonmeta) vs Royal Knights (egman) from deck_library.json

## Summary

| Severity | Count |
|----------|-------|
| Critical | 1 |
| High | 2 |
| Medium | 3 |
| Low | 1 |
| **Total** | **7** |

---

## Prior Issue Verification

| Prior Issue | Description | Status |
|-------------|-------------|--------|
| Issue 46 (Critical) | King Drasil OPT cost reduction never decrements | **FIXED** — `record_activation()` now called after cost reduction |
| Issue 47 (Critical) | King Drasil cost reduction skips "may" prompt | **KNOWN LIMITATION** — `calculate_play_cost()` runs during action mask (synchronous), cannot prompt player |
| Issue 48 (High) | Medusamon WhenDigivolving does not fire | **FIXED** — Effect now fires correctly. Delete, token placement, and DP scaling all trigger. |
| Issue 49 (Medium) | Lamiamon opponent hand card taken without choice | **FIXED** — `effect_select_hand_card()` now prompts opponent for selection |
| Issue 50 (Medium) | Lamiamon card placed at security TOP instead of BOTTOM | **FIXED** — Uses `append()` for bottom placement |
| Issue 51 (Low) | BT20-083 Omekamon On Deletion optional auto-accepts | **FIXED** — `execute_deletion_effects()` now uses `effect_choose_branch()` for optional effects |
| Issue 52 (Medium) | SelectHand descriptions show "Play" instead of "Select" | **FIXED** — Selection phase check moved before Play/Trash handlers in `_describe_single_action()` |

---

## New Issues Found

### Issue 1 (Critical): BT24-017 Medusamon DP scaling happens BEFORE delete and tokens

**Card**: BT24-017 Medusamon
**Category**: effect, ordering
**Severity**: Critical

**Card text**: "[When Digivolving] Delete 1 of your opponent's lowest DP Digimon. Then, by returning 2 cards from their trash to the bottom of the deck, they play 2 [Petrification] Tokens. **After, this Digimon gets +2000 DP for each of your opponent's Digimon until their turn ends.**"

**Expected**: DP scaling happens AFTER delete + token placement. With 0 initial opponent Digimon → delete 0 → play 2 tokens → DP = 11000 + 2×2000 = 15000.

**Actual**: DP scaling happens BEFORE delete and tokens. Counts 0 opponent Digimon → +0 DP → then deletes → then plays tokens. Final DP = 11000 (missing +4000).

**Root cause**: In `bt24_017.py` process2, `perm.change_dp(2000 * opp_digimon_count)` is called at the TOP of the function, before `effect_select_opponent_permanent()` and `effect_play_token()`.

**Fix**: Move DP scaling to AFTER token placement.

---

### Issue 2 (High): BT24-017 Medusamon missing trash-to-deck cost for tokens

**Card**: BT24-017 Medusamon
**Category**: effect, cost
**Severity**: High

**Card text**: "...by returning 2 cards from their trash to the bottom of the deck, they play 2 [Petrification] Tokens."

**Expected**: Opponent must return 2 cards from trash to bottom of deck as a cost before tokens are played. If they have <2 cards in trash, tokens should not be played.

**Actual**: Tokens are played unconditionally with no trash-to-deck cost. `game.effect_play_token()` is called directly.

---

### Issue 3 (High): BT24-082 Owen Dreadnought digivolve trigger uses wrong timing flag

**Card**: BT24-082 Owen Dreadnought
**Category**: effect, timing
**Severity**: High

**Card text**: "[Your Turn] When any of your Digimon digivolve into a [Reptile] or [Dragonkin] Digimon, by suspending this Tamer, that Digimon gets +3000 DP for the turn. Then, it may attack."

**Expected**: Effect fires when a Digimon digivolves into a Reptile/Dragonkin.

**Actual**: Effect never fires on digivolve. Script has `is_on_play = True` instead of `is_when_digivolving = True`.

**Root cause**: `bt24_082.py` effect1 sets `effect1.is_on_play = True` — wrong flag. Should be `effect1.is_when_digivolving = True`.

---

### Issue 4 (Medium): BT24-082 Owen Dreadnought digivolve trigger implementation wrong

**Card**: BT24-082 Owen Dreadnought
**Category**: effect, targeting
**Severity**: Medium

**Card text**: "...by suspending **this Tamer**, **that Digimon** gets +3000 DP..."

**Expected**: Cost = suspend THIS tamer (Owen). Reward = +3000 DP to the DIGIVOLVED Digimon.

**Actual**: Process suspends an OPPONENT'S permanent (via `effect_select_opponent_permanent`), applies +3000 DP to self (Owen, a Tamer). Both the cost target and reward target are wrong.

---

### Issue 5 (Medium): BT24-082 Owen Dreadnought Start of Main Phase missing card filter

**Card**: BT24-082 Owen Dreadnought
**Category**: effect, filter
**Severity**: Medium

**Card text**: "[Start of Your Main Phase] By returning this Tamer to the bottom of the deck, you may play 1 [Owen Dreadnought] from your hand..."

**Expected**: Selection shows only Owen Dreadnought cards in hand.

**Actual**: `play_filter` returns `True` for all cards. All hand cards appear as selectable options.

---

### Issue 6 (Medium): BT24-082 Owen Dreadnought Start of Main Phase missing self-bottom-deck cost

**Card**: BT24-082 Owen Dreadnought
**Category**: effect, cost
**Severity**: Medium

**Card text**: "**By returning this Tamer to the bottom of the deck**, you may play 1 [Owen Dreadnought]..."

**Expected**: Owen Dreadnought is returned to the bottom of the deck BEFORE playing from hand. This is a cost.

**Actual**: The process never bottom-decks the tamer. It just plays from hand without paying the cost.

---

### Issue 7 (Low): BT24-017 Medusamon missing Piercing keyword

**Card**: BT24-017 Medusamon
**Category**: keyword, missing
**Severity**: Low

**Card text**: "＜Raid＞ ＜Progress＞ **＜Piercing＞**"

**Expected**: Medusamon has Piercing keyword (when this Digimon attacks and deletes an opponent's Digimon and survives, it performs security checks).

**Actual**: Script only implements Raid (effect0), Progress (effect1), and WhenDigivolving (effect2). No Piercing effect is defined.

---

## Cards Tested

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT24-017 | Medusamon | PARTIAL | WhenDigivolving now fires (Issue 48 FIXED), but DP scaling order wrong (Issue 1), missing Piercing (Issue 7), missing trash-to-deck cost (Issue 2) |
| BT24-016 | Lamiamon | PASS | WhenDigivolving fires correctly. Opponent hand selection works (Issue 49 FIXED). Bottom security placement works (Issue 50 FIXED). Inherited OnLoseSecurity play effect works. |
| BT24-082 | Owen Dreadnought | FAIL | Digivolve trigger never fires (Issue 3). Start of Main Phase filter missing (Issue 5), cost missing (Issue 6). |
| BT24-008 | Elizamon | PASS | On Play optional trash-to-draw works. Inherited gain memory on security loss fires correctly. |
| BT21-017 | Dimetromon | PASS | Digivolve cost correct (2). WhenDigivolving triggered. Inherited gain memory works. |
| BT21-008 | Elizamon (BT21) | PASS | Play cost correct. All effects functional. |
| BT21-001 | Gigimon | PASS | Egg hatch works. Inherited digivolve-on-security-loss triggers correctly. |

---

## Fixes Verified This Session

1. **Issue 46 (King Drasil OPT)**: `record_activation()` added to `calculate_play_cost()` — confirmed working
2. **Issue 48 (Medusamon WhenDigivolving)**: Removed `set_timing(OnEnterFieldAnyone)` — WhenDigivolving now fires via `is_when_digivolving=True` flag
3. **Issue 49 (Lamiamon hand selection)**: `effect_select_hand_card()` provides opponent choice — confirmed working
4. **Issue 50 (Lamiamon security placement)**: `append()` used for bottom — confirmed working
5. **Issue 51 (Optional On Deletion)**: `effect_choose_branch()` wraps optional effects — confirmed by engine code
6. **Issue 52 (SelectHand descriptions)**: Selection phase check moved to top of `_describe_single_action()` — confirmed "Select X from hand" displays correctly

## Areas Not Covered

- BT21-029 Medusamon (BT21) token play on deletion/security loss (still PARTIAL)
- BT24-012 protection effect (did not trigger scenario)
- Petrification Token deletion mechanics (opponent can't attack with them per "[Your Turn] This Digimon can't suspend")
- Security battle interaction with Piercing (Piercing not implemented)
- Owen Dreadnought security effect (not triggered)
