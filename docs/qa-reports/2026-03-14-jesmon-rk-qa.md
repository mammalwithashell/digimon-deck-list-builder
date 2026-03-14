# Jesmon vs Royal Knights QA Report

**Date**: 2026-03-14
**Matchup**: Jesmon vs Royal Knights
**Method**: API-driven debug game testing with deterministic scenarios
**Focus Cards**: BT20-013 (BaoHuckmon), BT23-057 (Gankoomon)

## Summary

| Card ID | Name | Expected Status | Actual Verdict | Notes |
|---------|------|-----------------|----------------|-------|
| BT20-013 | BaoHuckmon | PASS | **FAIL** | Script fix is correct (`manual_reduction=2`), but [Main] effect cannot be activated due to systemic engine gap |
| BT23-057 | Gankoomon | PASS | **PARTIAL** | Cost reduction works (-5 applied correctly), but trash-to-deck return process not executed |
| BT23-076 | Sistermon Blanc | n/a | **FAIL** | On Play script is entirely wrong: wrong zone sources, wrong targets |
| BT23-006 | Huckmon | n/a | PASS | On Play reveal + add to hand works correctly |
| EX11-053 | Omekamon | n/a | PASS | Plays correctly, effects registered |
| BT20-083 | Omekamon | n/a | PASS | Plays correctly, Blocker and effects registered |
| BT20-091 | Cool Boy | n/a | PASS | Plays correctly, RK trigger conditions verified |

## Critical Findings

### BUG-1: BT20-013 [Main] Effect Unreachable (Systemic Engine Gap)

**Severity**: HIGH (affects 97+ scripts)
**Card**: BT20-013 BaoHuckmon
**Card Text**: "[Main] [Once Per Turn] You may play 1 Digimon card with [Sistermon] or [Gankoomon] in its name from your hand with the play cost reduced by 2."

**Script Fix (Correct)**: The script at `digimon_gym/engine/data/scripts/bt20/bt20_013.py` now correctly uses `free=False, manual_reduction=2` instead of the old `free=True`. The play filter correctly matches Sistermon/Gankoomon names.

**Engine Gap**: The script uses `EffectTiming.OnDeclaration` for its [Main] effect, but the action mask (`digimon_gym/engine/game/action_mask.py` lines 158-171) only checks for the `_is_field_main` attribute flag to enable field [Main] effect activations (action IDs 1000+, effectIdx=2). The `OnDeclaration` timing is:
- Explicitly skipped in `_collect_triggered_effects` for non-field zones (line 596 of `game/__init__.py`)
- Never called via `execute_effects(EffectTiming.OnDeclaration)` anywhere in the engine
- Not checked in the action mask code

**Impact**: 97 scripts use `EffectTiming.OnDeclaration` without `_is_field_main`. Only 4 scripts correctly set `_is_field_main`. All 97 affected scripts have [Main] effects that can never be activated.

**Reproduction**:
1. Create debug game with BT20-013 in starting hand, memory=10
2. Pass breeding, play BT20-013 (cost 5)
3. Set memory to 10, inject Sistermon into hand
4. Check action mask: no effect activation (1000+) actions available
5. BT20-013's [Main] effect is visible in `internal-state` but unreachable

**Fix needed**: Either add `_is_field_main = True` to all scripts using `OnDeclaration` for field [Main] effects, or update the action mask to recognize `OnDeclaration` timing as a field [Main] trigger.

### BUG-2: BT23-057 BeforePayCost Process Callback Not Executed

**Severity**: MEDIUM
**Card**: BT23-057 Gankoomon
**Card Text**: "When this card would be played, by returning 3 cards with [Huckmon], [Sistermon] or [Jesmon] in their names from your trash to the top or bottom of the deck, reduce the play cost by 5."

**What works**: The `cost_reduction = 5` property is correctly read by `calculate_play_cost()` (line 460 of `game/__init__.py`), and the play cost is correctly reduced from 11 to 6. Verified: memory 10, played BT23-057, memory became 4 (cost 6 = 11 - 5).

**What fails**: The process callback `process1` (which removes 3 qualifying cards from trash and places them on top of deck) is never executed. `calculate_play_cost()` only calls `record_activation()` on committed effects (line 518-519), not `on_process_callback`. After playing BT23-057, the trash still contains all 3 injected cards.

**Impact**: The cost reduction is free -- no cards are returned from trash. This is an advantage for the player (they get the discount without paying the return cost).

**Reproduction**:
1. Create debug game with BT23-057 in hand, memory=10
2. Inject 3 Huckmon/Sistermon/Jesmon cards into trash
3. Pass breeding, play BT23-057
4. Check: memory = 4 (cost reduced correctly), trash unchanged (cards not returned)

### BUG-3: BT23-076 Sistermon Blanc On Play Completely Wrong

**Severity**: HIGH
**Card**: BT23-076 Sistermon Blanc
**Card Text**: "[On Play] Add your top security card to the hand. Then, Recovery +1 (Deck)."

**Script** (`digimon_gym/engine/data/scripts/bt23/bt23_076.py`, lines 34-52):
1. **Line 40**: Calls `player.recovery(1)` FIRST, but card text says add security to hand first, then Recovery.
2. **Lines 42-44**: Adds a card from TRASH (`player.trash_cards.pop()`) instead of from SECURITY (`player.security_cards.pop(0)`).
3. **Lines 46-51**: Trashes OPPONENT's top security card -- the card text says nothing about affecting the opponent's security.

**Expected behavior**: Remove player's top security card, add it to player's hand. Then place top card of player's deck on top of player's security (Recovery +1).

**Observed behavior**: Adds top of deck to player's security (Recovery), pops a card from player's trash to hand, removes opponent's top security to opponent's trash.

**Reproduction**:
1. Create debug game with BT23-076 in hand
2. Play BT23-076
3. Check: security count increased by 1 (instead of staying same), opponent lost 1 security

## Additional Cards Tested

### BT23-006 Huckmon (Lv.3) -- PASS
- Play cost 3, On Play reveal top 3 fires correctly
- Selection phase (Phase 12 / SelectReveal) presents valid Huckmon/Sistermon/Royal Knight options
- Selected card correctly added to hand
- Remaining revealed cards returned to deck

### EX11-053 Omekamon (Lv.4) -- PASS
- Play cost 5, plays correctly
- Effects registered: X Antibody trait, RK placement under King Drasil, Omnimon X play

### BT20-083 Omekamon (Lv.4) -- PASS
- Play cost 5, plays correctly
- Blocker keyword registered
- Omnimon X Antibody digivolve effect registered

### BT20-091 Cool Boy (Tamer) -- PASS
- Play cost 4, plays correctly
- RK play/digivolve triggers registered (suspend for draw 1 + memory 1)
- WhenRemoveField Omekamon play registered
- Security play registered
- Triggers correctly do NOT fire on tamer's own play (only on RK Digimon play)

## Systemic Issues Identified

### 1. OnDeclaration Timing Dead Code (97 scripts affected)
The `EffectTiming.OnDeclaration` timing is used by 102 scripts for field [Main] effects, but:
- The action mask only checks `_is_field_main` flag (4 scripts have this)
- `execute_effects(OnDeclaration)` is never called
- Non-field zone collection explicitly skips `OnDeclaration`

This makes ALL `OnDeclaration`-based [Main] effects unreachable. Scripts affected include BT20-013 (BaoHuckmon), BT13-008 (Agumon), BT15-009 (Meramon), and 94 others.

### 2. BeforePayCost Process Callbacks Never Execute
`calculate_play_cost()` reads `cost_reduction` values from BeforePayCost effects but never calls their `on_process_callback`. Cards like BT23-057 that have a cost (returning cards from trash) as part of their cost reduction get the discount for free.

## Test Methodology

- Created debug games via `POST /debug/games` with `skip_shuffle=true` and specified starting hands
- Injected test cards via `POST /debug/games/{id}/inject-card` (trash zone)
- Set memory via `POST /debug/games/{id}/set-memory`
- Verified state via `GET /debug/games/{id}/internal-state`
- Checked action masks via `GET /games/{id}/action-mask`
- Executed actions via `POST /games/{id}/actions`
- Validated memory arithmetic against card costs to confirm reduction values

## Deck Lists Used

**Jesmon** (from deck_library.json, digilab_8f63d962dae6, placement 1st):
BT14-001 x4, BT20-008 x4, BT23-006 x4, BT23-076 x4, BT6-009 x2, BT20-013 x4, BT20-084 x2, BT23-077 x1, BT20-014 x4, BT20-057 x4, BT20-059 x4, BT23-013 x4, BT10-016 x1, BT20-021 x2, BT10-112 x1, EX1-066 x4, ST12-15 x4, BT8-097 x1

**Royal Knights** (from deck_library.json, digimonmeta_2fb1af3ba632, placement 1st):
BT13-007 x4, BT13-093 x1, EX11-053 x4, BT20-083 x4, BT23-054 x3, BT13-075 x1, BT19-072 x2, BT20-017 x3, BT22-052 x3, BT23-035 x2, BT23-058 x2, BT13-112 x3, BT20-060 x4, BT20-102 x4, EX11-071 x4, BT20-091 x4, BT13-110 x1, EX4-065 x1, BT20-100 x4
