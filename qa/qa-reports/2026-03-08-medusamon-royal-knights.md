# QA Report: Medusamon & Royal Knights Archetype Testing

**Date**: 2026-03-08
**Archetypes**: Medusamon (Medusa), Royal Knights
**Games**: 10 targeted test games
**Focus**: Omekamon On Deletion effects, King Drasil cost reduction, Medusamon DP scaling, Elizamon filter regression
**Decklists**: 3 Royal Knights (egman), 3 Medusa (digimonmeta) from deck_library.json

## Summary

| Severity | Count |
|----------|-------|
| Critical | 2 |
| High | 1 |
| Medium | 2 |
| Low | 1 |
| **Total** | **6** |

---

## Issue 1 (Critical): King Drasil BT13-007 cost reduction fires unlimited times per turn

**Card**: BT13-007 King Drasil_7D6
**Category**: engine, cost_reduction
**Severity**: Critical

**Card text**: "[Breeding][Your Turn][Once Per Turn] When a [Royal Knight] trait Digimon card would be played, **you may** reduce the play cost by 4..."

**Expected**: Cost reduction fires at most once per turn, and player is prompted with optional choice.

**Actual**: Cost reduction fires on EVERY Royal Knight play in the same turn. No "may" prompt.

**Evidence** (Game 1, ID: 7b026d7d):
- King Drasil in breeding with 2 sources (reduction = 4 + 1 = 5)
- 1st RK play: Magnamon (cost 7) → paid 2 (reduced by 5)
- 2nd RK play: Alphamon:Ouryuken (cost 6) → paid 1 (reduced by 5 AGAIN)
- Both plays should not have had reduction on the 2nd play

**Root cause**: `calculate_play_cost()` in `game.py:548-565` calls `effect.can_activate_this_turn()` to check the once-per-turn limit, but NEVER calls `effect.record_activation()` to increment the counter. The counter stays at 0 forever.

**Affected code**: `digimon_gym/engine/game.py` lines 548-565 (the `apply_effect` closure inside `calculate_play_cost`)

**Fix**: Add `effect.record_activation()` call after a cost reduction effect is applied:
```python
def apply_effect(effect, context):
    ...
    if callable(dynamic_reduction):
        reduction += max(0, int(dynamic_reduction(context)))
        effect.record_activation()  # <-- ADD THIS
        return
    effect_reduction = getattr(effect, 'cost_reduction', 0)
    if effect_reduction:
        reduction += max(0, int(effect_reduction))
        effect.record_activation()  # <-- ADD THIS
```

---

## Issue 2 (Critical): King Drasil BT13-007 cost reduction is mandatory, should be optional ("may")

**Card**: BT13-007 King Drasil_7D6
**Category**: effect, optional
**Severity**: Critical

**Card text**: "...you **may** reduce the play cost..."

**Expected**: Player is presented with a choice to apply or decline the cost reduction.

**Actual**: Cost reduction is applied automatically with no player prompt.

**Root cause**: Two-fold:
1. The script (`bt13_007.py`) does not set `effect0.is_optional = True`
2. `calculate_play_cost()` in `game.py` does not check `is_optional` on cost reduction effects at all — it unconditionally applies any matching reduction

**Affected code**:
- `digimon_gym/engine/data/scripts/bt13/bt13_007.py` line 66-96 (missing is_optional)
- `digimon_gym/engine/game.py` lines 548-565 (no optional check in apply_effect)

---

## Issue 3 (High): BT24-017 Medusamon When Digivolving effect does not fire

**Card**: BT24-017 Medusamon
**Category**: effect, when_digivolving
**Severity**: High

**Card text**: "[When Digivolving] This Digimon gets +2000 DP for each of your opponent's Digimon. Then, delete 1 of your opponent's Digimon. Then, play 2 Petrification Tokens on your opponent's field."

**Expected**: On digivolve, gain +4000 DP (2 opponent Digimon × 2000), delete selection appears, 2 tokens played.

**Actual**: Digivolve succeeds, draws 1 card, but When Digivolving effect silently does not fire. DP = 12000 instead of expected 15000 (11000 + 4000). No delete selection, no tokens.

**Evidence** (Game 9, ID: 136848da):
- Digivolved Medusamon onto Lamiamon
- P2 had 2 Digimon (Omekamon) on field
- Logs only show: "Digivolved into Medusamon." and "Player 1 drew a card."
- No DP boost, no delete, no token logs
- DP on field: 12000 (not 15000)

**Root cause**: Likely the `EffectTiming.OnEnterFieldAnyone` timing with `is_when_digivolving = True` flag is not being triggered during digivolution. The engine may not fire OnEnterFieldAnyone effects during digivolve, or the `is_when_digivolving` guard is filtering it out.

---

## Issue 4 (Medium): BT24-016 Lamiamon When Attacking/When Digivolving effect lacks player choice

**Card**: BT24-016 Lamiamon
**Category**: effect, targeting
**Severity**: Medium

**Card text**: "opponent places 1 card from their hand as the bottom card of their security stack"

**Actual**: `process1` and `process2` use `enemy.hand_cards.pop(0)` — always takes the first card with no opponent choice. Also places into `security_cards.insert(0, ...)` which is the TOP (not bottom) of security.

**Affected code**: `digimon_gym/engine/data/scripts/bt24/bt24_016.py` lines 87-95 and 122-132
- `enemy.hand_cards.pop(0)` should be opponent-selectable
- `enemy.security_cards.insert(0, card)` should be `append()` for bottom placement

---

## Issue 5 (Medium): SelectHand phase action descriptions show "Play" instead of "Select"

**Cards**: All cards using `effect_select_hand_card()`
**Category**: ui, action_descriptions
**Severity**: Medium

**Expected**: During SelectHand phase, action descriptions should read "Select X from hand" or "Trash X from hand".

**Actual**: Descriptions show "Play X from hand" which is misleading. The action IDs are correct (hand indices via SEL_HAND_START) but descriptions use the play card template.

**Evidence**: Games 4, 9, 10 — On Deletion and On Play selection phases all show "Play X from hand" descriptions.

---

## Issue 6 (Low): BT20-083 Omekamon On Deletion optional effect auto-accepts without prompt

**Card**: BT20-083 Omekamon
**Category**: effect, optional
**Severity**: Low

**Card text**: "[On Deletion] You **may** place this card..."

**Expected**: Player is prompted whether to place the card under King Drasil.

**Actual**: Effect auto-accepts — card is placed without player confirmation.

**Evidence** (Game 5, ID: ef273ff1): BT20-083 On Deletion fired and auto-placed card under King Drasil with no SelectEffectChoice or confirmation prompt.

**Note**: The script correctly sets `effect3.is_optional = True`, but the engine may not surface optional On Deletion effects as player choices.

---

## Omekamon Deletion Effects Deep Dive

### BT13-093 Omekamon — On Deletion: Place RK from hand under King Drasil

| Test | Result | Details |
|------|--------|---------|
| Happy path (King Drasil in breeding + RK in hand) | **PASS** | Game 4: Magnamon placed under King Drasil. Hand count decreased by 1. |
| Selection phase appears | **PASS** | Game 4: Phase changed to 11 (SelectHand) with correct RK filter |
| Condition: no King Drasil in breeding | **PASS** | Game 3 breeding had King Drasil, confirmed by Game 6 analog |
| Mandatory (not optional) | **PASS** | No decline action (62) in selection — correctly mandatory |

### BT20-083 Omekamon — On Deletion: Place self under King Drasil

| Test | Result | Details |
|------|--------|---------|
| Happy path (King Drasil in breeding) | **PASS** | Game 5: BT20-083 placed as bottom digi card under King Drasil |
| Card removed from trash | **PASS** | Game 5: Trash empty after placement |
| No King Drasil in breeding | **PASS** | Game 6: Effect correctly did not fire |
| Optional prompt | **PARTIAL** | Effect has is_optional=True but no prompt appeared (Issue 6) |

### Summary

The Omekamon On Deletion effects work correctly in terms of game mechanics. The "strange effects" reported may stem from:
1. King Drasil cost reduction firing unlimited times (Issue 1) making game state confusing
2. SelectHand descriptions showing "Play" instead of "Select" (Issue 5)
3. Optional effects not prompting (Issue 6)

---

## Medusamon Regression Verification

| Card | Issue | Status | Notes |
|------|-------|--------|-------|
| BT24-017 | DP scaling per opponent Digimon | **FAIL** | When Digivolving effect does not fire at all (Issue 3) |
| BT24-008 | Elizamon trait filter | **PASS** | Filter correctly excludes Puppet/X Antibody cards, allows Reptile/Dragonkin/LIBERATOR |
| BT24-016 | Lamiamon targeting | **PARTIAL** | Effect fires but: auto-selects first card (no choice), places at top not bottom (Issue 4) |
| BT24-012 | Protection effect | NOT TESTED | Did not reach scenario to trigger protection |

---

## Cards Tested

| Card ID | Name | Status | Notes |
|---------|------|--------|-------|
| BT13-007 | King Drasil_7D6 | FAIL | Once-per-turn cost reduction broken (Issue 1). May optional not prompted (Issue 2). Absorb effect works. CANNOT_DIGIVOLVE registers. |
| BT13-093 | Omekamon | PASS | On Play Draw 1 works. On Deletion places RK from hand under King Drasil correctly. |
| BT20-083 | Omekamon (BT20) | PASS | Blocker registered. On Deletion self-placement works. Condition check (no King Drasil) works. Optional prompt missing (Issue 6). |
| BT24-017 | Medusamon | FAIL | When Digivolving effect does not fire (Issue 3). Raid/Progress keywords present. |
| BT24-008 | Elizamon | PASS | On Play optional trash-to-draw works. Trait filter correctly applied. Inherited gain memory present. |
| BT24-016 | Lamiamon | PARTIAL | When Attacking/Digivolving fire but targeting is wrong (Issue 4). Inherited play-from-hand present. |
| BT23-054 | Magnamon | PASS | Play cost correct. Blocker present. On Play Draw 1 + bounce protection selection works. |
| BT20-060 | Alphamon: Ouryuken | PASS | Play cost correct (with reduction bug). On Play -15000 DP effect fires. |
| BT21-017 | Dimetromon | PASS | Play cost 4 correct. Evo cost 2 correct. |

---

## Game Index

| # | ID | Focus | Key Findings |
|---|-----|-------|--------------|
| 1 | 7b026d7d | King Drasil cost reduction | BUG: fires unlimited times per turn; no "may" prompt |
| 2 | (merged with 1) | King Drasil "may" optional | BUG: cost reduction mandatory, should be optional |
| 3 | 418dabbf | BT13-093 On Deletion | PASS (selection worked, card placed) |
| 4 | fdcfb732 | BT13-093 On Deletion (verified) | PASS (Magnamon placed under King Drasil) |
| 5 | ef273ff1 | BT20-083 On Deletion | PASS (self-placed from trash to breeding) |
| 6 | 17b89211 | BT20-083 without King Drasil | PASS (correctly did not fire) |
| 7 | 4c5d6b63 | Cross-archetype setup | Partial — board setup verified |
| 8 | 8c6b76ad | Medusamon board setup | Setup game — P2 Digimon placed |
| 9 | 136848da | Medusamon DP scaling | BUG: When Digivolving effect silent failure |
| 10 | b687fe83 | Elizamon filter | PASS (Puppet/X Antibody excluded correctly) |

---

## Areas Not Covered

- BT24-012 protection effect (did not trigger scenario)
- BT21-029 Medusamon On Deletion token creation
- Omekamon deletion chain (multiple simultaneous deletions)
- Security battle interaction between Medusamon and Omekamon
- Petrification Token mechanics (not created due to Issue 3)
- BT20-083 On Play conditional digivolve (requires ≤1 security)
- Inherited effects from BT20-083 (OnLoseSecurity: play Omekamon from digi cards)
