> **STATUS: DEFERRED — DO NOT START.** These tasks are a phasing sketch, not scheduled work. Do not begin any phase without an explicit decision to prioritize this change. Each phase is gated on differential testing against the Python path before cutover.

## 0. Prioritization gate (must clear before any task below)

- [ ] 0.1 Explicit decision to schedule this migration (owner + sequencing vs other engine work).
- [ ] 0.2 Confirm `make-training-build-legacy-free` has landed (training is already legacy-free; the server is the remaining surface).

## 1. Deck rules → Rust deck tools (lowest risk)

- [ ] 1.1 Expose parse/validate/summarize + restricted-list over PyO3 from `code/digimon-engine/src/deck_tools.rs`.
- [ ] 1.2 Differential-test against `engine_py_legacy.engine.data.deck_loader` (`parse_deck`, `validate_deck`, `summarize_deck`, `RESTRICTED_LIST`, `CardRestriction`) over a deck corpus.
- [ ] 1.3 Migrate `simulations.py`, `lobby.py`, `db/routers/decks.py`, `db/routers/training.py`; re-home `PlayerType` and small enums off legacy.

## 2. Replay + recordings → Rust replay core

- [ ] 2.1 Expose the replay core (`runners/replay.rs`) for server use via PyO3.
- [ ] 2.2 Recording-format compatibility gate over a corpus of existing recordings.
- [ ] 2.3 Migrate `state.py`, `replays.py`, `recordings.py` off `ReplayRunner`/`HeadlessGame`.

## 3. State redaction over Rust state

- [ ] 3.1 Implement redaction (Rust-native or thin Python over PyO3 state) satisfying the `state_filter` contract.
- [ ] 3.2 Differential-test redacted output (player + spectator) against `engine_py_legacy.engine.state_filter`.
- [ ] 3.3 Migrate `ws_manager.py`, `ws_games.py`.

## 4. Live PvP runtime → interactive Rust runner (highest risk)

- [ ] 4.1 Add an interactive Rust runner PyO3 surface (selection prompts, per-player observation, play-order/turn machinery); preserve the Python 1/2 ↔ Rust 0/1 player-ID convention.
- [ ] 4.2 Shadow-run against the Python engine; compare turn-by-turn outcomes.
- [ ] 4.3 Migrate `lobby.py`, `matchmaking.py`, `ws_*` to the Rust interactive runner; per-route flag for staged cutover.

## 5. Admin AI script_promotion

- [ ] 5.1 Decide retire vs migrate `script_promotion` (`db/routers/admin_ai.py`) with the admin AI pipeline owner.
- [ ] 5.2 Execute the decision.

## 6. Close-out

- [ ] 6.1 Assert `code/server/` imports zero `engine_py_legacy` (guardrail test, mirroring the training guardrail).
- [ ] 6.2 Retire `docs/RUST_PYTHON_PARITY.md`; update `docs/ARCHITECTURE.md` and `docs/DEPLOYMENT.md`.
