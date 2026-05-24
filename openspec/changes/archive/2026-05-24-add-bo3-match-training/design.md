## Context

RL pilot training today maps one game to one Gym episode. Recent measurements against the greedy opponent show games run 26.4–77.6 steps (mean 42.8, median 38.8). Real human play uses best-of-three matches under a 50-minute timer, which introduces strategic decisions the agent never sees: conceding doomed games to bank time, choosing play order after losing the previous game, and reading matchup advantage at the match level (2-0 sweep) rather than per-game.

Existing scaffolding the design leverages:

- The engine action decoder reserves `93-99` as "Unused" (per `docs/ACTION_SPEC.md`). Three of those seven slots are available for new actions without bumping `ACTION_SPACE_SIZE`, avoiding the kind of break that S1.3 just forced.
- The engine has `force_step_limit_winner()` on `HeadlessRunner` for game-level step-limit resolution.
- The `OpponentWrapper` already resets LSTM state at episode boundaries via `opponent_fn.reset_state()`. The current generalist sampler runs at the same layer (`GeneralistDeckPoolWrapper`).
- Recording artifacts are emitted per-game by the runner with `outcome.winner_id`, `outcome.terminated`, etc. — adding new metadata fields is additive.
- Digivolve reward shaping already exists with cumulative `n_digivolutions` / `n_dna_digivolutions` counters surfaced via `get_rl_state()` (per `docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md`). It is currently default OFF.

Stakeholders: the project owner (RL training direction), pilot-training users (CLI surface), the engine source-of-truth (Rust engine semantics), and the training-metrics MCP (matchup-grid sidecar consumer).

## Goals / Non-Goals

**Goals:**

- Make best-of-three the default Gym episode for `pilot_training`, with an escape hatch.
- Give the agent a concede action whose mask is `1` at every decision point.
- Give the agent a play-order choice action when it is the loser of the prior game in a match.
- Shape rewards so the agent learns to prefer match wins over game wins, prefer sweeps, smart-concede when ahead, and avoid scoping when behind or at 0-0.
- Preserve LSTM h-state across games within a match so the recurrent policy can carry inter-game read.
- Preserve the existing per-game recording artifact format; extend with match-aware metadata fields.
- Surface match-level metrics (sweep rate, concede rate by score state, per-archetype and per-matchup match-win rates) in TensorBoard and as a JSON matchup-grid sidecar.

**Non-Goals:**

- Sideboarding between games. Same deck across all 3 games.
- Modifying the observation tensor. Match score, game index, and concede availability are not encoded into the policy input; they are reward and mask state only.
- Bumping `ACTION_SPACE_SIZE` past 2192. The new actions occupy the existing unused range.
- Migrating any of this back to the Python legacy engine. Rust is the source of truth.
- Mid-selection concede semantics that require complex inner-state rewinds. Concede during a pending selection drains the queue and short-circuits — it does NOT attempt to resolve queued effects first.
- Best-of-N for N ≠ 3. The wrapper is BO3-specific; generalizing is a future change.
- Eval-time interactivity (a human picking play order). Eval runs the policy's choice; opponent decks pick their order by a deterministic rule (always-first, mirroring greedy).

## Decisions

### D1. Match as Gym episode, not as wrapper-above-episodes

**Choice**: One Gym episode = one BO3 match. The three games are stitched inside a single `MatchEnv` wrapper. `terminated=True` only at match end.

**Alternatives**:

- *Match-as-wrapper above single-game episodes* — `MatchEnv` would call `inner.reset()` between games and synthesize a match-level reward at the third game's end. Each game would still be its own episode from SB3's perspective.

**Rationale**: The concede story requires the value function to see future games as part of the same return calculation. In the wrapper-above-episodes shape, game 2's value function cannot see game 3 (separate episodes), so "concede game 2 because game 3 is coming" cannot be learned through the normal advantage estimate — it would have to be hand-shaped via wrapper synthetic rewards, exactly the kind of approximation that tends to wobble. Match-as-episode lets the standard PPO/GAE machinery do the credit assignment.

**Trade-off**: Episode length triples. Median ~120 steps, long matches ~450, hard cap 900. SB3's `n_steps=2048` default still fits 4–17 matches per rollout, which is sufficient. Recurrent credit across 600+ steps is harder than across 60 — accepted, this is fundamental to the design.

### D2. Action ranges 93 / 94 / 95 in the existing "Unused 93-99" slot

**Choice**: `93` = CONCEDE_GAME (always legal at agent decision points), `94` = PLAY_FIRST, `95` = PLAY_SECOND (legal only during a new `SelectPlayOrder` phase).

**Alternatives**:

- Reuse `62` (PASS/DECLINE) for concede — already heavily overloaded across phases. Adds yet another phase-aware interpretation.
- Reuse `SelectEffectChoice` indices `1000`/`1001` for play-order — works but is opaque in the action mask.
- Bump `ACTION_SPACE_SIZE` to 2195 — forces another retrain break on the heels of S1.3.

**Rationale**: The `93-99` range is already documented as "Unused" with seven free slots. Using three of them is the lowest-disruption option. `94`/`95` as named actions are clearer in the mask than reusing the generic effect-choice slots.

### D3. Concede primitive lives in the engine, not the wrapper

**Choice**: Add `Game::concede(player_id)` in the Rust engine. It clears `pending_selection`, drains `effect_queue`, emits a `concede` event (mirroring rule #16's surrender-event ordering), then calls `declare_winner(opponent)`. The terminal outcome reports `win_reason = "concede"`.

**Alternatives**:

- Wrapper-level "fake concede" — `MatchEnv` could detect action 93 and synthesize a game-over without touching the engine. But this leaves the engine's selection/effect state inconsistent and breaks recordings (no event log, no audit trail of the concede).

**Rationale**: Concede must be a first-class engine event for recordings, replay, and analysis. Wrapper-level fakery would silently corrupt downstream tooling. Engine-level primitive is small (one function on `Game`, one event variant, one new `win_reason`).

**Out of scope for engine**: The engine does NOT know about matches. It only knows about games. `MatchEnv` owns all match state.

### D4. `SelectPlayOrder` phase is injected by the wrapper, not engine-driven

**Choice**: `MatchEnv` calls a new `runner.request_play_order_selection(loser_id)` after each game (except game 3) to enter a `SelectPlayOrder` phase. The engine surfaces actions 94/95 as legal during this phase. When the player picks, the engine records the choice and exits the phase; `MatchEnv` then calls the next `Game::new_with_first_player(...)` for the next game.

**Alternatives**:

- Add a `Match` struct in the engine that owns the BO3 loop — couples engine to a single-game-format and bleeds match concerns into the engine.

**Rationale**: Keeps the engine match-agnostic. Future formats (BO5, single-game, exhibition) just don't call the new phase. The engine grows exactly two new APIs: `concede` and `request_play_order_selection`.

### D5. Same deck pair across all 3 games of a match

**Choice**: `MatchEnv.reset()` samples a deck pair once per match. All three games use those decks. Play order alternates per the BO3 rules below.

**Alternatives**:

- Re-sample decks per game — would model a different thing than BO3 tournaments.

**Rationale**: Real BO3 = same deck. The whole "I learned the matchup in game 1" effect is what makes BO3 strategically distinct from "three independent games."

### D6. Play-order rules

**Choice**:

- Game 1: random first player (coin flip, seeded).
- Game 2 onward: loser of the previous game chooses via the new selection phase. Action 94 = "I go first," action 95 = "I go second."

**Alternatives**:

- Always random — simpler, but discards the strategic dimension of choosing.

**Rationale**: Loser-picks is the actual tournament rule. Real strategic depth (e.g., "I have explosive starts, I want to go second to draw 6").

### D7. LSTM state carries across games within a match

**Choice**: `OpponentWrapper` only calls `opponent_fn.reset_state()` at match boundaries (true Gym `reset()`). Within a match, game boundaries do NOT reset hidden state.

**Alternatives**:

- Reset h-state at every game boundary — cleaner credit assignment, loses inter-game memory.

**Rationale**: The user's framing — "I learned in game 1 that this matchup is unwinnable so I'll concede game 2" — requires inter-game memory. Carrying h-state across games is closer to how human pilots think and gives the recurrent policy a chance to learn it.

### D8. Per-game dense-reward trackers reset at game boundary

**Choice**: `_prev_p1_security`, `_prev_p2_security`, `_prev_p1_digivolutions`, `_prev_p1_dna_digivolutions` reset to `None` at each game boundary (NOT only at match boundary).

**Rationale**: A new game has fresh security counts. Carrying the cross-game deltas would generate a phantom `−10` security signal at game 2 start (because last game ended with security counts reset to 5 each side, but the carried `_prev_*` was at game 1's ending state). The dense signal must be game-local.

### D9. Reward calibration

The full menu (all magnitudes in reward units):

| Signal | Value | Notes |
|---|---:|---|
| Step penalty | −0.001 | per step |
| Security remove (opp) | +1.5 | per event |
| Security lose (own) | −0.5 | per event, asymmetric |
| Security gained via effect | 0 | not rewarded |
| Digivolve (regular) | +0.1 | per event, agent-only, default ON in BO3 |
| Digivolve (DNA total) | +0.4 | per event (stacks: +0.1 base + +0.3 bonus) |
| Game terminal win | +12.0 | |
| Game terminal loss | −12.0 | |
| Fast-game bonus max | +3.0 | `max(0, (150 − game_steps)/150) × 3`, win only |
| Match terminal win | +30.0 | |
| Match terminal loss | −30.0 | |
| Match draw (1-1-1) | −1.0 | only at hard step-limit truncation |
| Sweep bonus | +10.0 | match win 2-0 only |
| Smart-concede bonus | +5.0 | match won AND ≥1 game in match conceded by agent (flat) |
| Scared-concede penalty | −10.0 | match lost 0-2 AND ≥1 game in match conceded by agent |
| Fast-match bonus max | +15.0 | `max(0, (450 − match_steps)/450) × 15`, win only |

**Calibration rationale**:

- Per-event security at `1.5` keeps the dense signal meaningful but capped: max cumulative per game is `+7.5` (clear all 5 opp), vs game-terminal `+12` — game-win is `1.6×` larger than max in-game dense. This explicitly addresses the recovery-deck attack surface where an opponent could heal back security and bait an agent into thinking it was winning.
- Asymmetric own-security loss at `−0.5` (vs `−1.5` symmetric) avoids over-punishing the agent for the opponent's offense, since security loss is partially out of the agent's control.
- Game terminal `±12` > cumulative game dense (`+7.5` to `−2.5`).
- Match terminal `±30` > cumulative match dense (~`±20` max).
- Sweep bonus `+10` is asymmetric (no `−10` for being swept). User-literal interpretation of "reward winning 2-0."
- Smart-concede bonus is flat `+5` — does NOT condition on "agent was ahead at concede time." We watch `pilot/concede_lucky_rate` to see if this gets exploited and tighten later if so.
- Scared-concede penalty `−10` makes 0-2-via-concede strictly worse than 0-2 honest, deterring "scared scoop at 0-0 or 0-1."
- Fast bonuses are win-only. Symmetric fast-loss bonus would encourage tanking when losing — exactly the failure mode we want to suppress.

**Scenario sanity** (typical-case numbers):

```
Standard 2-1 win (G1+G2+G3 = 200 steps):           +63.7
Smart-concede 2-1 win (G1+concede+G3 = 121s):      +69.1   (+5.4 vs play-out)
Dominant 2-0 sweep (50s + 50s = 100s):             +88.6
Lose 1-2 honestly (220s):                          −31.7
Lose 1-2 with G2 concede (141s):                   −34.1
Lose 0-2 honestly:                                 −49.2
Lose 0-2 with scared concede (any game):           −61.6
Concede G1 at 0-0 then win 2-1 ("lucky"):          +69.1
Lose G1 honestly then win 2-1:                     +64.5
                                                     ↑ lucky-concede pays +4.6
                                                       only if you actually
                                                       win games 2 and 3
```

Crossover analysis: the agent must be ~70% confident of winning the next two games for "concede at 0-0" to be EV-positive. High enough that we don't expect routine exploitation.

### D10. Match-step soft and hard limits

**Choice**: Keep the existing per-game soft limit at `max_turns * 10` (~300 steps). On overrun, `force_step_limit_winner()` resolves the game. Add a per-match hard limit at 900 steps. On overrun, the in-progress game is force-resolved, then the match is decided on game-count (2-x wins, tie → 1-1-1 draw with `−1` terminal).

**Rationale**: Matches that take more than 3 × 300 steps are pathological. Hard limit prevents indefinite rollouts in the rare cases where every game grinds.

### D11. Recording metadata: per-game artifacts, extra fields

**Choice**: Keep one JSON per game (existing tooling unchanged). Add five new fields to every artifact:

- `match_id` — UUIDv4 shared by all games in a match
- `game_index_in_match` — `0`, `1`, or `2`
- `match_score_before` — `{ p1_wins: int, p2_wins: int }` at game start
- `play_order_choice` — `null` for game 1; otherwise `{ chooser: player_id, picked: "first" | "second" }`
- `outcome.win_reason` — `"security_zero" | "deck_out" | "concede" | "step_limit"`

**Alternative**: One artifact per match with games nested.

**Rationale**: Per-game preserves existing replay tooling, MCP queries, and the AI-pipeline test harness. Match-level forensics ("did concede here pay off?") works by joining game artifacts on `match_id`.

### D12. CLI surface

**Choice**: New flag `--match-format {bo3, single}` with default `bo3`. `bo3` enables the MatchEnv wrapper + concede mask + new metrics. `single` retains the prior single-game behavior (concede masked off, no play-order selection, single-game reward shape).

Default `digivolve_shaping=True` only when `match_format=bo3`. `--match-format single` retains the existing `False` default for backward compatibility.

### D13. Checkpoint compatibility

**Choice**: Existing checkpoints can be loaded — `ACTION_SPACE_SIZE` does not change. But actions `93–95` are unmasked in BO3 mode, so the policy's distribution over those actions is whatever its softmax happens to produce on never-seen indices. Expect near-random behavior on the new actions until additional training.

The training runbook will document this with a recommended fine-tune procedure: load checkpoint, run for ~100k–500k timesteps in `--match-format bo3` to learn the new action semantics, evaluate, then continue full training.

**Alternative**: Hard-break the checkpoint compatibility — bump action space, force retrain.

**Rationale**: S1.3 just forced one retrain break. Avoiding a second back-to-back break is worth the soft "near-random on new actions" transition. The user can choose to retrain from scratch if preferred.

## Risks / Trade-offs

- **[Risk]** Conceding behavior is rare in early training, so the smart-concede bonus produces no gradient until the policy stumbles into a concede. → *Mitigation*: action 93 is always-legal at decision points, so random exploration will sample it on the order of `1/legal_action_count` per step. Across millions of steps, the policy will sample concedes and the gradient will sort it out. Optional later: add a tiny exploration bonus on first-time-per-episode action sampling, but we don't need it yet.

- **[Risk]** LSTM credit assignment across 600+ steps degrades. Match-as-episode triples episode length. → *Mitigation*: PPO with GAE handles long episodes via `lambda < 1.0` discounting. We'll watch `mean_eval_episode_length` and the value-function loss; if they diverge from baseline, we have options (recurrent state truncation at game boundary while still treating episode as match for reward, larger `n_steps`, smaller `gamma`).

- **[Risk]** "Lucky concede" path (concede G1 at 0-0, win 2 and 3) gets the same `+5` bonus as the "smart concede" path (concede G2 from 1-0, win G3). → *Mitigation*: log `pilot/concede_lucky_rate` separately and watch it. If exploitation appears, tighten the bonus condition to "agent was ahead at concede time." Easy follow-up change, no engine work.

- **[Risk]** `MatchEnv` and `OpponentWrapper` interaction with `--self-play` mode is subtly different — both sides share a policy, both sides should see the same match semantics. → *Mitigation*: opponent_fn in self-play already handles two-sided LSTM h-state via `policy_states` dict. Match-as-episode just means h-state isn't reset until match end. Verified by `test_match_env_self_play.py` in the test plan.

- **[Risk]** Step-limit truncation at 900 produces a 1-1-1 draw with `−1` terminal. If this happens often, the draw rate inflates evaluation noise. → *Mitigation*: Empirically rare — the per-game `force_step_limit_winner` resolves individual games before the match can stall. The hard limit is a safety net, not an expected outcome. We'll watch the draw rate.

- **[Risk]** Eval cost roughly 2.5× (matches take ~120s instead of ~50s on average). `--eval-episodes 20` was 20 games; now it's 20 matches ≈ 50 games. → *Mitigation*: document in the runbook. Suggest reducing `--eval-episodes` for match-format runs or increasing `--eval-freq`. No code change, just guidance.

- **[Risk]** Existing checkpoints have undefined behavior on actions 93–95 because those actions were never masked legal during training. → *Mitigation*: documented (D13). Provide a fine-tune procedure. Users who object can simply pass `--match-format single` to continue training in legacy mode.

- **[Risk]** Recording readers that ignore unknown fields will continue to work; readers that strictly validate schemas will break on the new fields. → *Mitigation*: The five new fields are documented in the spec and the runbook. The replay tooling treats absent `match_id` as a single-game artifact (default `match_id = null`, `game_index_in_match = 0`).

## Migration Plan

1. **Engine layer** — Land `Game::concede(player_id)`, the new `concede` event, the `win_reason="concede"` outcome path, and the `SelectPlayOrder` phase + selection variant. Tests in `code/digimon-engine/tests/`. Independently mergeable.
2. **PyO3 bindings** — Expose the new engine APIs via `RustHeadlessGame`. Rebuild via `maturin develop`. Add a Python smoke test that calls `concede(2)` and asserts `winner_id == 1`, `win_reason == "concede"`.
3. **Action decoder + mask** — Decode IDs 93/94/95. Mask 93 = 1 at every decision point. Mask 94/95 = 1 only during `SelectPlayOrder`. Update `docs/ACTION_SPEC.md`.
4. **`MatchEnv` wrapper** — New file `code/digimon_gym/agents/match_env.py`. Owns BO3 state machine, deck-pair persistence, play-order injection between games, per-game dense-tracker resets, match-level reward synthesis. Comprehensive unit tests cover every scenario in the design's payoff table.
5. **Reward integration** — Update `_compute_reward()` in `DigimonEnv` for the new dense calibration (`+1.5 / −0.5`, default digivolve ON). Match-level reward is added by `MatchEnv` at match-terminal, not by `DigimonEnv`. `DigimonEnv` doesn't know about matches.
6. **`pilot_training.py` wiring** — New `--match-format` CLI arg. `make_env()` chain: `DigimonEnv → OpponentWrapper → MatchEnv → GeneralistDeckPoolWrapper → ActionMasker`. `WinRateCallback` gains match metrics. Eval loop iterates matches.
7. **Recordings + sidecars** — Per-game artifact metadata stamping by `MatchEnv`. Matchup-grid sidecar JSON written by `WinRateCallback` at each eval step.
8. **Docs** — `docs/TRAINING_RUNBOOK.md` gains §13. `docs/ACTION_SPEC.md` claims 93/94/95. `CLAUDE.md` gains the new working rule.

**Rollback**: Every step is independently committable. The final user-visible break is the default flip in step 6; until then, the wrapper is opt-in via an undocumented env var. The flip itself is a one-line change in `make_env()` defaults.

## Open Questions

- **Self-play LSTM state correctness** — need to verify with a focused test that both sides' h-states carry across games within a match independently. Will land as a test in step 4, surface during implementation if there's an issue.
- **`--eval-episodes` default for match format** — keep 20 (= 50 games), or drop to 10 (= 25 games) to keep eval cost flat? Defer to first run; no code work either way, just a docstring decision.
- **Whether `digivolve_shaping=True` default should also extend to `--match-format single`** — current decision is no (single keeps the existing OFF default for back-compat). Could flip later if no one complains.
