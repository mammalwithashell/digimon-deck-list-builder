# QA Report: Rocks vs Chaos Control
Date: 2026-03-15

## Test Summary

| Category | Tests | Result |
|----------|-------|--------|
| Random policy (Rocks P1) | 5 games | 5/5 PASS (all complete, Chaos won all 5) |
| Random policy (Chaos P1) | 5 games | 5/5 PASS (all complete, Chaos won 3, Rocks won 2) |
| Greedy policy (Rocks P1) | 5 games | 5/5 PASS (all complete, Chaos won all 5) |
| Greedy policy (Chaos P1) | 5 games | 5/5 PASS (all complete, Chaos won all 5) |
| Debug: DemiDevimon On Play | 1 game | ISSUE found (see below) |
| Debug: Board saturation | 1 game | PASS (13 field cards, no crash) |
| Debug: DigiXros check | 1 game | N/A (no true DigiXros in these decks) |

**Overall: 20/20 automated games completed without crashes, deadlocks, or empty masks.**

---

## Automated Regression Results

### Random Policy (10 games)

| Game | P1 Deck | Winner | Turns | Steps |
|------|---------|--------|-------|-------|
| rnd_R_v_C_1 | Rocks | Chaos (P2) | 23 | 74 |
| rnd_R_v_C_2 | Rocks | Chaos (P2) | 14 | 45 |
| rnd_R_v_C_3 | Rocks | Chaos (P2) | 16 | 52 |
| rnd_R_v_C_4 | Rocks | Chaos (P2) | 9 | 29 |
| rnd_R_v_C_5 | Rocks | Chaos (P2) | 10 | 40 |
| rnd_C_v_R_1 | Chaos | Chaos (P1) | 12 | 48 |
| rnd_C_v_R_2 | Chaos | Rocks (P2) | 19 | 78 |
| rnd_C_v_R_3 | Chaos | Chaos (P1) | 19 | 64 |
| rnd_C_v_R_4 | Chaos | Chaos (P1) | 13 | 46 |
| rnd_C_v_R_5 | Chaos | Chaos (P1) | 6 | 25 |

**Random: Chaos Control won 8/10 games (80%).**

### Greedy Policy (10 games)

| Game | P1 Deck | Winner | Turns | Steps |
|------|---------|--------|-------|-------|
| grdy_R_v_C_1 | Rocks | Chaos (P2) | 57 | 149 |
| grdy_R_v_C_2 | Rocks | Chaos (P2) | 71 | 179 |
| grdy_R_v_C_3 | Rocks | Chaos (P2) | 67 | 183 |
| grdy_R_v_C_4 | Rocks | Chaos (P2) | 51 | 145 |
| grdy_R_v_C_5 | Rocks | Chaos (P2) | 69 | 186 |
| grdy_C_v_R_1 | Chaos | Chaos (P1) | 43 | 117 |
| grdy_C_v_R_2 | Chaos | Chaos (P1) | 42 | 115 |
| grdy_C_v_R_3 | Chaos | Chaos (P1) | 48 | 128 |
| grdy_C_v_R_4 | Chaos | Chaos (P1) | 70 | 180 |
| grdy_C_v_R_5 | Chaos | Chaos (P1) | 71 | 185 |

**Greedy: Chaos Control won 10/10 games (100%).**

Notable: No greedy deadlock observed in this matchup. The Start-phase deadlock reported in Rocks-vs-Dark-Masters (2026-03-14) did NOT reproduce here, likely because Chaos Control's deletion effects prevent the extreme field saturation (29+ permanents) that triggered it.

---

## Debug Test Results

### DemiDevimon (EX8-057) On Play
- **Status: QA-FAIL**
- Card text: "Reveal the top 3 cards of your deck. Add 1 card with the [NSo] trait and 1 card with the [Fallen Angel] trait among them to the hand. Return the rest to the bottom of the deck."
- Script behavior: On Play triggers a SelectEffectChoice (phase 12) to trash from hand instead of a reveal-and-select from deck top. The process starts by popping from `trash_cards` (line 58-59 of `ex8_057.py`), then calls `effect_reveal_and_select` with no trait-specific filters.
- Issues:
  1. Lines 58-59: `player.trash_cards.pop()` and `player.hand_cards.append()` -- takes a card from trash to hand before the reveal (incorrect, should only reveal from deck)
  2. `reveal_filter` returns `True` for all cards (should filter for NSo and Fallen Angel traits)
  3. Only adds 1 card via reveal, but card text says add 1 NSo + 1 Fallen Angel (2 cards total)

### Board Saturation
- **Status: PASS**
- Rocks filled 13 field slots (out of 14 max, slot 14 is breeding) without crash or deadlock
- At 13 field permanents, no more play actions were available (field full)
- No empty action masks observed
- The game loop stalled because the test kept resetting memory but couldn't play any more cards

### DigiXros
- **Status: N/A**
- Neither deck contains cards with true DigiXros requirements (material-based play from field)
- The `xros_req` fields on cards like BT22-054, EX11-040, EX8-057, EX8-058 are actually alternate digivolution requirements (already handled by `_alt_digi_*` attributes)

---

## Script Issues Found (Code Review)

### HIGH

#### BT24-071 Raidramon -- On Play/When Digivolving are no-ops + On Deletion filter missing
- Card text: "[On Play] [When Digivolving] 1 of your Digimon with the [System], [Life] or [Transmutation (App Name)] trait gains Security A. +1 for the turn."
- Effects 1 and 2 have `pass` in the process callback -- SA+1 grant is never applied
- On Deletion: plays ANY card from trash free with no filter. Card text says "play 1 level 3 Digimon card with the [Appmon] trait from your trash" -- needs level 3 + Appmon trait filter
- Also has duplicate On Deletion effects (effect 3 and 4 are identical)
- File: `digimon_gym/engine/data/scripts/bt24/bt24_071.py`

#### BT24-079 Hadesmon -- When Digivolving effect has no timing + re-activate has no process
- Card text: "[When Digivolving] You may play 1 level 4 or lower [System] or [Life] trait Digimon card from your trash without paying the cost. Then, you may link 1 [Appmon] trait Digimon card..."
- Effect 2 (play lv4- from trash) has no timing set -- will never fire as a triggered effect
- Effect 2 also has no [System]/[Life] trait filter on the play target
- Effect 3 (When Digivolving with timing) is identical to effect 2 but at least has timing
- Effect 4 ("All Turns: when other Digimon deleted, activate When Digivolving") has no process callback -- fires but does nothing
- The "link" portion of When Digivolving is not implemented in any effect
- File: `digimon_gym/engine/data/scripts/bt24/bt24_079.py`

#### EX10-054 VenomMyotismon -- Suspend only 1 target (card says 2) + cannot-unsuspend on wrong target
- Card text: "[On Play] [When Digivolving] You may suspend 2 of your opponent's Digimon or Tamers. Then, 2 of their Digimon or Tamers can't unsuspend until their turn ends."
- On Play/When Digivolving: `effect_select_opponent_permanent` is called once (card says "suspend 2" -- needs to be called twice or use multi-select)
- `grant_keyword('_is_cannot_unsuspend')` is applied to self (`perm`) instead of the two suspended opponent targets
- The "can't unsuspend" should apply to the suspended targets, not self
- On Deletion: card says "Delete 1 of your opponent's suspended Digimon" but `target_filter` only checks `p.is_digimon` (no suspended check)
- File: `digimon_gym/engine/data/scripts/ex10/ex10_054.py`

#### BT20-073 MetalPhantomon -- On Play/When Digivolving skips self-deletion cost
- Card says "By deleting 1 of your Digimon" (cost), then delete opponent's lv5 or lower
- Script calls `effect_select_opponent_permanent` directly without first deleting own Digimon
- Also: opponent target filter doesn't check `p.level <= 5`
- File: `digimon_gym/engine/data/scripts/bt20/bt20_073.py`

#### EX8-057 DemiDevimon -- On Play reveal is incorrect
- See debug test results above. Process pops from trash, filter is unfiltered, only selects 1 card (should be 2)
- File: `digimon_gym/engine/data/scripts/ex8/ex8_057.py`

### MEDIUM

#### BT24-097 Soul Fear -- When Attacking effect is NOT in the card text
- Card text only has [Main] and [Security] effects. No [When Attacking] effect.
- Effect 3 in the script adds a [When Attacking] "delete level 5 or lower" effect that does not exist on this card
- The script's effect 3 appears to be a fabricated effect. Soul Fear is an Option card, not a Digimon, so it should not have When Attacking.
- The linked inherited behavior (When Attacking as a digi card under a Digimon) is actually the Link effect -- but the script description says "level 5 or higher" while the filter checks level <= 5
- File: `digimon_gym/engine/data/scripts/bt24/bt24_097.py`

#### BT24-074 SkullSeadramon -- On Play effect is wrong + On Deletion plays from hand
- Card text (On Play/When Digivolving): "Trash any 3 digivolution cards from 1 of your opponent's Digimon. Then, if played by effects, delete 1 of your opponent's Digimon with no digivolution cards."
- Script implements delete + trash own digi cards, but card says trash OPPONENT's digi cards + conditional delete
- The "if played by effects" condition is not checked
- On Deletion plays from `'hand'` zone but card says "play from your trash"
- When Attacking inherited: should place another Digimon as bottom digi card THEN unsuspend; script just unsuspends any own Digimon
- File: `digimon_gym/engine/data/scripts/bt24/bt24_074.py`

### LOW

#### BT24-084 Inori Misono -- Memory condition checks game.memory not player memory
- Condition checks `game.memory > 4` but should check the player's perspective of memory
- This may or may not be correct depending on the engine's memory model
- File: `digimon_gym/engine/data/scripts/bt24/bt24_084.py`

---

## Engine-Level Observations

### No Greedy Deadlock
The Start-phase empty-mask deadlock reported in Rocks-vs-Dark-Masters (2026-03-14) did **not** reproduce in 10 greedy games between Rocks and Chaos Control. The highest turn count was 71 (vs the previous deadlock at ~turn 40-50 with 29 permanents). Chaos Control's deletion effects likely prevent the extreme field saturation that triggers the deadlock.

### Selection Phase Behavior
- Sunarizamon (EX8-047) correctly triggers SelectEffectChoice for hand trash selection on play
- DemiDevimon (EX8-057) incorrectly triggers SelectEffectChoice instead of reveal-from-deck
- Board saturation at 13 field permanents correctly prevents further plays without crashing

---

## Matchup Win Rate Summary

| Policy | Chaos Control Wins | Rocks Wins |
|--------|-------------------|------------|
| Random | 8 | 2 |
| Greedy | 10 | 0 |
| **Total** | **18** | **2** |

Chaos Control dominates this matchup with 90% overall win rate. Under greedy policy, Chaos Control wins 100% regardless of which player goes first. This is expected given Chaos Control's superior removal suite and Rocks' lack of protection/disruption.
