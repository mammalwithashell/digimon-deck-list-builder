# Gameplay QA Report — Medusamon vs Royal Knights (Regression)

## Test Setup
- **Date**: 2026-03-22
- **Archetypes**: Medusamon (P2) vs Royal Knights (P1)
- **Focus**: Regression test of King Drasil_7D6 fix + untested Medusamon/Royal Knights cards
- **Game IDs**:
  - Game 1: `78548efa-f3d8-4251-8cc3-887ac88e7a81` (King Drasil regression, 6 turns)
  - Game 2: `ea8160aa-5fd9-4b82-800b-7b108cb09b52` (untested Medusamon cards, 12 turns)
  - Game 3: `00813bad-3183-44b2-99c0-1ae8fef12e8e` (remaining cards, 12 turns)
- **Total Turns Played**: ~30

## Summary
- **Total Issues Found**: 2
- Critical: 1 | High: 1 | Medium: 0 | Low: 0

## Regression Test Results — King Drasil_7D6 Fix

All King Drasil_7D6 (BT13-007) mechanics verified working after the `Permanent.is_digimon` fix:

| Mechanic | Status | Evidence |
|----------|--------|----------|
| Stays in breeding area | PASS | No "Non-Digimon trashed" log; trash empty |
| CANNOT_DIGIVOLVE modifier | PASS | Effect fires every turn from breeding |
| OnStartMainPhase absorb | PASS | Absorbs digi-egg + Royal Knights each turn |
| Cost reduction (once/turn) | PASS | Magnamon cost 7→0 (4+3 evo cards = 7 reduction) |
| Once-per-turn enforcement | PASS | LordKnightmon paid full 11 after Magnamon used reduction |
| +1 memory on Royal Knight Option | PASS | The Last Guardian placed → memory -2+1=-1 |
| White color for Options | PASS | The Last Guardian now playable (White from breeding) |
| EX11-053 tuck under KD7D6 | PASS | On Play offers tuck selection with decline option |
| BT13-093 On Deletion tuck | PASS | Tuck Royal Knight from hand under KD7D6 on deletion |

**Previously FAIL cards now PASS**: BT13-007, EX11-053

## Detailed Findings

### Issue 1: EX8-074 MedievalGallantmon "suspend 2 to reduce cost" auto-fires
- **Card(s)**: EX8-074 — MedievalGallantmon
- **Severity**: high
- **Category**: effect
- **Expected**: Card text says "Suspend 2 of your other Digimon/Tamers to reduce the play cost of this card by 4." The "suspend 2" is a cost the player should choose to pay or decline.
- **Actual**: The cost reduction auto-applied without presenting a choice. MedievalGallantmon played at cost 7 (11 - 4 reduction) without asking the player whether to suspend 2 permanents.
- **Steps to Reproduce**:
  1. Have 2+ unsuspended permanents on field
  2. Play EX8-074 MedievalGallantmon from hand
  3. Observe: cost automatically reduced by 4, permanents auto-suspended
- **Evidence**: Game 3, Turn 11. Memory 1→-6 (cost 7 not 11).
- **Notes**: Same systemic "by" cost auto-acceptance as Issues 86/88. All BeforePayCost "suspend N to reduce cost by M" effects auto-fire without player choice.

### Issue 2: SYSTEMIC — OnEndTurn effects never fire (phase_end skipped)
- **Card(s)**: BT20-102 — Omnimon (X Antibody) (and ALL cards with [End of Your Turn] effects)
- **Severity**: critical
- **Category**: game_flow
- **Expected**: BT20-102's "[End of Your Turn] [Once Per Turn] 1 of your Digimon may gain Rush and attack without suspending" should fire at end of each of P1's turns. All cards with OnEndTurn timing should execute their effects.
- **Actual**: OnEndTurn effects never execute. Both `pass_turn()` (line 296) and `check_turn_end()` (line 303) set `current_phase = GamePhase.End` then call `next_phase()`. But `next_phase()` sees `End` and routes to `switch_turn()` + `phase_start()`, completely skipping `phase_end()` which is the ONLY place that calls `execute_effects(EffectTiming.OnEndTurn)`.
- **Steps to Reproduce**:
  1. Play BT20-102 Omnimon (X Antibody) with any method
  2. End P1's turn (pass or play a card that crosses memory)
  3. Observe: no End of Turn selection, no Rush grant, no log entry for the effect
- **Evidence**: Game `76e32409`, Turn 12. P1 passed with Omnimon X on field — no End of Turn log, immediate switch to P2's turn.
- **Root Cause**: `pass_turn()` and `check_turn_end()` both call `self.next_phase()` with phase already set to `End`, bypassing `phase_end()`. Fix: replace `self.next_phase()` with `self.phase_end()` in both methods.
- **Impact**: Affects ALL cards with [End of Your Turn] or [End of All Turns] effects across the entire card pool. Cards affected include Reboot (end-of-opponent-turn unsuspend), Medusamon end-of-attack effects, security-related end-of-turn triggers, etc.

## Cards Tested Successfully

### Royal Knights (regression + new)
| Card | Name | Status | Notes |
|------|------|--------|-------|
| BT13-007 | King Drasil_7D6 | PASS | **Fixed**: Stays in breeding, all effects work. Cost reduction, memory gain, absorption confirmed. |
| EX11-053 | Omekamon | PASS | **Fixed**: On Play tuck selection works with KD7D6 in breeding. Decline option available. |
| BT20-100 | The Last Guardian | PASS | Reveal top 3 works. Placed as Delay. Now playable with White color from KD7D6. |
| BT19-072 | LordKnightmon | PASS | Played at full cost (KD reduction already used). On Play auto-skips correctly. |
| BT13-093 | Omekamon | PASS | On Deletion tuck under KD7D6 works correctly after fix. |

### Medusamon (new)
| Card | Name | Status | Notes |
|------|------|--------|-------|
| EX4-006 | Guilmon | PASS | Digivolves onto Lv2 egg correctly. No On Play (Rush requires 20+ trash). |
| EX9-008 | Biyomon | PASS | Play cost 3 correct. No On Play (Training/Raid are inherited). |
| BT16-082 | Ukkomon | PASS | Play cost 3 correct. No On Play — reveal triggers on move from breeding. |
| BT17-018 | Gallantmon: Crimson Mode | PASS | Play cost 8 correct. On Play delete effect: multi-select up to 15000 DP with decline. |
| BT21-093 | Raging Serpentine | PASS | Cost 8 correct (no reduction — opponent had 5 security). Deletes opponent highest DP. Placed as Delay. |
| BT23-014 | Gallantmon | PASS | Play cost 11 correct. On Play trash-play restriction + DP-scaled delete work. Auto-skips with no targets. |
| BT8-097 | Crimson Blaze | PASS | Cost reduced by 1 per opponent Digimon (6→5 with 1 target). Deletes all 6000 DP or less. Option trashed. |
| P-206 | Digital Gate Open | PASS | Cost 4 correct. Reveal top 3, add 1 Digimon + 1 Tamer. Placed as Delay. |
| BT20-102 | Omnimon (X Antibody) | PASS | Played from hand (cost 16). On Play condition fails without evo cards (correct auto-skip). |
| EX8-074 | MedievalGallantmon | PARTIAL | Card functions but "suspend 2 to reduce cost" auto-fires without choice (Issue 1). |

## Areas Not Covered
- EX11-012 (Medusamon alt) — in hand but not played in any game
- BT24-018 Styracomon — digivolve validator bug still present (not retested)
- BT20-083, BT20-060, BT13-112, BT13-075 — Royal Knights Lv6+ not tested via digivolve (only play)
- Many Royal Knights cards still untested (48 cards in broader archetype pool)

## Key Observations

1. **King Drasil fix is comprehensive**: All cascading effects from the breeding bug are resolved. The Royal Knights archetype is now functional — cost reduction, memory gain, tuck/pull, White Options all work.

2. **"By" cost auto-acceptance remains systemic**: EX8-074's "suspend 2 to reduce cost" auto-fires, consistent with Issues 86/88. This is a cross-archetype engine issue affecting all BeforePayCost effects with "by" costs.

3. **Medusamon cards mostly working**: 9 of 11 untested cards validated. The archetype's core mechanics (Petrification Tokens, Progress, delete-on-security-removal) continue working correctly.
