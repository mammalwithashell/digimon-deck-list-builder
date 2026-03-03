# Royal Knights Re-Test Follow-Up

- **Date**: 2026-03-02
- **Source Report**: `2026-03-01-royal-knights.md`
- **Verification Mode**: implementation follow-up only; deterministic gameplay re-test not executed in this session

## Summary

Code changes landed for the core Royal Knights blockers:
- BT13-007 King Drasil_7D6 now has a real `BeforePayCost` breeding reducer path wired through engine play-cost calculation.
- BT13-007 start-of-main effect was replaced with source absorption logic instead of the old incorrect immunity stub.
- BT23-072 King Drasil_7D6 now grants keywords to the played Digimon instead of itself.
- BT20-017 and BT23-057 now call `game.effect_play_token(...)` with registered Royal Knights token types.
- Conditional passive keywords now honor `can_use_condition()`, which affects Sistermon Blocker / Decoy behavior.
- Normal non-Delay Options now trash after resolution instead of staying in battle area.

## BT13-007 Status

- **Legal play actions updated**: yes, via `Game.calculate_play_cost()` in action mask generation.
- **Reduced memory charge implemented**: yes, via the shared play-cost helper.
- **Breeding-area restriction handling**: yes, by explicit opt-in (`_allow_breeding_source`) rather than globally enabling breeding-area reductions.

## Cards Re-Verified

No live gameplay re-verification was executed in this session.

Static implementation review covered:
- `BT13-007`
- `BT20-017`
- `BT23-057`
- `BT23-072`
- `BT6-082`
- `ST12-12`
- `BT23-047`

## Remaining Work

- Run debug-game validation against the original March 1 scenarios.
- Confirm memory deltas and action legality in live gameplay.
- Update the March 1 issue statuses to `FIXED` only after runtime confirmation.
