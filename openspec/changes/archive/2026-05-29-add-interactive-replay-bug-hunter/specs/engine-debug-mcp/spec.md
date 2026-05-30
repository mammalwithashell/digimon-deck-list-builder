## MODIFIED Requirements

### Requirement: Lifecycle Tools

The MCP server SHALL expose the following lifecycle tools:

- `new_game_from_decks(deck1: [card_id], deck2: [card_id], seed: number | null) -> { game_id }`
- `new_game_debug(hands: {p0, p1}, decks: {p0, p1}, first_player: 0 | 1) -> { game_id }`
- `load_recording(path_or_json: string) -> { game_id, total_steps, source_format }` — SHALL accept both a native `GameRecorder` JSON document and a DCGO `RecordingV1` JSONL document (deterministic bot-vs-bot or opaque PvP), auto-detecting the format and selecting the corresponding `RecordingSource` adapter. `source_format` SHALL report which adapter was selected.
- `seek(game_id, step_n: number) -> { current_step }`
- `list_games() -> [{ game_id, source, current_step, turn_count, game_over }]`
- `close_game(game_id) -> { ok }`

The server SHALL impose an upper bound (configurable, default 32) on the number of concurrent games. Exceeding the bound SHALL return a structured error suggesting `close_game`.

#### Scenario: Lifecycle round-trip

- **WHEN** a client opens a game with `new_game_from_decks`, queries `list_games`, then `close_game`s it
- **THEN** the game appears in the list after open, is gone from the list after close, and subsequent calls with the freed `game_id` return unknown-game errors

#### Scenario: Concurrent limit enforced

- **WHEN** the configured limit is 32 and a client attempts to open a 33rd game
- **THEN** `new_game_from_decks` returns a structured error and no game is created

#### Scenario: load_recording ingests DCGO recordings

- **WHEN** a client calls `load_recording` with the path to a DCGO bot-vs-bot JSONL recording, and separately with an opaque PvP JSONL recording
- **THEN** both return a `game_id` with `source_format` identifying the DCGO adapter, the deterministic recording reconstructs with both deck orders, and the opaque recording reconstructs via the opaque-opponent path with its reveal stream

#### Scenario: load_recording ingests native recordings

- **WHEN** a client calls `load_recording` with a native `GameRecorder` JSON recording
- **THEN** the call returns a `game_id` with `source_format` identifying the native adapter and reconstructs the post-mulligan state

## REMOVED Requirements

### Requirement: Branching Tools Deferred

**Reason**: Superseded — the deferral assumed branching required cheap `Game::Clone` (Arc-wrapping `card_data`). The task-1.1 audit found the mutable game graph is closure-bearing and not cloneable/serializable, so this change instead implements stepping and branching via **reset-and-replay** (in-place reset of the existing game + deterministic forward replay), which needs no clone. Branching tools are therefore now in scope. See the new "Interactive Replay Stepping Tools" requirement.

**Migration**: Clients gain `step_forward`, `step_back`, `restore_checkpoint`, and `replay_step_view` tools (backed by reset-and-replay). There is no removal of existing behavior; the deferral note is retired.

## ADDED Requirements

### Requirement: Interactive Replay Stepping Tools

For a `game_id` backed by a loaded recording, the MCP server SHALL expose tools that drive the underlying `ReplaySession` cursor:

- `step_forward(game_id) -> StepView` — advance the cursor by one recorded action and return the step view.
- `step_back(game_id) -> StepView` — move the cursor back by one step (checkpoint-restore-backed) and return the step view at the new cursor.
- `restore_checkpoint(game_id, step_n) -> { current_step }` — restore game state to the cursor position `step_n`.
- `replay_step_view(game_id, step_n) -> StepView` — return the step view for a given step without permanently moving the live cursor.

These tools SHALL return a structured error for a `game_id` that was not created from a recording. Stepping past the end SHALL be a no-op that returns the terminal step view.

#### Scenario: Step forward then back returns to prior state

- **WHEN** a client calls `step_forward` to reach step N, then `step_back`
- **THEN** the cursor is at step N-1 and the returned game state equals the state before the step-N action was applied

#### Scenario: Stepping tools reject non-recording games

- **WHEN** a client calls `step_forward` on a `game_id` created via `new_game_from_decks`
- **THEN** the tool returns a structured error indicating the game is not backed by a recording

### Requirement: Fat Step View Tool Payload

The step view returned by the stepping tools SHALL be a single structured object carrying, for the step at the cursor: the decoded recorded action (`action_id`, decoded label, `card_id`, `actor`, `phase`), the engine's full decoded legal-action set, any divergence detected at that step, the events emitted by applying the recorded action, a before/after state delta, and the card IDs in play. The object SHALL be sufficient for both differential and judge bug-hunting without further round-trips for the common case (card text is fetched separately via `inspect_card`).

#### Scenario: Step view carries decoded action and legal set

- **WHEN** a client reads a step view for any step
- **THEN** the payload includes the decoded recorded action AND the engine's decoded legal-action set at that step

#### Scenario: Step view carries events and delta

- **WHEN** a client reads a step view for a step whose recorded action changed the board
- **THEN** the payload includes the structured events emitted and a before/after state delta reflecting the change

### Requirement: Mechanical Scanner Tools

The MCP server SHALL expose cheap, deterministic scanner tools over a recording-backed `game_id`:

- `scan_divergences(game_id, stop_at_first?: bool) -> [Divergence]` — replay under the CheckThenApply policy and report mask-membership / actor / winner / reveal divergences; `stop_at_first` defaults to `true`.
- `scan_fizzles(game_id) -> [FizzleLead]` — collect `EffectFizzled` events across the replay, each with the step and source permanent.
- `scan_panics(game_id) -> [PanicLead]` — collect any recorded engine panics/errors across the replay.

Scanners SHALL NOT mutate the caller-visible cursor position (they operate on an internal pass or restore the cursor afterward).

#### Scenario: scan_divergences finds a masked-out recorded action

- **WHEN** a DCGO recording contains an action the Rust engine masks out at that step and a client calls `scan_divergences`
- **THEN** the response contains a divergence naming the step, the recorded `action_id`, and a sample of the engine's legal actions at that point

#### Scenario: scan_fizzles surfaces fizzle leads

- **WHEN** a recording's replay produces one or more `EffectFizzled` events and a client calls `scan_fizzles`
- **THEN** the response lists each fizzle with its step and source permanent
