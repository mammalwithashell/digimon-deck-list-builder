## Why

RL pilots currently train one game per Gym episode, which is mismatched with how Digimon TCG is actually played in store tournaments — best-of-three with a 50-minute timer. Real play forces strategic decisions the current training never sees: knowing when to concede a doomed game to bank time for the decider, choosing play order when the prior game's loser picks, and recognizing matchup advantage at the match level (sweep 2-0) rather than just at the per-game level. Today an agent has no language for "give up game 2 fast because I won game 1 and want a fresh game 3." Match-level training closes that gap and produces metrics — per-matchup match win rates, sweep rates, concede rates — that line up with how human pilots evaluate decks.

## What Changes

- **BREAKING**: Default Gym episode for `pilot_training` shifts from one game to one best-of-three match. Escape hatch `--match-format single` preserves prior behavior.
- Add three new actions in the currently-unused `93-99` action-decoder range. `ACTION_SPACE_SIZE` stays at 2192; no observation- or action-space size break:
  - `93` CONCEDE_GAME — legal at every agent decision point
  - `94` PLAY_FIRST — legal only during `SelectPlayOrder`
  - `95` PLAY_SECOND — legal only during `SelectPlayOrder`
- Add a `Game::concede(player)` engine primitive that clears `pending_selection`, drains `effect_queue`, emits a `concede` event before `declare_winner`, and reports `win_reason = "concede"`. Always-legal at the engine layer; the action mask exposes it whenever the agent has a decision point.
- Add a `SelectPlayOrder` `pending_selection` variant injected by the new match wrapper between games 2 and 3 (and between games 1 and 2 of every match — the loser of the previous game picks).
- New `MatchEnv` Gym wrapper sits below `OpponentWrapper`. Owns BO3 state, samples deck pair once per match, holds same decks across all 3 games, accumulates step count across games, and stamps every per-game recording artifact with `match_id`, `game_index_in_match`, `match_score_before`, and `play_order_choice`. LSTM h-state carries across games within a match (no `opponent_fn.reset_state()` call at sub-episode game boundaries).
- Recording artifacts remain **one JSON per game**. New fields are additive: `match_id`, `game_index_in_match`, `match_score_before`, `play_order_choice`, and `outcome.win_reason` ∈ {`security_zero` | `deck_out` | `concede` | `step_limit`}.
- Reward shape redesigned for match format. Dense signals softened to support recovery-deck training without rewarding "clear all security and lose":
  - Dense: `+1.5` opp-security removed, `−0.5` own-security lost (asymmetric, was `±2.0`); `+0.1` digivolve, `+0.4` DNA digivolve (existing values, now **default ON**, was OFF); `−0.001` step penalty.
  - Per-game terminal: `±12.0` win/loss, plus up to `+3.0` fast-game bonus (win only, par 50 steps, zero at 150).
  - Per-match terminal: `±30.0` base; `+10.0` sweep bonus on 2-0; `+5.0` smart-concede bonus when a match is won and any game in it was conceded by the agent; `−10.0` scared-concede penalty when a match is lost 0-2 and any game in it was conceded; up to `+15.0` fast-match bonus (win only, par 150 steps, zero at 450); `−1.0` for a 1-1-1 draw at hard step-limit.
- Match-aware logging surface — new TensorBoard scalars in `WinRateCallback`: `pilot/match_win_rate`, `pilot/match_sweep_rate`, `pilot/match_swept_rate`, `pilot/match_total_steps_mean`, `pilot/games_per_match_mean`, `pilot/concede_rate`, `pilot/concede_lead_rate`, `pilot/concede_tied_rate`, `pilot/concede_down_rate`, `pilot/concede_correct_rate`, `pilot/concede_lucky_rate`, `pilot/play_first_rate`, `pilot/play_order_first_winrate`, per-archetype slices, per-matchup grids.
- Per-match matchup-grid JSON sidecar at `runs/<id>/matchup_grid_<step>.json`, generated at eval time alongside the existing eval sidecar. Surfaced by `digimon-training-mcp` via `run_summary` / `run_metric`.
- Per-game soft step limit unchanged (≈300 steps via existing `force_step_limit_winner`). New per-match hard limit: 900 steps; on overrun, the in-progress game is force-finished and the match resolves on game-count (tie → 1-1-1 draw).
- `--self-play` mode flips to BO3 with no special handling. Both sides share the policy; match-as-episode semantics carry the LSTM state cleanly across games for both.
- Default checkpoint compatibility preserved at the action-space level (no size change), but **policy semantics change** because actions 93–95 are unmasked. Existing checkpoints can be resumed; behavior on the new actions will be near-random until additional training. The training runbook will document this and prescribe a fine-tune step rather than a hard checkpoint break.

## Capabilities

### New Capabilities

- `bo3-match-training`: Best-of-three match format for RL pilot training. Covers the match-as-episode Gym semantics, deck-pair persistence across games, the loser-picks play-order selection, the concede action and its always-legal engine primitive, the match-tier reward shape (per-game terminal, per-match terminal, sweep bonus, smart/scared concede bonuses, fast-match bonus), match-aware recording metadata, and the match-level TensorBoard / matchup-grid logging surface.

### Modified Capabilities

<!-- No modifications. Generalist pilot pretraining (generalist-pilot-pretraining) operates inside the match-as-episode wrapper; the existing requirement "each episode reset injects a sampled deck1 and deck2" continues to hold because the BO3 wrapper samples decks once per match (= once per episode). No requirement-level change. -->

## Impact

- **Engine (Rust)**: `code/digimon-engine/src/game.rs` (new `concede` method, `concede` event emission); `code/digimon-engine/src/selection.rs` (new `SelectPlayOrder` `PendingSelection` variant); `code/digimon-engine/src/action/decoder.rs` (decode 93 → concede, 94/95 → play-order); `code/digimon-engine/src/action/mask.rs` (mask rules for the three new IDs); `code/digimon-engine/tests/concede_primitive.rs` and `code/digimon-engine/tests/select_play_order.rs` (new integration tests).
- **PyO3 bindings**: `code/digimon-engine-py/src/lib.rs` — expose `concede(player_id)`, `request_play_order_selection(loser_id)`, and surface `win_reason` in the terminal outcome dict.
- **Gym env / wrapper**: `code/digimon_gym/digimon_gym.py` — reward-shape constants updated; new BO3-aware reward path; default `digivolve_shaping=True` for `--match-format bo3`. New file `code/digimon_gym/agents/match_env.py` — `MatchEnv` wrapper.
- **Training entry**: `code/digimon_gym/agents/pilot_training.py` — new `--match-format {bo3, single}` CLI arg (default `bo3`); `make_env()` wires `MatchEnv` into the chain below `OpponentWrapper`; `WinRateCallback` gains match-tier metrics; eval loop iterates matches rather than games; eval sidecar gains match-format and matchup-grid path fields.
- **Training config**: `code/digimon_gym/agents/training_config.py` — new fields `match_format: str = "bo3"` and the new reward-shape dials.
- **Action / Tensor docs**: `docs/ACTION_SPEC.md` updated to claim 93–95 from the unused range; `docs/TRAINING_RUNBOOK.md` adds a §13 "Best-of-three match training" with the metric catalog and the checkpoint-compatibility caveat.
- **MCP**: `code/digimon-training-mcp/` surfaces matchup-grid sidecars via `run_summary`. No schema additions needed beyond reading the new JSON.
- **Tests**: New `code/tests/rl/test_match_env.py` (BO3 state machine, deck persistence, LSTM carry, fast-bonus math); `code/tests/rl/test_concede_action.py` (action mask, end-to-end concede flow, recording `win_reason`); `code/tests/rl/test_match_rewards.py` (every scenario in the design's payoff table verified numerically); engine-level tests as above.
- **Working rules**: New rule in `CLAUDE.md` — "Match-format training (`--match-format bo3`) makes the Gym episode equal a best-of-three match; deck pair, LSTM h-state, and step counter persist across games-within-match. The MatchEnv wrapper sits below OpponentWrapper. Single-game behavior is available via `--match-format single`."
- **No data migration**. Existing recordings without `match_id` continue to load; replay tools treat absent match metadata as a single-game artifact (which it was).
