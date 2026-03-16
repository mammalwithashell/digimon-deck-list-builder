# QA Report: TS Jupitermon vs Royal Knights (Cross-Archetype)

**Date**: 2026-03-15
**Matchup**: TS Jupitermon vs Royal Knights
**Method**: Automated regression (20 games) + targeted debug game testing (6 games)
**Prior QA**: 2026-03-14 (TS Jupitermon vs Zephagamon, Jesmon vs Royal Knights)

## Regression Results

### Random Policy (10 games)

| # | P1 Deck | P2 Deck | Winner | Turns | Steps |
|---|---------|---------|--------|-------|-------|
| 1 | TS | RK | P1 (TS) | 26 | 183 |
| 2 | TS | RK | P2 (RK) | 29 | 164 |
| 3 | TS | RK | P1 (TS) | 33 | 153 |
| 4 | TS | RK | P1 (TS) | 39 | 311 |
| 5 | TS | RK | P2 (RK) | 19 | 74 |
| 6 | RK | TS | P2 (TS) | 26 | 121 |
| 7 | RK | TS | P1 (RK) | 30 | 143 |
| 8 | RK | TS | P1 (RK) | 21 | 115 |
| 9 | RK | TS | P1 (RK) | 23 | 97 |
| 10 | RK | TS | P1 (RK) | 24 | 98 |

**Random summary**: TS wins 4/10, RK wins 6/10. Avg 27 turns, 146 steps. **0 crashes.**

### Greedy Policy (10 games)

| # | P1 Deck | P2 Deck | Winner | Turns | Steps |
|---|---------|---------|--------|-------|-------|
| 1 | TS | RK | P2 (RK) | 52 | 143 |
| 2 | TS | RK | P2 (RK) | 42 | 124 |
| 3 | TS | RK | P2 (RK) | 61 | 153 |
| 4 | TS | RK | P2 (RK) | 49 | 131 |
| 5 | TS | RK | P2 (RK) | 46 | 134 |
| 6 | RK | TS | P1 (RK) | 45 | 113 |
| 7 | RK | TS | P1 (RK) | 36 | 97 |
| 8 | RK | TS | P1 (RK) | 43 | 119 |
| 9 | RK | TS | P1 (RK) | 67 | 165 |
| 10 | RK | TS | P1 (RK) | 49 | 133 |

**Greedy summary**: TS wins 0/10, RK wins 10/10. Avg 49 turns, 131 steps. **0 crashes.**

### Overall: 20/20 games completed, 0 crashes, 0 hangs.

---

## Targeted Debug Test Results

### Test 1: BT24-041 Minervamon On Play Effect

**Setup**: P1 hand = [BT24-041, BT24-085, BT24-034, P-104, BT24-036], memory=10

**Findings**:
- BT24-041 play cost = 12 (correct, no Iliad on field for -5 reduction)
- On Play effect triggers SelectTarget: offers Iliad trait cards from hand with cost <=5
- BT24-034 Aegiomon (Iliad, cost 5) correctly offered; BT24-036 Medicmon (no Iliad) excluded
- Played BT24-034 for free via On Play effect -- **PASS**
- De-Digivolve step correctly skipped when opponent has no Digimon -- **PASS**

### Test 2: BT24-085 + BT24-102 Tamer Auras

**Setup**: P1 hand = [BT24-085, BT24-102, P-194, BT24-034, BT24-036], memory=10

**Findings**:
- BT24-085 Dan Yuki & Kanan Yuki: play cost 4, no On Play trigger (correct) -- **PASS**
- BT24-102 Homeros: play cost 5 (correct) -- **PASS**
- P-194 Aegiomon: play cost 4 (correct), DP on field = 5000 (base 4000 + 1000 from BT24-102 [TS] aura) -- **BT24-102 DP aura PASS**
- Start of Main Phase (turn 3): Memory 1 -> 3 (+1 from BT24-085 since <=4, +1 from BT24-102 always). Neither tamer suspended (memory 3 < 5 for BT24-102 suspend threshold) -- **BT24-085 memory gain PASS**, **BT24-102 Start of Main PASS**

### Test 3: BT13-100 + BT13-101 RK Tamers

**Setup**: P1 (RK) hand = [BT13-100, BT13-101, BT13-028, BT13-032, EX11-054], memory=10

**Findings**:
- BT13-100 Yoshino Fujieda: plays correctly, no On Play trigger on own play (correct -- triggers on digivolve with Vegetation/Plant/Fairy) -- **PASS**
- BT13-101 Miki & Megumi: plays correctly, On Play "play PawnChessmon" correctly skipped when no PawnChessmon in hand -- **PASS**

### Test 4: BT24-041 Reboot/Blocker Aura (Opponent's Turn)

**Setup**: P1 has BT24-041 Minervamon + BT24-034 Aegiomon (Iliad) + P-194 + BT24-085 on field

**Findings**:
- BT24-041 aura effects registered: "Reboot (grant to Iliad Digimon)" and "Blocker (grant to Iliad Digimon)"
- Condition correctly checks `is_my_turn == False` (opponent's turn only)
- Aura keyword scanning confirmed active in `has_keyword()` path -- **PASS** (structural verification)

### Test 5: BT24-090 Option Card [Main] Effect

**Setup**: P1 hand = [BT24-090, P-194, BT24-034, BT24-036, BT24-085], memory=10

**Findings**:
- BT24-090 play cost = 3 (correct) -- **PASS**
- Security swap: bottom security (P-197) moved to hand, BT24-090 placed as bottom security face-up -- **PASS**
- Security count unchanged (5 -> 5) -- **PASS**
- Play TS Digimon with -3: SelectTarget offers P-194 Aegiomon (Yellow [TS], cost 4), BT24-034 Aegiomon (Yellow [TS], cost 5), P-197 Patamon (Yellow [TS], cost 3). BT24-036 Medicmon (no TS) correctly excluded. BT24-085 (Tamer, not Digimon) correctly excluded -- **PASS**

### Test 6: BT13-086 Gizmon: XT (RK Deck)

**Setup**: P1 (RK) hand = [BT13-086, EX11-054, BT13-047, BT13-043, BT13-036], memory=10

**Findings**:
- BT13-086 (cost 9) played for only 3 memory (reduction of 6) WITHOUT deleting a Lv4 Digimon -- **QA-FAIL** (see Script Issues below)

---

## Script Issues Found

### BT13-086 Gizmon: XT -- QA-FAIL (3 bugs)

**Severity**: HIGH

1. **Duplicate unconditional cost_reduction**: `effect1` (line 61) has `cost_reduction = 6` with no timing and condition `return True`. This applies ALWAYS, making the BeforePayCost "by deleting 1 Lv4 Digimon" cost optional/free. The reduction fires without the deletion cost being paid. This is both a script bug (unconditional effect should not exist) and the known systemic "BeforePayCost Process Callbacks Never Execute" gap.

2. **On Play filter too broad**: `process3` (line 117-130) uses `play_filter` that returns `True` for ANY card from trash. Should filter for cards with [Akihiro Kurata] in their name only.

3. **On Deletion filter too broad**: `process4` (line 148-161) uses `play_filter` that returns `True` for ANY card from trash. Should filter for cards with [ProtoGizmon] in their name only.

**Script**: `digimon_gym/engine/data/scripts/bt13/bt13_086.py`

### BT13-036 Liollmon -- QA-FAIL (2 bugs)

**Severity**: MEDIUM

1. **When Attacking -2000 DP auto-selects target**: `process1` (line 68-79) uses `min(dp_targets, key=lambda p: p.dp)` to auto-select lowest DP opponent Digimon. Card text says "1 of your opponent's Digimon" which requires player selection via `effect_select_opponent_permanent`.

2. **Missing security count condition**: The effect should only fire "if there're 6 or fewer total cards in both players' security stacks" but `condition1` does not check this.

**Script**: `digimon_gym/engine/data/scripts/bt13/bt13_036.py`

### BT24-098 Invasion of the Titans -- QA-FAIL (4 bugs)

**Severity**: HIGH

1. **Trash 2 only trashes 1**: `process0` (line 31-48) calls `effect_select_hand_card` once but card text says "trash 2 cards in your hand."

2. **On Play play filter missing [Titan] trait check**: `process2` (line 91-106) `play_filter` only checks `level <= 5`, not [Titan] trait.

3. **Security play filter missing [Titan] trait check**: `process3` (line 124-143) `play_filter` only checks `level <= 4`, not [Titan] trait.

4. **Security "add this card to hand" is wrong**: `process3` (line 138-139) pops from trash instead of adding the option card itself to hand.

**Script**: `digimon_gym/engine/data/scripts/bt24/bt24_098.py`

### BT13-100 Yoshino Fujieda -- QA-FAIL (2 bugs)

**Severity**: MEDIUM

1. **effect1 is_on_play incorrect**: effect1 uses `is_on_play = True` but the card text says "When one of your Digimon digivolves" -- this should use `is_when_digivolving = True` instead. The effect is meant to observe digivolution events, not play events.

2. **process1 suspends opponent instead of self**: The process calls `effect_select_opponent_permanent` for suspend. Card text says "by suspending this Tamer" (suspend-as-cost on self).

**Script**: `digimon_gym/engine/data/scripts/bt13/bt13_100.py`

---

## Cross-Archetype Observations

1. **Greedy policy strongly favors RK**: RK won 10/10 greedy games. The greedy policy (play first valid action) benefits RK's lower-cost creatures and Blocker keyword. TS Jupitermon's combo-heavy effects (tamer synergies, On Play chains) don't materialize well under greedy play.

2. **Random policy is more balanced**: TS wins 4/10, RK wins 6/10 under random play.

3. **No cross-archetype crashes**: The TS tamer aura system and RK Blocker/keyword system coexist without conflicts. BT24-041's Reboot/Blocker grants and BT13-047's native Blocker both function through the same `has_keyword()` aura scanning path.

4. **BT24-085 End of Turn effect stub**: The "1 [TS] Digimon may attack" portion of BT24-085's End of Turn effect is stubbed (`pass # descriptive-tagged: force_attack_ts_digimon`). This is a functional gap but does not cause crashes.

---

## Summary

| Category | Count |
|----------|-------|
| Regression games completed | 20/20 |
| Crashes | 0 |
| Debug tests | 6 |
| Focus cards PASS | BT24-041, BT24-085, BT24-090, BT24-102, BT13-101, BT13-047 |
| Focus cards QA-FAIL | BT13-086 (3 bugs), BT13-036 (2 bugs), BT24-098 (4 bugs), BT13-100 (2 bugs) |
| Known stubs | BT24-085 force_attack (End of Turn), BT24-090 ignore_color_req |

### Pre-existing Issues (from prior QA)
- BeforePayCost process callbacks never execute (systemic engine gap)
- BT24-085 End of Turn force_attack stubbed
- BT24-090 ignore_color_req stubbed
