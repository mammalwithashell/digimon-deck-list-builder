# QA Report: Jesmon vs Dark Masters

**Date**: 2026-03-15
**QA Agent**: Agent 5 (cross-archetype matchup)
**Matchup**: Jesmon vs Dark Masters
**Method**: Direct GameState API (greedy/random policy), HTTP debug game testing

## Summary

| Metric | Value |
|--------|-------|
| Random games attempted | 10 |
| Random games completed | 1 (10%) |
| Random game crashes | 9 (BT15-069: 4, BT14-044: 5) |
| Greedy games attempted | 10 |
| Greedy games completed | 7 (70%) |
| Greedy game crashes | 3 (BT15-069: 3) |
| Jesmon wins (greedy, completed) | 4 |
| Dark Masters wins (greedy, completed) | 3 |
| Avg steps (greedy, completed) | 56.4 |
| Debug games run | 5 |

## Crash Bugs Found

### CRASH-1: BT15-069 Candlemon `is_player_one` AttributeError (CRITICAL)

**Severity**: CRITICAL (crashes game)
**File**: `digimon_gym/engine/data/scripts/bt15/bt15_069.py`, line 42
**Card**: BT15-069 Candlemon (Dark Masters deck, 4x)
**Frequency**: 3/10 greedy, 4/10 random

**Root cause**: Script uses `player.is_player_one` which does not exist on the `Player` class. The correct attribute is `player.player_id == 1`.

**Traceback**:
```
File "scripts/bt15/bt15_069.py", line 42, in process0
    opponent_memory = game.memory if not player.is_player_one else -game.memory
AttributeError: 'Player' object has no attribute 'is_player_one'
```

**Fix**: Replace `player.is_player_one` with `player.player_id == 1` (two occurrences, lines 42-46). Only BT15-069 uses this incorrect attribute across all scripts.

---

### CRASH-2: BT14-044 Palmon `card_name` AttributeError (CRITICAL)

**Severity**: CRITICAL (crashes game when granted effect fires)
**File**: `digimon_gym/engine/data/scripts/bt14/bt14_044.py`, line 80
**Card**: BT14-044 Palmon (Dark Masters deck, 4x)
**Frequency**: 5/10 random (fires when opponent's Digimon suspends after being granted the effect)

**Root cause**: The granted `OnTappedAnyone` effect's process callback uses `target_perm.top_card.card_name` (singular), but `CardSource` has `card_names` (plural list). The crash only happens in the log formatting line, so the functional effect (lose 2 memory) may still be correct if the log line is bypassed.

**Traceback**:
```
File "scripts/bt14/bt14_044.py", line 80, in granted_process
    f"[BT14-044] {target_perm.top_card.card_name} "
AttributeError: 'CardSource' object has no attribute 'card_name'. Did you mean: 'card_names'?
```

**Fix**: Change `target_perm.top_card.card_name` to `target_perm.top_card.card_names[0]` (line 80).

---

## Focus Card Findings

### BT20-013 BaoHuckmon [Main] Effect -- FAIL (confirms prior report)

**Status**: Script has `_is_field_main = True` set correctly, but the [Main] effect still does not appear in the action mask after BT20-013 is on the field with valid targets in hand. However, the card was also self-deleted by the spurious OnEnterFieldAnyone "Delete, Effect Immunity" spam (see P-216 bug below), so BT20-013 never stays on the field to test its [Main] effect.

**Impact**: The [Main] effect that plays a Sistermon/Gankoomon from hand at -2 cost is unreachable. Combined with the self-deletion on play, BT20-013 is functionally broken in this matchup.

**Prior report reference**: BUG-1 in `2026-03-14-jesmon-rk-qa.md` documented the systemic `OnDeclaration` timing issue (97 scripts affected). This test confirms BT20-013 still has the issue despite having `_is_field_main = True` added.

---

### BT23-076 Sistermon Blanc On Play -- FAIL (confirms prior report)

**Status**: Three distinct bugs in the On Play effect:

| # | Expected Behavior | Actual Behavior |
|---|-------------------|-----------------|
| 1 | Add top security card to hand FIRST | Does Recovery +1 first |
| 2 | Card added from security stack | Card added from trash (if any) |
| 3 | No effect on opponent | Trashes opponent's top security card |

**Observed state changes** (10 memory, 5 security each side):
- P1 security: 5 -> 6 (wrong: should be 5, net -1+1=0)
- P2 security: 5 -> 4 (wrong: should stay 5)
- P1 hand: 5 -> 4 (wrong: should be 5, -1 play +1 from security = net 0)

**Script issues** (`bt23_076.py` lines 34-52):
1. Line 40: `player.recovery(1)` called before security-to-hand
2. Lines 42-44: Takes from `player.trash_cards.pop()` instead of `player.security_cards.pop(0)`
3. Lines 46-51: Trashes `enemy.security_cards.pop(0)` -- entirely fabricated behavior

**Card text**: "[On Play] Add your top security card to the hand. Then, Recovery +1 (Deck)."

---

### BT23-057 Gankoomon BeforePayCost -- PARTIAL (confirms prior report)

**Status**: Cost reduction works correctly. Process callback does not execute.

- Play cost correctly reduced: 11 - 5 = 6 (memory 10 -> 4)
- Trash cards NOT returned to deck (3 qualifying cards remained in trash)
- This is a systemic engine issue: `calculate_play_cost()` reads `cost_reduction` but never calls `on_process_callback` on BeforePayCost effects

**Impact**: Player gets the -5 cost discount for free without paying the cost of returning 3 cards from trash. This is an unfair advantage.

---

### BT14-044 Palmon Grant Effect -- PARTIAL

**Status**: The Start of Main Phase effect fires correctly and enters SelectTarget phase. Target selection works. However:

1. The granted `OnTappedAnyone` effect crashes the game when it fires (see CRASH-2 above)
2. The functional behavior (opponent loses 2 memory when suspended Digimon suspends) cannot be verified due to the crash

**Positive findings**:
- BT14-044 plays without being self-deleted (unlike other Digimon)
- Start of Main Phase timing fires on the correct turn
- `effect_select_opponent_permanent` correctly enters SelectTarget phase
- `grant_temp_effect` with `expiry_turn` correctly attaches the effect

---

## Engine-Level Issues

### CRITICAL: P-216 WaruMonzaemon Spurious OnEnterFieldAnyone Effects

**Severity**: CRITICAL (causes mass self-deletion of played cards)
**File**: `digimon_gym/engine/data/scripts/p/p_216.py`, effect2 (lines 66-103)

P-216's effect2 has timing `OnEnterFieldAnyone` with `is_on_play = True` but `condition2` always returns `True`. This means:
- It fires on EVERY card play (not just P-216's own play)
- It calls `effect_select_opponent_permanent` with `is_optional=False`
- When opponent has no Digimon, this causes the just-played card to be deleted

This causes 90+ spurious "Delete, Effect Immunity" log entries per card play and self-deletion of Digimon played by either side. This is the same engine-level issue documented in `2026-03-14-rocks-dark-masters-qa.md` as "Spurious SelectTarget phase after card play."

**Note**: The P-216 script appears to conflate two card abilities:
- What the card says: "The Digimon this effect played can't digivolve and is deleted at turn end"
- What effect2 does: Deletes an opponent's Digimon on play + grants self effect immunity

The condition should check that this is P-216's own play, not any card play.

---

### BeforePayCost Process Callbacks Not Executed (systemic)

Confirmed from prior report. `calculate_play_cost()` reads `cost_reduction` values but never calls `on_process_callback`. Affects BT23-057 and potentially many other cards with BeforePayCost costs.

---

### BT20-013 [Main] Effect Unreachable (systemic)

Confirmed from prior report. Despite `_is_field_main = True` being set, the action mask does not present the [Main] effect activation. The prior report documented 97+ scripts affected by the `OnDeclaration` timing issue.

---

## Card-Level Verdicts

| Card ID | Name | Deck | Verdict | Notes |
|---------|------|------|---------|-------|
| BT20-013 | BaoHuckmon | Jesmon | **FAIL** | [Main] effect unreachable (systemic); self-deleted by P-216 spam |
| BT23-076 | Sistermon Blanc | Jesmon | **FAIL** | On Play completely wrong: wrong zone, wrong order, harms opponent |
| BT23-057 | Gankoomon | Jesmon | **PARTIAL** | Cost reduction works; process callback (trash return) never fires |
| BT14-044 | Palmon | Dark Masters | **PARTIAL** | Grant fires correctly; crashes on log line (`card_name` vs `card_names`) |
| BT15-069 | Candlemon | Dark Masters | **CRASH** | `is_player_one` AttributeError crashes game |
| P-216 | WaruMonzaemon | Dark Masters | **FAIL** | Spurious OnEnterFieldAnyone fires on ALL plays, deletes arbitrary permanents |

---

## Regression Status

### Dark Masters Cards (from prior `2026-03-14-rocks-dark-masters-qa.md`)

| Card | Prior Status | This Session | Notes |
|------|-------------|--------------|-------|
| BT15-006 | PASS | OK | Digi-egg appears in games |
| BT16-082 | OK | OK | Ukkomon appears |
| P-216 | OK | **FAIL** | Spurious OnEnterFieldAnyone fires for all plays -- may have been masked in mirror |
| BT15-062 | OK* | OK* | Same spurious target selection (engine issue) |
| BT15-077 | OK | OK | LadyDevimon appears |
| BT15-069 | n/a | **CRASH** | Candlemon crashes -- `is_player_one` |
| BT15-070 | n/a | OK | DemiDevimon appears |
| BT14-044 | n/a | **PARTIAL** | Grant works but crashes on log line |
| BT15-072 | OK | OK | Vilemon appears |
| BT15-087 | n/a | OK | Appears in games |
| BT15-088 | n/a | OK | Appears in games |
| BT15-089 | n/a | OK | Appears in games |
| BT15-098 | n/a | OK | Appears in games |
| BT15-099 | n/a | OK | Appears in games |

### Jesmon Cards

| Card | Prior Status | This Session | Notes |
|------|-------------|--------------|-------|
| BT12-001 | PASS | OK | Gigimon digi-egg appears |
| BT20-008 | n/a | OK | Huckmon appears |
| BT23-006 | PASS | OK | Huckmon On Play fires |
| BT23-010 | n/a | OK | Appears in games |
| BT23-015 | n/a | OK | Appears in games |
| BT23-057 | IMPLEMENTED | **PARTIAL** | Cost reduction works, process callback missing |
| BT6-015 | PASS | OK | Jesmon appears |
| BT23-082 | n/a | OK | Appears in games |
| BT23-088 | n/a | OK | Appears in games |
| BT23-092 | n/a | OK | Appears in games |
| BT23-098 | n/a | OK | Appears in games |

---

## Methodology

### Automated Regression
- 10 random-policy games via direct `GameState` API (5 each direction)
- 10 greedy-policy games via direct `GameState` API (5 each direction)
- 10 greedy simulations via `/simulations` endpoint (5 each direction, 4 server timeouts)

### Debug Testing
- 5 debug games via `POST /debug/games` with `skip_shuffle`, `starting_hand1`, `initial_memory`
- Card injection via `POST /debug/games/{id}/inject-card`
- Memory manipulation via `POST /debug/games/{id}/set-memory`
- Internal state inspection via `GET /debug/games/{id}/internal-state`
- Action mask verification via `GET /games/{id}/action-mask`

### Known Limitation
- BT18-100 "Gospel of the Fallen Angel" crash (DelaySkill) not encountered (card not in either deck)
