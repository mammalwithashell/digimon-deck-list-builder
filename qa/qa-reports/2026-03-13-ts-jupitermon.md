# TS Jupitermon Archetype QA Report

**Date**: 2026-03-13
**Archetype**: TS Jupitermon
**Cards Tested**: 8 (BT24-003, BT24-037, BT24-046, BT24-084, BT24-101, BT7-032, P-194, P-213)
**Method**: Debug game API + direct engine testing

---

## Summary

| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT24-003 | Tsunomon | PASS | Inherited OnLoseSecurity digivolve into Shaman trait fires correctly. OPT, optional, cost_reduction=1 all work. |
| BT24-037 | Silphymon | PARTIAL | On Play DP -5000 works. Auto-selects lowest DP (should be player choice). Force attack and SA+1/DNA bonus stubbed. WhenRemoveField play-from-sources implemented. |
| BT24-046 | Garurumon | FAIL | On Play/When Digivolving/Inherited suspend effects broken. Filter uses `getattr(p, 'owner', None) == player.enemy` but Permanent has no `owner` attribute -- filter always returns False. Jamming keyword works. |
| BT24-084 | Inori Misono | PARTIAL | Memory +1 triggers but MISSING "4 or less memory" condition check. Aegiomon->Aegiochusmon digivolve trigger fires correctly on security loss. Security play effect declared. |
| BT24-101 | Jupitermon | PASS | On Play/When Digivolving: trash own security, -13000 DP, conditional Recovery +2 all work correctly. OnLoseSecurity trash opponent security (OPT) works. TS protection effect has correct condition checks. |
| BT7-032 | Pulsemon | PASS | Inherited When Attacking: +2 memory if exactly 3 security. Verified triggers at 3 security, does NOT trigger at 4 security. OPT works. |
| P-194 | Aegiomon | PASS | Blocker, Barrier keywords both work (own effects and inherited). Alt-digi from TS Lv3 at cost 2 works. |
| P-213 | Aegiochusmon | PARTIAL | Raid, Decode keywords work. When Digivolving: Rush + 3000 DP correctly conditional on <=3 security. "Then, this Digimon may attack" stubbed (force_attack not implemented). Alt-digi from Aegiomon at cost 3 works. |

**Totals**: 4 PASS, 3 PARTIAL, 1 FAIL

---

## Detailed Findings

### BT24-003 Tsunomon (PASS)

**Card Text**: Inherited Effect [Your Turn] [Once Per Turn] When your security stack is removed from, this Digimon may digivolve into a [Shaman] trait Digimon card in the hand with the digivolution cost reduced by 1.

**Tests**:
- Built Tsunomon -> Aegiomon (BT24-034) stack on field
- Removed security card manually
- OnLoseSecurity effect fired, entered SelectTarget phase with valid Shaman-trait hand card
- Effect correctly filtered for Shaman trait
- OPT flag and optional flag both set correctly
- `cost_reduction=1` passed to `effect_digivolve_from_hand`

**Minor Note**: `effect_digivolve_from_hand` uses `card.get_cost_itself` (play cost) as base instead of digivolution cost. This is a known engine pattern, not a script bug.

### BT24-037 Silphymon (PARTIAL)

**Card Text**: [On Play] [When Digivolving] 1 of your opponent's Digimon gets -5000 DP for the turn. Then, 1 of your Digimon may attack. If DNA digivolving, 1 of your Digimon gains SA+1 and +5000 DP.

**Tests**:
- Played Silphymon with opponent Deltamon (5000 DP) on field
- DP -5000 applied correctly (Deltamon deleted at 0 DP)
- Play cost correct: 8 memory deducted

**Issues**:
1. DP target auto-selects lowest DP opponent (`min(dp_targets, key=lambda p: p.dp)`) instead of player choice via `effect_select_opponent_permanent`
2. Force attack effect stubbed (`pass # descriptive-tagged: force_attack`)
3. DNA digivolving SA+1 and +5000 DP bonus stubbed (`pass # descriptive-tagged: change_security_attack`)

**WhenRemoveField effect**: Implemented -- plays Lv4 or lower Yellow/Red/TS Digimon from digivolution cards. Auto-selects first candidate.

### BT24-046 Garurumon (FAIL)

**Card Text**: [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon. Inherited: [When Attacking] [Once Per Turn] Suspend 1 of your opponent's Digimon.

**Root Cause**: The filter function in the script checks:
```python
def target_filter(p):
    return (getattr(p, 'is_digimon', False) and
            getattr(p, 'owner', None) == player.enemy)
```
`Permanent` objects do not have an `owner` attribute. `getattr(p, 'owner', None)` always returns `None`, so the filter always fails and `effect_select_opponent_permanent` returns early with no valid targets.

**Fix**: Remove the `owner` check. The `effect_select_opponent_permanent` function already iterates only over opponent's battle area, making the owner check redundant. The filter should be `lambda p: p.is_digimon`.

**Affected lines**: bt24_046.py lines 62 and 131 (On Play and inherited When Attacking).

**Jamming keyword**: Works correctly (`_is_jamming = True`).

### BT24-084 Inori Misono (PARTIAL)

**Card Text**: [Start of Your Main Phase] If you have 4 or less memory, gain 1 memory. [All Turns] When your security stack is removed from, by suspending this Tamer, 1 of your [Aegiomon] may digivolve into [Aegiochusmon] from hand without paying the cost.

**Issue 1 (BUG)**: Memory gain effect (condition0) does NOT check the "4 or less memory" condition. It only checks `is_my_turn`. Verified: with 6 memory at Start of Main Phase, memory still increased to 7. The script should add `game.memory <= 4` to the condition.

**Aegiomon digivolve trigger**: Works correctly.
- OnLoseSecurity timing fires when security is removed
- Condition checks for Aegiomon on field
- Suspends this Tamer as cost
- Selects Aegiomon via `effect_select_own_permanent`
- Digivolves into Aegiochusmon from hand (no cost) via `effect_digivolve_from_hand`

**Security play**: Declared as `is_security_effect = True` (factory pattern).

### BT24-101 Jupitermon (PASS)

**Card Text**: [On Play] [When Digivolving] Trash your top security card and 1 of your opponent's Digimon gets -13000 DP. Then, if 1 or fewer security, Recovery +2. [All Turns] [OPT] OnLoseSecurity: trash opponent's top security. [All Turns] [OPT] When TS would leave, trash own security to prevent.

**Tests**:
1. On Play with 5 security: trashed to 4, no recovery (4 > 1). Correct.
2. On Play with 2 security: trashed to 1, Recovery +2 triggered (1 <= 1), ended with 3. Correct.
3. Multiple Jupitermon on field: each one's OnLoseSecurity triggers independently per turn.
4. Opponent's security depleted from 5 to 0 across 4 turns (cascading OnLoseSecurity triggers).

**DP target**: Auto-selects lowest DP (same pattern as BT24-037). Acceptable since no opponent Digimon were present in most tests.

**TS protection effect**: Has correct condition checks (TS trait, own battle area, security available, OPT).

### BT7-032 Pulsemon (PASS)

**Card Text**: Inherited [When Attacking] [Once Per Turn] If you have 3 security cards, gain 2 memory.

**Tests**:
- Built Pulsemon -> Aegiomon stack, attacked with 3 security: memory +2. Correct.
- Attacked with 4 security: no memory gain. Correct.
- Condition checks `len(owner.security_cards) != 3` (exact match). Correct per card text.
- OPT enforced via `set_max_count_per_turn(1)`.

### P-194 Aegiomon (PASS)

**Card Text**: Blocker. Barrier. Inherited: Barrier.

**Tests**:
- Blocker keyword (`_is_blocker = True`): confirmed via `has_keyword('_is_blocker')`.
- Barrier keyword (`_is_barrier = True`): confirmed via `has_keyword('_is_barrier')`.
- Inherited Barrier: verified under P-213 Aegiochusmon after digivolve. `has_keyword('_is_barrier')` returns True.
- Alt-digi from TS Lv3 at cost 2: `_alt_digi_trait = "TS"`, `_alt_digi_level = 3`, `_alt_digi_cost = 2`.

### P-213 Aegiochusmon (PARTIAL)

**Card Text**: Raid. Decode ([Aegiomon]). [When Digivolving] If <=3 security, Rush + 3000 DP. Then, this Digimon may attack. Inherited: Decode.

**Tests**:
- Raid keyword (`_is_raid = True`): works.
- Decode keyword (`_is_decode = True`): declared for both main and inherited.
- When Digivolving with 3 security: Rush granted, DP went from 7000 to 10000. Correct.
- When Digivolving with 5 security: effect did NOT fire. Correct.
- Alt-digi from Aegiomon at cost 3: works.

**Issue**: "Then, this Digimon may attack" is stubbed (`pass # descriptive-tagged: force_attack`). Engine lacks a general force-attack mechanism.

---

## Issues Summary

| # | Card | Issue | Severity | Status |
|---|------|-------|----------|--------|
| 66 | BT24-046 | Suspend filter uses nonexistent `owner` attribute on Permanent | critical | OUTSTANDING |
| 67 | BT24-084 | Memory gain missing "4 or less memory" condition | high | OUTSTANDING |
| 68 | BT24-037 | Force attack and SA+1/DNA bonus effects stubbed | med | OUTSTANDING |
| 69 | P-213 | Force attack after digivolve stubbed | med | OUTSTANDING |
| 70 | BT24-037 | DP target auto-selects instead of player choice | low | OUTSTANDING |
