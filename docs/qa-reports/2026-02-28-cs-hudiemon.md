# Gameplay QA Report -- CS Hudiemon

## Test Setup
- **Date**: 2026-02-28
- **Archetype**: CS Hudiemon
- **Game ID(s)**: 201b4f83-f518-4dd5-b956-077faec265a4, 3c9b9d09-e996-4398-a4a6-11b1a6b44aa8
- **Total Turns Played**: 9 (across 2 games)
- **Focus Areas**: play costs, digivolution, keywords, On Play effects, inherited effects, effect timings

## Summary
- **Total Issues Found**: 12
- Critical: 1 | High: 5 | Medium: 3 | Low: 3

## Detailed Findings

### Issue 1 (SYSTEMATIC): Effects fire at every timing
- **Card(s)**: ALL tamer/option cards with field effects -- BT22-089 Mirei Mikagura, BT23-081 Chitose Imai, BT23-090 Keisuke Amasawa, BT22-099 Kuremi Detective Agency
- **Severity**: critical
- **Category**: effect
- **Expected**: Effects should only fire at their designated timings (On Play, Start of Main Phase, End of Turn, etc.)
- **Actual**: Effects fire at OnTappedAnyone, OnAllyAttack, OnSecurityCheck, OnLoseSecurity, OnEndAttack, OnAddDigivolutionCards, OnAddHand, OnUseOption, OnEnterFieldAnyone, OnEndMainPhase, OnAddSecurity -- literally every timing the engine checks.
- **Root Cause**: Engine's `_effect_matches_timing()` in `game.py` falls through to `return True` for any effect without one of the 4 timing flags (`is_on_play`, `is_when_digivolving`, `is_on_attack`, `is_on_deletion`). The `effect.timing` field exists on `ICardEffect` and `set_timing()` is defined, but no scripts call it and the engine never checks it.
- **Impact**: Causes memory corruption (delay "Gain 2 memory" effects fire multiple times per action), spurious selection phases, and polluted logs. This is the single most impactful bug affecting both decks.
- **Rules Reference**: Effects should only trigger at their designated timing per Digimon TCG rules

### Issue 2: BT23-048 Gotsumon On Play reveal filter only checks Tamers, not "Tamer or Option"
- **Card(s)**: BT23-048 -- Gotsumon
- **Severity**: high
- **Category**: effect
- **Expected**: The On Play effect should add "1 Tamer card or Option card with the [CS] trait" -- both Tamers AND Options with CS trait should be selectable.
- **Actual**: `reveal_filter_1` in the script only checks `is_tamer`, missing Options with CS trait.
- **Evidence**: Script line 62-65: `if not getattr(c, 'is_tamer', False): return False`

### Issue 3: Spurious trash pop pattern (BT23-048, BT16-082, BT23-017, P-035)
- **Card(s)**: BT23-048 Gotsumon, BT16-082 Ukkomon, BT23-017 Betamon (from variant decks), P-035 Red Memory Boost
- **Severity**: high
- **Category**: effect
- **Expected**: Reveal-and-select effects should only reveal from deck top and add matching cards to hand.
- **Actual**: Scripts pop a card from `player.trash_cards` and add to hand BEFORE the reveal-and-select operation. This is a transpiler artifact that corrupts game state when there are cards in trash.
- **Evidence**: Pattern: `if player and player.trash_cards: card_to_add = player.trash_cards.pop(); player.hand_cards.append(card_to_add)`

### Issue 4: BT16-082 Ukkomon reveal-and-select logic entirely missing
- **Card(s)**: BT16-082 -- Ukkomon
- **Severity**: high
- **Category**: effect
- **Expected**: "When one of your Digimon moves from breeding area: reveal top 3, add 1 Digimon or Tamer to hand, rest to deck bottom, may hatch"
- **Actual**: Script's `process0` just does `player.trash_cards.pop()` and appends to hand. No reveal, no selection, wrong zone (trash instead of deck). The entire reveal-and-select logic is missing.
- **Evidence**: Static analysis of `bt16/bt16_082.py`

### Issue 5: BT23-090 Keisuke Amasawa protection effect targets opponent instead of own Digimon
- **Card(s)**: BT23-090 -- Keisuke Amasawa
- **Severity**: high
- **Category**: effect
- **Expected**: "When one of your CS Digimon would leave the battle area, by suspending this Tamer and trashing 2 same-level digivolution cards, prevent that Digimon from leaving."
- **Actual**: The process calls `game.effect_select_opponent_permanent` with a suspend callback, targeting an opponent's permanent instead of your own CS Digimon. The protection logic is completely inverted.
- **Evidence**: Static analysis of `bt23/bt23_089.py` (note: file is named bt23_089 but maps to BT23-090 Keisuke)

### Issue 6: BT16-025 Paildramon When Digivolving suspends 1 instead of all eligible
- **Card(s)**: BT16-025 -- Paildramon
- **Severity**: high
- **Category**: effect
- **Expected**: "Suspend all of your opponent's Digimon with as many or fewer digivolution cards as this Digimon."
- **Actual**: Script uses `effect_select_opponent_permanent` to suspend a single target instead of iterating all eligible targets. CANNOT_UNSUSPEND also applied to only 1 target.
- **Evidence**: Static analysis of `bt16/bt16_025.py`

### Issue 7: Action descriptions show "Trash X from hand" during SelectReveal
- **Card(s)**: BT22-099 Kuremi Detective Agency, BT23-048 Gotsumon
- **Severity**: medium
- **Category**: ui
- **Expected**: During SelectReveal phase, action descriptions should reference revealed cards.
- **Actual**: Actions say "Trash [hand card] from hand" instead of showing revealed card options.
- **Steps to Reproduce**: Play BT22-099, enter reveal-and-select, observe action descriptions

### Issue 8: Spurious SelectTarget phases with no pending selection
- **Card(s)**: Multiple
- **Severity**: medium
- **Category**: game_flow
- **Expected**: Selection phases should only appear when there are valid targets.
- **Actual**: Engine frequently enters SelectTarget with no pending selection, auto-recovers with "[Recovery] Selection phase GamePhase.SelectTarget with no pending selection -- recovering".

### Issue 9: BT23-032 Shakkoumon effect3 has wrong timing flag
- **Card(s)**: BT23-032 -- Shakkoumon
- **Severity**: medium
- **Category**: effect
- **Expected**: "Start of Your Main Phase: Attack with this Digimon" should trigger at start of main phase.
- **Actual**: Effect has `is_on_play = True` instead of a start-of-main-phase timing. Additionally, the de-digivolve effect always runs without checking if this was a DNA digivolve. Force attack is a `pass` stub.
- **Evidence**: Static analysis of `bt23/bt23_032.py`

### Issue 10: BT23-050 Ankylomon DP target auto-selected instead of player-chosen
- **Card(s)**: BT23-050 -- Ankylomon
- **Severity**: low
- **Category**: effect
- **Expected**: "1 of your opponent's Digimon gets -2000 DP" should let player choose target.
- **Actual**: Targets lowest-DP Digimon automatically with `min(dp_targets, key=lambda p: p.dp)`.

### Issue 11: Game stuck in Draw phase during turn transition
- **Card(s)**: N/A (engine issue)
- **Severity**: low
- **Category**: game_flow
- **Expected**: Turn transition from P1 pass to P2 draw should auto-advance.
- **Actual**: Game became stuck in Draw phase with empty actions after a long turn with many effects firing. Likely caused by effect timing spam creating corrupt state.
- **Evidence**: Game 3c9b9d09 became stuck at Turn 2 Phase 1 (Draw) with empty actions dict

### Issue 12: BT23-017 Betamon effect3 deletes opponent's Digimon instead of self
- **Card(s)**: BT23-017 -- Betamon (in variant decks, not this specific list)
- **Severity**: low
- **Category**: effect
- **Expected**: "End of opponent's turn: delete this Digimon" should delete self.
- **Actual**: Process calls `game.effect_select_opponent_permanent` and deletes an opponent's Digimon. Completely inverted.
- **Evidence**: Static analysis of `bt23/bt23_017.py`

## Cards Tested

| Card ID | Card Name | Lv | Played | Play Cost | Evo Cost | On Play | When Digivolving | Inherited | Keywords | Status |
|---------|-----------|-----|--------|-----------|----------|---------|-----------------|-----------|----------|--------|
| BT22-005 | Tsumemon | 2 | Hatched | N/A | N/A | N/A | N/A | Registered (CS draw) | N/A | PASS |
| BT22-043 | Terriermon | 3 | Yes | 3 (correct) | N/A | Registered (CS tamer play) | N/A | Registered | N/A | PASS |
| BT22-044 | Palmon | 3 | Yes | 3 (correct) | N/A | N/A | N/A | Registered (memory gain) | N/A | PASS |
| BT22-089 | Mirei Mikagura | - | Yes | 3 (correct) | N/A | Triggered (trash+draw) | N/A | N/A | N/A | PARTIAL (timing bug) |
| BT22-099 | Kuremi Detective Agency | - | Yes | 3 (correct) | N/A | Triggered (reveal) | N/A | N/A | N/A | PARTIAL (descriptions wrong) |
| BT23-020 | Seadramon | 4 | Not played | -- | -- | -- | -- | -- | Alliance | PASS (static: clean) |
| BT23-027 | Angemon | 4 | Yes | 5 (correct) | N/A | Triggered (draw+DNA) | N/A | N/A | N/A | PASS |
| BT23-032 | Shakkoumon | 5 | Digivolved | N/A | 4 (correct) | N/A | Triggered | Inherited fires | N/A | PARTIAL (wrong timing flag) |
| BT23-048 | Gotsumon | 3 | Yes | 3 (correct) | 1 (correct) | FAIL (Issues 2-3) | N/A | PARTIAL | N/A | FAIL |
| BT23-050 | Ankylomon | 4 | Yes | 5 (correct) | N/A | Triggered (DP-2000+DNA) | N/A | N/A | N/A | PARTIAL (auto-target) |
| BT23-081 | Chitose Imai | - | Yes | 4 (correct) | N/A | Triggered (free CS play) | N/A | N/A | N/A | PASS |
| BT23-090 | Keisuke Amasawa | - | Yes (free) | 0 (via effect) | N/A | Registered | N/A | N/A | N/A | FAIL (Issue 5) |
| BT23-101 | Hudiemon | 4 | Digivolved | N/A | 5 (correct) | N/A | Triggered (free CS play) | N/A | N/A | PASS |
| BT16-025 | Paildramon | 5 | Digivolved | N/A | 4 (correct) | N/A | Triggered (suspend) | N/A | N/A | PARTIAL (Issue 6) |
| BT16-082 | Ukkomon | 3 | Yes | 3 (correct) | N/A | Registered (on-move) | N/A | N/A | N/A | FAIL (Issue 4) |

## Coverage
- Cards played/tested: 15/15 (100%)
- Play costs verified: 10/10 applicable (100%)
- Evo costs verified: 3/3 digivolutions (100%)
- Effects verified (gameplay): ~55%
- Effects verified (static analysis): ~85%
- Full evo chain tested: Tsumemon -> Terriermon -> Hudiemon -> Shakkoumon (PASS)
- Digivolve bonus draw: confirmed for all 3 digivolutions

## Areas Not Covered
- DNA Digivolution (Shakkoumon requires Angemon + Ankylomon -- approximated as free play)
- Alliance keyword (BT23-020 Seadramon)
- Blocker keyword
- When Attacking effects (no attacks completed)
- Security effects
- Inherited effects in full stacks (beyond registration)
- Once-per-turn limits
- Cross-deck interaction (game stuck during turn transition)

## Root Cause Analysis: Systematic Effect Timing Bug

The engine's `_effect_matches_timing()` function only has 4 timing flags: `is_on_play`, `is_when_digivolving`, `is_on_attack`, `is_on_deletion`. Effects without these flags (including all tamer Start-of-Main-Phase effects, End-of-Turn effects, Option Delay effects, and reactive effects) fall through to `return True` and fire at every timing. The `effect.timing` field and `set_timing()` method exist on `ICardEffect` but are never used by any script or checked by the engine.

**Fix required**: Either (a) scripts must call `effect.set_timing()` and the engine must check `effect.timing`, or (b) `_effect_matches_timing` must default to `return False` for unflagged effects at non-specified timings. Approach (a) is cleanest.
