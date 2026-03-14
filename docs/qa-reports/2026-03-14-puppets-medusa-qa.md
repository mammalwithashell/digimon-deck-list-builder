# QA Report: Puppets vs Medusamon
Date: 2026-03-14

## Test Setup
- **P1 Deck**: Puppets (dogortcg variant with 3x BT22-036)
- **P2 Deck**: Medusamon (LoggyMcFroggy 1st-place variant)
- **Engine**: localhost:8000 debug API
- **Method**: Manual scenario injection + automated greedy-vs-greedy games

## Automated Game Results

### Puppets (P1) vs Medusamon (P2) - 5 games
| Game | Winner | Turns | Steps |
|------|--------|-------|-------|
| 1 | P1 | 7 | 39 |
| 2 | P1 | 7 | 32 |
| 3 | P1 | 9 | 45 |
| 4 | P1 | 9 | 46 |
| 5 | P1 | 9 | 52 |

All 5 games completed without errors.

### Medusamon (P1) vs Puppets (P2) - 5 games
| Game | Winner | Turns | Steps |
|------|--------|-------|-------|
| 1 | P2 | 8 | 31 |
| 2 | P2 | 8 | 39 |
| 3 | P2 | 8 | 39 |
| 4 | P2 | 10 | 50 |
| 5 | P1 | 9 | 38 |

All 5 games completed without errors.

### Medusamon Mirror - 5 games
| Game | Winner | Turns | Steps |
|------|--------|-------|-------|
| 1 | P2 | 6 | 33 |
| 2 | P1 | 7 | 44 |
| 3 | P2 | 10 | 44 |
| 4 | P1 | 7 | 28 |
| 5 | P1 | 7 | 38 |

All 5 games completed without errors.

**Total: 15/15 games completed with no crashes or hangs.**

## BT22-036 (Chaperomon) - Detailed Validation

### Effect 0: Overclock ([Puppet] Trait)
- **Status**: Not tested independently (Overclock is a systemic keyword)

### Effect 1: [Hand][Main] Compound Flow
**Card text**: "[Hand][Main] If you have [Arisa Kinosaki], by placing 1 [ShoeShoemon] from your trash as any of your [Shoemon]'s bottom digivolution card, it digivolves into this card for a digivolution cost of 3, ignoring digivolution requirements."

**Test scenario**: Arisa Kinosaki + Shoemon on field, ShoeShoemon in trash (injected), BT22-036 in hand. Memory = 3.

**Result**: **FAIL**

**Bug found**: The `_on_trash_action` callback double-subtracts `_SEL_TRASH_START` (constant 130).

The engine's `_decode_trash_selection` (in `action_decoder.py` line 278) passes `idx` (already subtracted from the raw action_id by `SEL_TRASH_START`) to the callback. But the script's callback does:
```python
def _on_trash_action(action_id):
    idx = action_id - _SEL_TRASH_START  # 0 - 130 = -130
    on_shoeshoemon_selected(idx)
```

This produces a negative index (-130), causing the nested `on_shoeshoemon_selected` to either error silently or access the wrong element. The effect appears to trigger (enters SelectTrash phase correctly) but the follow-up digivolution never executes. The game silently returns to Main phase via `_recover_from_stale_selection()`.

**Fix needed**: Remove the subtraction in `_on_trash_action`:
```python
def _on_trash_action(trash_idx):
    on_shoeshoemon_selected(trash_idx)
```

**Same bug affects**:
- BT24-016 (Lamiamon) - Medusa archetype [Hand][Main] compound flow
- EX11-032 - same `_on_trash_action` pattern
- EX10-032 - same `_on_trash_action` pattern

### Effect 2: Inherited - WhenPermanentWouldBeDeleted
**Card text**: "[All Turns][Once Per Turn] When this Digimon would leave the battle area other than by your effects, by deleting 1 of your Tokens or other [Puppet] trait Digimon, it doesn't leave."

**Code review findings**:
- Timing (`WhenPermanentWouldBeDeleted`) is correct
- `is_inherited_effect = True` is correct (this is an inherited effect)
- `set_max_count_per_turn(1)` correctly implements once-per-turn
- Uses `_will_not_be_removed = True` flag which the engine checks after the timing fires
- Token/Puppet filter in `sub_filter` is correct
- Auto-selects first valid substitute (no agent selection during deletion -- acceptable simplification)

**Issue**: The condition does not check "other than by your effects". The `delete_permanent` method receives `is_opponent_effect` but doesn't pass it into the `WhenPermanentWouldBeDeleted` context. The script therefore doesn't filter self-caused deletions.

**Status**: **PARTIAL** - The deletion prevention mechanism works correctly in principle, but does not enforce the "other than by your effects" restriction. In practice this rarely matters since players don't typically delete their own Digimon.

## BT24-016 (Lamiamon, Medusa) - Cross-check

**Status**: **FAIL** (same bug as BT22-036 Effect 1)

The [Hand][Main] compound flow has the identical `_on_trash_action` double-subtraction bug. The trash selection triggers but the Dimetromon placement + digivolution never executes.

Note: This card was previously marked as fixed in `medusa.md` ("Converted to alt-digi pattern"), but the actual script still uses `_is_hand_main = True` and has the broken callback.

## Medusa Archetype Regression

All 20 unique Medusa deck cards were exercised across 10 automated games (5 mirror + 5 vs Puppets). No crashes, hangs, or errors observed.

**Cards tested via gameplay** (20 unique in deck):
BT18-087, BT21-001, BT21-008, BT21-017, BT21-025, BT21-029, BT21-081, BT23-005, BT24-008, BT24-011, BT24-012, BT24-016, BT24-017, BT24-018, BT24-082, BT24-089, LM-027, P-035, P-103, P-189

**Regression status**: PASS (no regressions detected)

**Exception**: BT24-016's [Hand][Main] compound flow is broken (see above), but this was not detected in automated games because the greedy agent may not encounter the specific board state required (Owen Dreadnought tamer + Elizamon on field + Dimetromon in trash).

## Summary

| Card | Effect | Status | Notes |
|------|--------|--------|-------|
| BT22-036 | Overclock | N/T | Systemic keyword, not tested independently |
| BT22-036 | [Hand][Main] compound flow | **FAIL** | `_on_trash_action` double-subtracts SEL_TRASH_START |
| BT22-036 | Inherited deletion prevention | **PARTIAL** | Missing "other than by your effects" check |
| BT24-016 | [Hand][Main] compound flow | **FAIL** | Same `_on_trash_action` bug |
| Medusa archetype (20 cards) | Regression | **PASS** | 10 games, no errors |
| Puppets archetype (all) | Smoke test | **PASS** | 10 games, no errors |

## Systemic Issue: `_decode_trash_selection` callback convention

The engine's `_decode_trash_selection` passes the adjusted index (`action_id - SEL_TRASH_START`) to callbacks, unlike `_decode_selection` which passes the raw `action_id`. Scripts using `request_selection(GamePhase.SelectTrash, ...)` with custom `_on_trash_action` callbacks that subtract `_SEL_TRASH_START` internally will all break.

**Affected scripts**: BT22-036, BT24-016, EX11-032, EX10-032

**Root cause options**:
1. Fix all 4 scripts to not subtract `_SEL_TRASH_START` in callbacks
2. OR fix `_decode_trash_selection` to pass `action_id` instead of `idx` (matching `_decode_selection` convention)

Option 2 is the safer systemic fix since it aligns all decoder callbacks to the same convention.
