## 1. Reproduce and Pin the Engine Invariant

- [x] 1.1 Add a Rust regression that plays a card which pays memory across zero and installs a mandatory pending selection.
- [x] 1.2 Assert the regression exposes the pending selection's legal action IDs instead of a pass-only `EndTurn` mask.
- [x] 1.3 Assert the regression fails on the current engine by showing unresolved `pending_selection` with `GamePhase::EndTurn`.

## 2. Fix Rust Turn Progression

- [x] 2.1 Update the Rust turn-end check so unresolved pending selections defer `end_turn()` / turn rotation.
- [x] 2.2 Confirm pending-selection resolution paths re-run turn-end checks after selections and follow-up effects finish.
- [x] 2.3 Preserve existing `EndOfTurnAction` behavior for Vortex, Overclock, MayAttack, and optional end-of-turn attack windows.
- [x] 2.4 Keep `ACTION_SPACE_SIZE`, tensor profiles, and exported action constants unchanged.

## 3. Verify Greedy and Headless Masks

- [x] 3.1 Add or update coverage proving Rust greedy selects a legal pending-selection action after its own setup play.
- [x] 3.2 Verify the headless action mask exposes `pending_selection.valid_action_ids` for all selection phases involved in the regression.
- [x] 3.3 Verify pass remains legal only for optional selections and normal pass-allowed phases.

## 4. Python/RL Smoke Verification

- [x] 4.1 Rebuild or refresh PyO3 bindings if local Rust changes require it.
- [x] 4.2 Run a Python reproduction through `DigimonEnv` and `OpponentWrapper` showing the greedy opponent advances past the previous pass loop.
- [x] 4.3 Run a short Rust-backed generalist pilot smoke using fully implemented decks.
- [x] 4.4 Confirm the smoke no longer records timeout draws caused by pass repeating in `EndTurn` with `winner_id=None` and `game_over=False`.

## 5. Documentation and Reference Notes

- [x] 5.1 Note the DCGO phase-flow reference used for turn/end processing in implementation comments or parity docs only where it clarifies behavior.
- [x] 5.2 Update `docs/RUST_PYTHON_PARITY.md` if the fix changes or clarifies a tracked Rust/Python divergence.
- [x] 5.3 Record final verification commands and outcomes in the implementation summary.
