# Gameplay QA Report — Royal Knights

## Test Setup
- **Date**: 2026-03-03
- **Archetype**: Royal Knights
- **Game IDs**: 221e53f7-015a-4d6d-849c-eadb7f75d935, 51d938de-7b84-44fd-9084-d09a8dfd2192
- **Total Turns Played**: 2 (focused card-by-card testing via API)
- **Focus Areas**: play costs, keyword verification, token creation, option lifecycle, cost reduction, On Play effects
- **Pre-game fixes**: BT13-111 cost reduction + delete logic, P-186 cost reduction + delete-or-recovery

## Summary
- **Total Issues Found**: 3
- Critical: 0 | High: 1 | Medium: 1 | Low: 1

## Detailed Findings

### Issue 1: BT20-056 Alphamon DP shows 3000 instead of 11000
- **Card(s)**: BT20-056 — Alphamon
- **Severity**: medium
- **Category**: game_flow
- **Expected**: DP should be 11000 per card database
- **Actual**: Battle area shows DP=3000 after playing
- **Evidence**: cards.json has dp=11000, CardDatabase returns dp=11000, but in-game permanent shows 3000
- **Notes**: May be related to how DP is initialized on the Permanent from CardSource. Other cards (Jesmon 11000, Gallantmon 13000) display correctly.

### Issue 2: BT23-057 Gankoomon cost reduction always applies without trash cost
- **Card(s)**: BT23-057 — Gankoomon
- **Severity**: high
- **Category**: play_cost
- **Expected**: Cost reduction of 5 should only apply when player returns 3 Huckmon/Sistermon/Jesmon cards from trash to deck
- **Actual**: Cost reduction always applies unconditionally (condition only checks `card_source is not card`)
- **Evidence**: Gankoomon played for cost 1 (base 11 - 5 BT13-007 breeding - 5 unconditional Gankoomon = 1) with empty trash
- **Notes**: Pre-existing script issue in bt23_057.py. The trash-return cost step is not implemented.

### Issue 3: BT13-111 / P-186 cost reduction and delete effects were stubbed
- **Card(s)**: BT13-111, P-186 — Gallantmon
- **Severity**: low (now fixed)
- **Category**: effect
- **Expected**: Variable cost reduction based on trash count; conditional delete effects
- **Actual**: Cost reduction was a no-op stub; BT13-111 delete filter was impossible (required DP <= 6000 AND >= 13000)
- **Evidence**: Fixed in this session before gameplay testing
- **Fix**: BT13-111: Added BeforePayCost with `_cost_reduction_value_fn`, rewrote delete to try low DP first then high DP. P-186: Added BeforePayCost with 13000+ DP condition, rewrote On Play/When Digivolving to delete-or-recovery.

## Cards Tested Successfully

### Game 1 (221e53f7)
| Card | Name | Result | Notes |
|------|------|--------|-------|
| BT23-072 | King Drasil_7D6 | PASS | Play cost 6 correct. Keyword grant fires when Royal Knight/CS played: Rush/Raid/Reboot/Blocker granted to Gankoomon on play, King Drasil suspends as cost. Correctly doesn't re-grant when already suspended. |
| BT23-057 | Gankoomon | PARTIAL | Play cost reduced correctly by BT13-007 breeding. Hinukamuy Token created with Blocker/Reboot/Alliance. Rush granted, can attack same turn. However: own cost reduction (-5) applies unconditionally without trash-return cost. |
| P-186 | Gallantmon | PASS | Play cost 12, reduced by BT13-007 breeding to 7. Rush and Blocker keywords present. On Play: no 13000+ DP target, Recovery +1 fired correctly (security 5→6). |
| BT8-097 | Crimson Blaze | PASS | Play cost 6 correct. Option effect fires. Card goes to trash after resolving (previously stayed in battle area). |
| BT13-007 | King Drasil_7D6 (egg) | PASS | Hatched to breeding. Start-of-main absorption effect runs. Cost reduction: 4 + digivolution cards correctly applied to Royal Knight plays. |
| BT20-017 | Jesmon | PASS (partial in Game 1) | Drew into hand later. Verified via Game 2. |

### Game 2 (51d938de)
| Card | Name | Result | Notes |
|------|------|--------|-------|
| BT6-082 | Sistermon Blanc | PASS | Play cost 3. On Play Draw 1 fires. No Blocker initially (no Royal Knight on field). After Jesmon played, Blocker appears (conditional keyword working). |
| BT20-017 | Jesmon | PASS | Play cost 11, reduced to 6 by BT13-007 breeding. Atho, Rene & Por Token created with Blocker/Reboot/Decoy. "When another Digimon played" triggers from Token play. |
| BT20-056 | Alphamon | PARTIAL | Play cost 0 correct. Barrier keyword present (previously missing). Recovery +1 fires. Optional digivolve selection appears and can be declined. However DP displays as 3000 instead of 11000. |
| ST12-12 | Sistermon Blanc | PASS | Play cost 3. On Play trash-to-draw works (trash 1, draw 2). Decoy keyword present when Royal Knight on field (previously missing). |
| BT9-103 | Kongou | PASS | Play cost 2 correct. Effect fires. Card goes to trash after resolving (previously stayed in battle area). |
| BT13-111 | Gallantmon | PASS | Play cost 13, reduced to 8 by BT13-007 breeding (own cost reduction not active — Digimon exist). Rush keyword present. On Play delete effect fires (no targets available, no crash). |
| BT23-047 | Examon | PASS (static) | Not played in game but script verified: has `_is_piercing = True`, `_security_attack_modifier = 1`, Partition effects. |

## Pre-Test Script Fixes Applied
1. **BT13-111**: Added `BeforePayCost` cost reduction with `_cost_reduction_value_fn` (2 per 5 trash cards, requires no Digimon in play). Rewrote delete filter: try 6000 DP or less first, fallback to 13000+ DP.
2. **P-186**: Added `BeforePayCost` cost reduction with `_cost_reduction_value_fn` (2 per 5 trash cards, requires 13000+ DP Digimon on field). Rewrote On Play/When Digivolving: delete 13000+ DP or Recovery +1.

## Areas Not Covered
- BT23-047 Examon: Not played in game (Lv.7, expensive). Piercing/SA+1 verified via static analysis.
- P-186 cost reduction: Not triggered in game (no 13000+ DP Digimon when played). Code-verified.
- BT13-111 cost reduction: Not triggered in game (Digimon already existed). Code-verified.
- BT23-072 inherited effect (play King Drasil from sources with 6+ digi cards): Not tested (insufficient digi cards).
- BT23-057 Gankoomon delete effect: No opponent Digimon available to delete.
- Attack-time interactions (Piercing, Security Attack +1, Barrier blocking): Not tested in this session.
