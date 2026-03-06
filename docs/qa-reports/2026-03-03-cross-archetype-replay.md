# Gameplay QA Report — Cross-Archetype Replay

## Test Setup
- **Date**: 2026-03-03
- **Matchup**: Royal Knights (P1) vs Medusa (P2)
- **Game IDs**: c92e4b1e-cba1-4ff4-a43a-d7e3f346b3ce, fefcf19b-8a3b-4075-8a8e-14cb2d41133e
- **Focus Areas**: Verify trigger-context improvements prevent selection-phase crash (Report 20 #1)

## Summary
- **Total Issues Found**: 1
- Critical: 0 | High: 1 | Medium: 0 | Low: 0
- **Key Finding**: Selection-phase deadlock persists in cross-archetype games

## Detailed Findings

### Issue 1: Cross-archetype deadlock after selection phase decline
- **Severity**: high
- **Category**: game_flow
- **Expected**: After declining an optional On Play selection, game should return to Main phase with full action mask
- **Actual**: Game returns to Main phase but action mask is empty (no valid actions), creating an unrecoverable deadlock
- **Evidence**: Game fefcf19b — P2 digivolves Dimetromon onto Elizamon, When Digivolving selection triggered, declined, game gets stuck with empty action mask. Game c92e4b1e — same pattern after Elizamon On Play selection decline.
- **Notes**: This appears to be the same issue as Report 20 #1 (trigger-context handling). The Phase 2D selection deadlock fix improved some scenarios but cross-archetype matchups with Medusa On Play/When Digivolving selections still deadlock.

## Successful Observations (Game c92e4b1e)
- P1 played Omekamon (cost 4) and Jesmon (cost 6 with breeding reduction) — both correct
- Atho, Rene & Por token created on Jesmon play
- P1 attacked player with Omekamon — security check worked correctly
- P1 attacked player with token — security battle (6000 vs 2000 DP) resolved correctly
- No crash during attack sequences (original Report 20 concern was crash during attacks)

## Conclusion
Attack-time interactions between Royal Knights and Medusa work correctly. The remaining issue is the selection-phase deadlock when Medusa cards trigger On Play/When Digivolving optional selections — declining these leaves the game stuck. This is an engine-level issue in `_decode_selection()` that requires further investigation.
