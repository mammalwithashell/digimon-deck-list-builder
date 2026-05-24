# bo3-match-training Specification

## Purpose
Best-of-three match format for RL pilot training. Covers the match-as-episode Gym semantics, deck-pair persistence across games, the loser-picks play-order selection, the concede action and its always-legal engine primitive, the match-tier reward shape (per-game terminal, per-match terminal, sweep bonus, smart/scared concede bonuses, fast-match bonus), match-aware recording metadata, and the match-level TensorBoard / matchup-grid logging surface.

## Requirements

### Requirement: BO3 match equals one Gym episode

When `--match-format bo3` is active, one Gym episode SHALL correspond to one best-of-three match consisting of up to three games. `terminated=True` SHALL be returned only when the match resolves (a player has won 2 games, or the per-match hard step limit is reached). Game-end transitions inside the match SHALL NOT terminate the episode.

#### Scenario: Match terminates after 2-0 sweep

- **WHEN** an agent wins game 1 and wins game 2 of a match
- **THEN** the Gym step returns `terminated=True` with the match-terminal reward at the step that ends game 2
- **AND** no game 3 is played

#### Scenario: Match terminates after game 3

- **WHEN** game 1 and game 2 finish 1-1 and the agent plays game 3 to completion
- **THEN** `terminated=True` is returned only at the step ending game 3

#### Scenario: Inter-game step does not terminate

- **WHEN** game 1 finishes with the match score at 1-0
- **THEN** the next Gym step continues the same episode (no `terminated=True`)
- **AND** the wrapper either enters the `SelectPlayOrder` phase or begins game 2 at the agent's next decision point

### Requirement: Single-game escape hatch via `--match-format single`

The pilot training CLI SHALL accept a `--match-format` flag with values `bo3` (default) and `single`. When `single` is selected, the Gym episode SHALL correspond to one game and the BO3 wrapper SHALL be omitted from the env chain. Concede, play-order, and match-level rewards SHALL NOT apply in `single` mode.

#### Scenario: Default is bo3

- **WHEN** pilot training is started without `--match-format`
- **THEN** match format is `bo3`

#### Scenario: Single-game escape hatch is selectable

- **WHEN** pilot training is started with `--match-format single`
- **THEN** every Gym episode resolves at the first game's end
- **AND** the action mask reports actions 93, 94, and 95 as illegal at every step

### Requirement: Deck pair persists across all games of a match

The match wrapper SHALL sample a `(deck1, deck2)` pair at most once per match. The same pair SHALL be used for game 1, game 2, and (if played) game 3 of that match. No sideboarding SHALL occur between games.

#### Scenario: Same decks across the match

- **WHEN** a match runs to game 3
- **THEN** the deck used by each player in games 2 and 3 matches that player's deck in game 1 byte-for-byte

#### Scenario: Resample on next match

- **WHEN** one match ends and the next `env.reset()` is called
- **THEN** a fresh `(deck1, deck2)` pair is sampled

### Requirement: Concede action 93 always legal at agent decision points

Action ID `93` (`CONCEDE_GAME`) SHALL be reported legal by `get_action_mask()` whenever the agent has any decision point — including the mulligan phase, the main phase, the breeding phase, block / counter / alliance timing windows, end-of-turn-action windows, and any pending selection phase. Selecting `93` SHALL invoke the engine's `Game::concede(player)` primitive, which clears `pending_selection`, drains `effect_queue`, emits a `concede` event, calls `declare_winner(opponent)`, and reports `win_reason = "concede"` on the terminal outcome.

#### Scenario: Concede legal in main phase

- **WHEN** the agent has the turn in the main phase with no pending selection
- **THEN** action 93 is reported legal

#### Scenario: Concede legal during selection

- **WHEN** a pending selection is open and the agent is the chooser
- **THEN** action 93 is reported legal in addition to the selection's normal valid indices

#### Scenario: Concede ends the game with the correct win reason

- **WHEN** the agent submits action 93 during its turn
- **THEN** the next state has `game_over=True`, `winner_id=opponent`, and the terminal outcome reports `win_reason = "concede"`
- **AND** the recording artifact's `outcome.win_reason` field equals `"concede"`

### Requirement: Loser-picks play order between games

After game 1 and game 2 of every match (and only those points), the match wrapper SHALL drive the engine into a `SelectPlayOrder` phase where the loser of the previous game is the chooser. Actions `94` (PLAY_FIRST) and `95` (PLAY_SECOND) SHALL be reported legal during this phase and ONLY during this phase.

#### Scenario: Loser of game 1 chooses play order for game 2

- **WHEN** game 1 ends with player 1 winning
- **THEN** the next decision point is a `SelectPlayOrder` phase with the chooser set to player 2
- **AND** actions 94 and 95 are reported legal for player 2

#### Scenario: Play-order actions illegal outside the phase

- **WHEN** the engine is not in `SelectPlayOrder`
- **THEN** actions 94 and 95 are reported illegal at every decision point

#### Scenario: Game 1 uses random play order

- **WHEN** a new match starts at game 1
- **THEN** the first player is selected from a seeded coin flip
- **AND** no `SelectPlayOrder` phase is entered before game 1

### Requirement: LSTM hidden state carries across games within a match

Recurrent policy hidden state SHALL be reset only at true Gym episode boundaries (match boundaries). The `OpponentWrapper`'s `opponent_fn.reset_state()` SHALL NOT be called between games of a match. The agent policy's recurrent state SHALL be preserved across game transitions inside a match.

#### Scenario: LSTM state carries between games

- **WHEN** game 1 ends with non-zero hidden state and game 2 begins
- **THEN** the policy's hidden state at the first step of game 2 equals the hidden state at the final step of game 1
- **AND** the opponent policy's hidden state likewise carries

#### Scenario: LSTM state resets at match boundary

- **WHEN** a match ends and `env.reset()` is called
- **THEN** the policy's hidden state and the opponent policy's hidden state are both reset to their initial values

### Requirement: Dense reward signals — security and digivolve

The per-step dense reward SHALL apply the following magnitudes:

- `+1.5` per opponent security card removed in the step.
- `−0.5` per agent security card lost in the step (asymmetric).
- `0` for agent security gained via effect (not rewarded).
- `+0.1` per agent regular digivolution in the step.
- `+0.3` additional per agent DNA digivolution in the step (DNA total per event = `+0.4` because DNA increments both counters).
- `−0.001` per step.

Digivolve shaping SHALL default `True` in `--match-format bo3`. The dense trackers (`_prev_p1_security`, `_prev_p2_security`, `_prev_p1_digivolutions`, `_prev_p1_dna_digivolutions`) SHALL reset to `None` at every game boundary inside a match, not only at match boundary.

#### Scenario: Opp security removal rewards +1.5

- **WHEN** the agent's action removes one opponent security card and no other state changes count toward the reward
- **THEN** the step reward includes `+1.5` from the security-delta signal

#### Scenario: Own security loss penalizes −0.5

- **WHEN** the agent loses one own security card and no other state changes count toward the reward
- **THEN** the step reward includes `−0.5` from the security-delta signal

#### Scenario: DNA digivolve rewards +0.4 in total

- **WHEN** the agent executes one DNA digivolve in a step in bo3 mode and no other state changes count toward the reward
- **THEN** the step reward includes `+0.4` from the digivolve signal (`+0.1` regular + `+0.3` DNA bonus)

#### Scenario: Dense trackers reset at game boundary inside a match

- **WHEN** game 1 ends with `p1_security = 0` and game 2 begins with `p1_security = 5`
- **THEN** the first step of game 2 does NOT emit a `+10` phantom security signal
- **AND** the `_prev_p1_security` tracker is `None` at game 2 start

### Requirement: Per-game terminal reward

When a game inside a match ends, the wrapper SHALL emit a per-game terminal reward of `+12.0` (game won by agent) or `−12.0` (game lost by agent). A fast-game bonus of `max(0, (150 − game_step_count) / 150) × 3.0` SHALL be added on a game win and never on a game loss. The bonus SHALL be 0 when `game_step_count ≥ 150`.

#### Scenario: Fast game win pays the maximum bonus

- **WHEN** the agent wins a game in 0 steps (theoretical minimum)
- **THEN** the per-game terminal reward equals `+12 + 3 = +15`

#### Scenario: Slow game win pays no bonus

- **WHEN** the agent wins a game in 200 steps
- **THEN** the per-game terminal reward equals `+12` exactly

#### Scenario: Game loss never gets fast bonus

- **WHEN** the agent loses a game in 30 steps
- **THEN** the per-game terminal reward equals `−12` exactly (no bonus reduction)

### Requirement: Per-match terminal reward

At match end the wrapper SHALL emit a match-terminal reward composed of:

- `+30.0` for a match win (2 game wins), or `−30.0` for a match loss (0 or 1 game wins).
- `+10.0` sweep bonus when the match was won 2-0.
- `+5.0` smart-concede bonus when the match was won AND at least one game in the match was lost by agent concede (action 93).
- `−10.0` scared-concede penalty when the match was lost 0-2 AND at least one game in the match was lost by agent concede.
- `−1.0` (overriding the win/loss base) for a 1-1-1 draw caused by per-match hard step-limit truncation.
- `max(0, (450 − total_match_steps) / 450) × 15.0` fast-match bonus when the match is won. Bonus SHALL be `0` on losses and draws.

Bonuses SHALL stack additively. A 2-0 sweep that involved a conceded game (rare path: agent wins G1, opponent concedes G2) SHALL pay both the sweep bonus and the smart-concede bonus.

#### Scenario: 2-0 sweep pays base, sweep, and fast-match

- **WHEN** the agent wins a match 2-0 in 100 total steps with no concede
- **THEN** the match-terminal reward equals `+30 + 10 + (450 − 100)/450 × 15 = +51.7`

#### Scenario: Smart-concede 2-1 win pays bonus

- **WHEN** the agent wins game 1, concedes game 2 via action 93, wins game 3, total match steps 121
- **THEN** the match-terminal reward equals `+30 + 5 + (450 − 121)/450 × 15 ≈ +45.97`
- **AND** the sweep bonus is NOT paid (match was 2-1, not 2-0)

#### Scenario: Scared concede in 0-2 loss

- **WHEN** the agent concedes game 1 via action 93, loses game 2 normally, match ends 0-2
- **THEN** the match-terminal reward equals `−30 + (−10) = −40`
- **AND** the fast-match bonus is `0`

#### Scenario: 0-2 honest loss does not trigger scared concede

- **WHEN** the agent loses game 1 and game 2 without ever submitting action 93
- **THEN** the match-terminal reward equals `−30` exactly

#### Scenario: 1-2 loss with concede pays no extra penalty

- **WHEN** the agent wins game 1, concedes game 2, loses game 3, match ends 1-2
- **THEN** the match-terminal reward equals `−30` exactly (scared-concede penalty does NOT apply because the match was not 0-2)

### Requirement: Per-match hard step limit

The match wrapper SHALL track a cumulative match step counter across all games of the match. If the counter exceeds 900, the in-progress game SHALL be force-resolved via the existing per-game step-limit mechanism, then the match SHALL resolve on game-count: the player with more game wins wins the match; if tied (1-1-0 or 1-1-1), the match is a draw and the match-terminal reward is `−1`.

#### Scenario: Match step limit truncates final game

- **WHEN** the match reaches its 901st step in the middle of game 3 at 1-1
- **THEN** the in-progress game is force-resolved with a winner (via `force_step_limit_winner`)
- **AND** the match terminates with `terminated=True` at that step

#### Scenario: Per-game step limit unchanged

- **WHEN** a single game inside a match reaches the existing per-game step-limit soft cap
- **THEN** the existing `force_step_limit_winner` resolves that game and the match continues at the next game

### Requirement: Per-game recording artifact carries match metadata

Every per-game recording artifact emitted while `--match-format bo3` is active SHALL include the following metadata fields:

- `match_id` — UUID identifying the match, shared by all games in that match.
- `game_index_in_match` — integer `0`, `1`, or `2`.
- `match_score_before` — object with `p1_wins` and `p2_wins` reflecting the match score at game start.
- `play_order_choice` — `null` for game 1; for games 2 and 3, an object `{ chooser: player_id, picked: "first" | "second" }` describing the resolved play-order selection.
- `outcome.win_reason` — string drawn from `{ "security_zero", "deck_out", "concede", "step_limit" }`.

Recording artifacts emitted while `--match-format single` is active SHALL set `match_id = null`, `game_index_in_match = 0`, `match_score_before = { p1_wins: 0, p2_wins: 0 }`, `play_order_choice = null`, and an appropriate `outcome.win_reason`.

#### Scenario: Game 2 recording stamps match metadata

- **WHEN** game 2 of a bo3 match finishes
- **THEN** its recording artifact has the same `match_id` as game 1's artifact
- **AND** `game_index_in_match = 1`
- **AND** `match_score_before.p1_wins + match_score_before.p2_wins = 1`

#### Scenario: Single-mode artifact has null match metadata

- **WHEN** a game finishes under `--match-format single`
- **THEN** its recording artifact has `match_id = null` and `game_index_in_match = 0`

### Requirement: Match-aware logging metrics

`WinRateCallback` SHALL emit the following scalars to TensorBoard during evaluation when `--match-format bo3` is active:

- `pilot/match_win_rate` — fraction of eval matches won.
- `pilot/match_sweep_rate` — fraction of match wins that were 2-0.
- `pilot/match_swept_rate` — fraction of match losses that were 0-2.
- `pilot/match_total_steps_mean` — mean total steps across full matches.
- `pilot/games_per_match_mean` — mean number of games played per match (2.0 to 3.0).
- `pilot/concede_rate` — fraction of agent game-losses that occurred via action 93.
- `pilot/concede_lead_rate` — concede rate conditioned on agent being ahead in the match at concede time.
- `pilot/concede_tied_rate` — concede rate conditioned on the match being tied (0-0 only) at concede time.
- `pilot/concede_down_rate` — concede rate conditioned on agent being behind in the match at concede time.
- `pilot/concede_correct_rate` — fraction of agent concedes that preceded a match win.
- `pilot/concede_lucky_rate` — fraction of agent concedes performed while NOT ahead that preceded a match win.
- `pilot/play_first_rate` — fraction of `SelectPlayOrder` decisions in which the agent picked "first."
- `pilot/play_order_first_winrate` — match-win rate conditioned on having chosen "first."
- Per-archetype slices `pilot/match_win_rate/archetype/{name}` and `pilot/match_sweep_rate/archetype/{name}`.
- Per-matchup grid `pilot/matchup/{agent_archetype}_vs_{opp_archetype}/match_win_rate` and corresponding sweep rates.

#### Scenario: TensorBoard receives match metrics during eval

- **WHEN** an eval pass completes in bo3 mode with at least one concede observed
- **THEN** the TensorBoard log writer receives at least `pilot/match_win_rate`, `pilot/match_sweep_rate`, and `pilot/concede_rate` scalars

#### Scenario: Single-mode skips match metrics

- **WHEN** an eval pass completes in `--match-format single`
- **THEN** none of the `pilot/match_*` scalars or `pilot/concede_*` scalars are written

### Requirement: Matchup-grid sidecar JSON

At each evaluation pass in `--match-format bo3` mode, the trainer SHALL write a machine-readable matchup-grid sidecar to `runs/<run_id>/matchup_grid_<step>.json` capturing per-archetype-pair match-win rate, sweep rate, and games-per-match. The schema SHALL be a top-level object whose keys are agent archetype names; each value is an object whose keys are opponent archetype names; each leaf is `{ matches: int, match_wins: int, sweeps: int, total_games: int }`.

#### Scenario: Sidecar written at every eval step in bo3

- **WHEN** an eval pass completes at training step `S` in bo3 mode against a generalist deck pool
- **THEN** a file `runs/<run_id>/matchup_grid_S.json` exists
- **AND** parsing it yields a two-level dict with at least one populated archetype pair

#### Scenario: Sidecar consumable by training MCP

- **WHEN** the matchup-grid sidecar exists for a run
- **THEN** `digimon-training-mcp`'s `run_summary` lists it under the run's available artifacts
