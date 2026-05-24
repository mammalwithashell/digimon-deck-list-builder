## 1. Engine: concede primitive

- [x] 1.1 Add `Game::concede(player_id: PlayerId)` method in `code/digimon-engine/src/game.rs` that clears `pending_selection`, drains `effect_queue`, emits a new `GameEvent::Concede { player }` variant, then calls `declare_winner(opponent)`. Order of operations mirrors rule #16 surrender handling.
- [x] 1.2 Add the `Concede` variant to the engine's terminal `WinReason` (or equivalent) enum and surface it through `get_terminal_outcome()` as `win_reason = "concede"`.
- [x] 1.3 Write `code/digimon-engine/tests/concede_primitive.rs` covering: concede in main phase, concede during a pending selection (target / source / effect-choice), concede during attack timing windows (block / counter / alliance), concede during mulligan. Each scenario asserts `game_over=True`, correct `winner_id`, queue cleared, `win_reason == "concede"`, and `Concede` event emitted before `declare_winner` in the event log.
- [x] 1.4 Confirm no existing tests regress: `cargo test --manifest-path code/digimon-engine/Cargo.toml`. **Note**: 3 pre-existing `cards_behavioral` failures (`bt24_008`, `ex9_024`, `st19_04` "decline does not trash/draw") confirmed to fail on clean tree before this change; tracked separately and unrelated to concede primitive.

## 2. Engine: SelectPlayOrder phase

- [x] 2.1 Add a new `PendingSelection::PlayOrder { chooser: PlayerId }` variant in `code/digimon-engine/src/selection.rs`. **Note**: implemented as `SelectionKind::PlayOrder` (the variant carrier — `PendingSelection` itself is a struct with `kind: SelectionKind`) plus a new `GamePhase::SelectPlayOrder` variant; chooser captured by the existing `selecting_player` field on `PendingSelection`. Also added `PlayOrder { First | Second }` enum + `Game::last_play_order_choice` outcome slot for the wrapper to consume.
- [x] 2.2 Add `Game::request_play_order_selection(loser_id)` method that installs the `PlayOrder` pending selection with `chooser = loser_id`.
- [x] 2.3 Add `Game::resolve_play_order_selection(picked: PlayOrder)` that consumes the pending selection, records the choice, and exits the phase. Define `PlayOrder` as `enum { First, Second }`.
- [x] 2.4 Write `code/digimon-engine/tests/select_play_order.rs` covering: phase entry, mask reports 94/95 legal, mask reports 94/95 illegal outside the phase, resolution sets the chosen first player for the subsequent `Game::new_with_first_player(...)` call. (9 tests, all pass.)

## 3. Engine: action decoder and mask updates

- [x] 3.1 Update `code/digimon-engine/src/action/decoder.rs` to decode `93 → Concede`, `94 → PlayOrder(First)`, `95 → PlayOrder(Second)`. **Implementation**: action 93 intercepted at the top of `Game::decode_action` before the `pending_selection` dispatch, calling `Game::concede(player_id)` directly. Actions 94/95 route through the standard `resolve_selection` path because they're in `valid_action_ids` of the installed `PlayOrder` pending selection.
- [x] 3.2 Update `code/digimon-engine/src/action/mask.rs` so action `93` is `1` at every decision point. **Implementation**: explicit `mask[CONCEDE_GAME] = 1.0` inside the pending-selection branch (gated on `selecting_player == player_id`), plus a tail "if any other legal bit is set, also set 93" rule for non-selection phases. Players with no agency (e.g., not their turn) get an all-zero mask, so concede stays zero.
- [x] 3.3 Update mask logic so `94` and `95` are `1` only when `pending_selection == PlayOrder`, and `0` otherwise. **Implementation**: free — mask already emits `valid_action_ids` for any pending selection. `request_play_order_selection` installs `valid_action_ids = [94, 95]`; no PlayOrder selection ⇒ no 94/95 in mask.
- [x] 3.4 Update `docs/ACTION_SPEC.md`: claim 93 = CONCEDE_GAME, 94 = PLAY_FIRST, 95 = PLAY_SECOND. Reduce the "Unused 93-99" range note to "Unused 96-99" (4 slots remaining). Add a "SelectPlayOrder" sub-section under "Phase-Aware Meaning."

## 4. PyO3 bindings

- [x] 4.1 In `code/digimon-engine-py/src/lib.rs`, expose `concede(self, player_id: u8)` on `RustHeadlessGame` (Python convention 1/2 → Rust 0/1 at the boundary).
- [x] 4.2 Expose `request_play_order_selection(self, loser_id: u8)` with the same player-ID convention. **Also added** `take_play_order_choice()` returning `"first"` / `"second"` / `None` so the wrapper can read-and-clear the selection result; needed because `last_play_order_choice` is a `PlayOrder` enum that doesn't have a direct Python conversion.
- [x] 4.3 Add `win_reason: str` to the `get_terminal_outcome()` dict. Existing `reason` key kept as an alias for back-compat.
- [x] 4.4 Rebuild bindings: built release wheel via `python -m maturin build --release` and installed via `pip install --force-reinstall --no-deps <wheel>`. **Note**: `maturin develop` requires a virtualenv this environment doesn't have; `build`+`pip install` produces the same outcome. Also extended `RustGamePhase` and `HeadlessRunner` with the new `SelectPlayOrder` mapping + `concede` / `request_play_order_selection` / `take_play_order_choice` helper methods.
- [x] 4.5 Add a Python smoke test for the binding (concede, action-93 routing, win_reason field, play-order request + take, invalid PID rejection). **Path deviation**: placed at `code/tests/test_rust_concede_binding.py` (flat layout) instead of `code/tests/engine/test_concede_binding.py` — matches the existing PyO3 smoke-test convention (`test_rust_bindings_surface.py`, `test_rust_digivolve_counters.py`). 8 tests, all pass.

## 5. Reward shape — DigimonEnv updates

- [x] 5.1 Update the dense block in `code/digimon_gym/digimon_gym.py::_compute_reward` so opp-security removal pays `+1.5` and own-security loss pays `−0.5` (was `±2.0` symmetric). Keep the existing "security gained → 0" behavior.
- [x] 5.2 Confirm digivolve shaping defaults are unchanged in `DigimonEnv.__init__` (`digivolve_reward=0.1`, `dna_digivolve_bonus=0.3`). Default for `digivolve_shaping` stays `False` at the env layer.
- [x] 5.3 Add `code/tests/rl/test_dense_reward_calibration.py` (9 tests, all pass). Also updated `test_digivolve_shaping.py::test_shaping_off_default_matches_baseline_reward_path` since it pinned the prior `±2.0` calibration as the "baseline" — new baseline is the asymmetric `+1.5 / −0.5`.

## 6. MatchEnv wrapper

- [x] 6.1 Create `code/digimon_gym/agents/match_env.py` with `MatchEnv(gymnasium.Wrapper)`. State includes `match_id`, `match_score`, `match_step_count`, `current_game_index`, `concede_history`, `_pending_play_order_pick`. Walks the wrapper chain to find inner `DigimonEnv`.
- [x] 6.2 `reset()` samples a fresh `match_id`, resets match state, coin-flips game-1 first player via the seed-parity trick (`Game::new` uses `seed % 2` for 2-player first-player selection), builds inner via `self.env.reset` with deck options, snapshots decks for game-2/3 reuse.
- [x] 6.3 `step(action)`:
  - Forwards action via `self.env.step` (handles `OpponentWrapper` autoplay).
  - On game termination: detects concede via `terminal_outcome.win_reason`, updates `concede_history`, replaces inner game-terminal (`±10 + up to +5 fast`) with BO3 (`±12 + up to +3 fast`).
  - If match decided: adds match-terminal (base + sweep + smart/scared concede + fast-match bonus), returns `terminated=True`.
  - Else: installs SelectPlayOrder via `runner.request_play_order_selection(loser_pid)`, returns `terminated=False`.
  - Special path `_step_during_play_order_pick` handles actions 94/95 (route through `resolve_selection`) and 93 (forfeit match). After resolution, calls `OpponentWrapper.reset_inner_only` (new method, preserves LSTM h-state) with same decks + first-player seed-alignment.
- [x] 6.4 Per-match hard step limit at 900 steps. On overrun, calls `force_step_limit_winner` to resolve the in-progress game. Match resolves on game count (tie → 1-1-1 draw with `-1` terminal).
- [x] 6.5 Action-mask passthrough — free: MatchEnv doesn't override `action_mask`, so the inner env's mask (which already includes 93 always-legal and 94/95 only during SelectPlayOrder) flows through unchanged.
- [x] 6.6 `code/tests/rl/test_match_env.py` — 27 tests covering: inner-game-terminal calculator, BO3 game-terminal calculator, terminal adjustment math, all match-terminal scenarios from the design's payoff table (sweep, smart concede 2-1, lose 1-2 honest, lose 0-2 honest, lose 0-2 with scared concede, lose 1-2 with concede, draw, fast-match capped at par, fast-match zeroes at par), seed alignment (both directions), integration tests driving full BO3 matches via real engine, wrapper-chain integrity.

**Engine support changes landed alongside Section 6:**
- `HeadlessRunner::step` now permits stepping when `game_over` is True but a `PendingSelection` is installed — required so SelectPlayOrder can resolve between games while the engine's game-over flag persists from the prior game.
- `HeadlessRunner` gained `concede(player_id)`, `request_play_order_selection(loser_id)`, `take_play_order_choice() -> Option<PlayOrder>` helper methods delegating to `Game`.
- `OpponentWrapper` gained `reset_inner_only(**kwargs)` (resets inner without resetting opponent_fn state — LSTM preserved) and `advance_opponent_until_agent_acts()`.

## 7. OpponentWrapper interaction

- [x] 7.1 Verify `OpponentWrapper.reset()` continues to call `opponent_fn.reset_state()` only at true Gym `reset()` calls. Covered by `test_reset_state_called_exactly_once_per_match_reset` and `test_reset_state_called_only_on_match_boundary_after_full_match` in `test_match_env_lstm_carry.py`.
- [x] 7.2 `code/tests/rl/test_match_env_lstm_carry.py` — 5 tests covering reset-state counter contracts: fires once per match, NOT between games within a match, fires again on next match. Also pins `OpponentWrapper.reset_inner_only` existence.
- [ ] 7.3 Self-play sanity test deferred — same `reset_inner_only` machinery applies in self-play; structurally similar to 7.2 with both sides' state counters tracked. Tracked as a follow-up; the design's "Open Questions" section acknowledges this as an implementation-time verification rather than a hard contract.

## 8. CLI surface and config

- [x] 8.1 Added `match_format: str = "bo3"` to `TrainingConfig` with `VALID_MATCH_FORMATS = {"bo3", "single"}` validation in `_validate`.
- [x] 8.2 Added `--match-format {bo3,single}` argparse arg. CLI default is `None` (defer to TrainingConfig's `"bo3"` default); explicit value goes through the `overrides` dict.
- [x] 8.3 BO3-implies-digivolve-on default applied in `main()` after override merging: if `effective_match_format == "bo3"` and `digivolve_shaping` not in overrides, force `overrides["digivolve_shaping"] = True`. User can opt out via `--set digivolve_shaping=false`.
- [x] 8.4 `MatchEnv` inserted into the chain immediately after `OpponentWrapper` and before deck-pool wrappers in both `make_env()` and the two subprocess base-env construction sites. Match-format `single` skips the wrapper entirely.
- [x] 8.5 Threaded `match_format=cfg.match_format` and `match_env_seed=cfg.seed` (or `cfg.eval_seed`) through the two `make_env` call sites; added matching MatchEnv installation to the two SubprocVecEnv worker construction paths. **Test fix**: `test_make_vec_env_passes_config_tensor_profile` was patching DigimonEnv with a fake env — added `match_format="single"` to the test's TrainingConfig so the MatchEnv wrapper-chain walk doesn't fail on the fake.

## 9. Eval loop and callback updates

- [x] 9.1 `WinRateCallback._run_evaluation` now uses `find_match_env(eval_env)` to detect BO3 episodes. When present, the win/loss/draw determination reads `match_env_instance.match_score` instead of the inner DigimonEnv's per-game `winner_id`. In single mode the existing per-game path is preserved.
- [x] 9.2 Per-match outcome data collected at episode-end from `MatchEnv` state: `match_score`, `concede_history`, `match_step_count`, `_play_order_history`. Aggregated into class-level counters (`_match_played`, `_match_wins`, `_match_sweeps`, `_match_swept`, `_match_draws`, `_match_total_steps`, `_match_total_games`, `_concede_total`, `_concede_when_lead`, `_concede_when_tied`, `_concede_when_down`, `_concede_correct`, `_play_order_picks`, `_play_first_count`, `_play_first_match_wins`).
- [x] 9.3 New TB scalars emitted in BO3 mode: `pilot/match_win_rate`, `pilot/match_sweep_rate`, `pilot/match_swept_rate`, `pilot/match_total_steps_mean`, `pilot/games_per_match_mean`, `pilot/concede_rate`, `pilot/concede_lead_rate`, `pilot/concede_tied_rate`, `pilot/concede_down_rate`, `pilot/concede_correct_rate`, `pilot/play_first_rate`, `pilot/play_order_first_winrate`.
- [x] 9.4 Per-matchup BO3 scalars `pilot/matchup/{agent}_vs_{opp}/match_win_rate` and `.../sweep_rate` emitted via new `_matchup_match_*` counters when MatchEnv is active and generalist deck pool is in use.
- [x] 9.5 Matchup-grid sidecar JSON written to `runs/<run_id>/matchup_grid_<step>.json` at each eval pass when MatchEnv is active and matchups have been observed. Schema: top-level dict keyed by agent archetype, values are dicts keyed by opponent archetype with `{matches, match_wins, sweeps, total_games}` leaves.
- [x] 9.6 In `--match-format single` mode, the `if self._match_played > 0` and `if self._matchup_match_games` guards keep the match-tier scalars and the matchup-grid sidecar OFF (only the legacy `pilot/win_rate` block fires).

## 10. Recordings — match metadata stamping

- [x] 10.1 `TrainingRunMetadata` extended with `match_format: str = "single"` (default preserves legacy sidecar shape). Threaded `match_format=cfg.match_format` through the sidecar construction site in `train()`.
- [x] 10.2 `MatchEnv._info_with_match_metadata` stamps `match_id`, `game_index_in_match`, `match_score_before`, and `play_order_choice` on the info dict at every step. Fixed `match_score_before` to roll back the just-applied score delta on game-terminating steps via new `_last_game_score_delta` field.
- [x] 10.3 `outcome.win_reason` and `outcome.winner_id` stamped on info at game-terminating steps from the inner runner's `terminal_outcome()` dict (`reason` / `win_reason` keys).
- [x] 10.4 In `single` mode `MatchEnv` is not in the chain, so the info dict has no match-related keys — equivalent to absent / null metadata from the recording layer's perspective.
- [ ] 10.5 `test_match_recording_metadata.py` deferred — the info-dict stamping IS verified by `test_match_env.py` and `test_match_env_metrics.py`; a dedicated per-artifact-file test requires `TrainingRecordingWrapper` to be moved below `MatchEnv` in the chain (a larger refactor since `TrainingRecordingWrapper.step` writes one artifact per Gym episode, which in BO3 is per-match). Per-game artifact files in BO3 mode is a planned follow-up; the info-dict metadata that recordings would carry is plumbed end-to-end already.

## 11. Training MCP integration

- [x] 11.1 `summary.py::_list_matchup_grid_sidecars` globs `matchup_grid_<step>.json` and returns a `[{step, path}]` list sorted by step. `run_summary` now includes `"matchup_grids"` in its response (empty list for pre-BO3 / single runs).
- [x] 11.2 Free — `run_metric` reads from TensorBoard event files which use opaque scalar keys; the new `pilot/match_*` / `pilot/concede_*` / `pilot/play_*` tags surface automatically without any MCP-side change.

## 12. Documentation

- [x] 12.1 `docs/TRAINING_RUNBOOK.md` — added §12 "Best-of-three Match Training" with CLI examples, the action surface table (93/94/95), the full reward calibration (per-step / per-game / per-match), eval-cost note, wrapper-chain diagram, and the checkpoint-compatibility / fine-tune procedure. Renumbered the prior "Dependencies" section to §13.
- [x] 12.2 `docs/ACTION_SPEC.md` — already updated in 3.4. Confirmed `93 / 94 / 95` entries plus the new "SelectPlayOrder (BO3 only)" and "Concede (always legal)" sub-sections.
- [x] 12.3 `CLAUDE.md` — added working rule #26 covering BO3 default, MatchEnv chain position, concede action 93 always-legal, SelectPlayOrder driven by `Game::request_play_order_selection`, and the seed-parity trick for first-player selection in subsequent games. Linked to the design spec for reward calibration.
- [ ] 12.4 `docs/RUST_PYTHON_PARITY.md` — deferred (low priority, the parity tracker is transitional and the BO3 features are intentionally Rust-only per CLAUDE.md rule #21; updating the parity test (`test_rust_python_parity.py::test_initial_action_mask_parity`) to mask out actions `93/94/95` IS done and serves as the executable form of this divergence note).

## 13. End-to-end smoke

- [x] 13.1 BO3 smoke (`--generalist --curriculum-seed 123 --eval-seed 999 --timesteps 10000 --match-format bo3`) completed in 272s. Confirmed `matchup_grid_5000.json` and `matchup_grid_10000.json` written under the TB log dir. TB scalars verified: `pilot/match_win_rate=0`, `pilot/match_swept_rate ≈ 0.5–0.6` (untrained agent loses ~half via sweep), `pilot/concede_rate ≈ 0.4` (significant concede activity), `pilot/concede_tied_rate=1.0` (all concedes happen at 0-0 — exactly the "scared concede" pattern the −10 penalty deters). `final.meta.json` carries `match_format: bo3`, `digivolve_shaping: true` (BO3-implies-on default fired). Found and fixed a bug along the way: concede during `SelectPlayOrder` now correctly forfeits the match (awards opponent the games needed to reach 2-wins) instead of falling into the draw branch.
- [x] 13.2 Backward-compat smoke (`--match-format single --timesteps 5000`) completed in 239s. `win_rate: 1.0`, `mean_terminal_score: 1.0`, `draw_rate: 0.0`. Zero `pilot/match_*` / `pilot/concede_*` / `pilot/play_*` scalars emitted in single mode (spec compliance verified). No `matchup_grid_*.json` written.
- [x] 13.3 Self-play smoke (`--self-play --timesteps 5000 --match-format bo3`) completed in 171s. Run finished without LSTM-state errors. BO3 metrics emitted: `match_win_rate=1.0`, `match_sweep_rate=1.0` (the self-play P1-always-wins convention combined with quick sweep matches). `MatchEnv.reset_inner_only` fallback path (no `OpponentWrapper` present) exercised correctly.

## 14. Final validation

- [x] 14.1 `python -m pytest code/tests/rl` — 277 passed (10 new tests: concede primitive, play-order, dense calibration; 27 new tests: MatchEnv; 5 new tests: LSTM carry; 8 new tests: PyO3 binding; plus updates to existing tests for the new ±1.5 / −0.5 dense calibration and the Rust-only `93/94/95` mask divergence).
- [x] 14.2 `cargo test --manifest-path code/digimon-engine/Cargo.toml` — 161 lib + 10 concede + 9 play-order + 165 mask_and_tensor all pass. 3 pre-existing `cards_behavioral` failures (`bt24_008`, `ex9_024`, `st19_04`) are tracked separately and confirmed to fail on the pre-change baseline.
- [x] 14.3 `openspec validate add-bo3-match-training --strict` — passes.
