## ADDED Requirements

### Requirement: Deterministic Game Reconstruction From Recording

The system SHALL provide a `ReplayRunner` type in `digimon-engine` that deterministically reconstructs a `Game` from a `GameRecorder` recording, producing engine state byte-identical to what the recording captured at any chosen step.

`ReplayRunner` SHALL be a port of `engine_py_legacy/engine/runners/replay_runner.py`. It SHALL NOT call `Game::start_game()` (which would re-shuffle); instead it SHALL restore post-shuffle libraries, security stacks, opening hands, digitama libraries, and `first_player_id` directly from the recording's `InitialState`.

`ReplayRunner::new(recording, verify)` SHALL return `Err` if the recording lacks an `initial_state` field, contains card IDs absent from the loaded card pool, or has an internally inconsistent action log.

#### Scenario: Replay reproduces initial state

- **WHEN** a `ReplayRunner` is constructed from a recording and no `step()` calls are made
- **THEN** the resulting `Game`'s libraries, hands, security stacks, digitama, and turn order match the recording's `InitialState` exactly

#### Scenario: Construction rejects missing initial_state

- **WHEN** a `ReplayRunner` is constructed from a recording dict lacking an `initial_state` field
- **THEN** construction returns `Err` with a descriptive error

#### Scenario: Construction rejects unknown cards

- **WHEN** a `ReplayRunner` is constructed from a recording referencing a card not in the loaded card pool
- **THEN** construction returns `Err` naming the unknown card IDs

---

### Requirement: Step-Forward Replay

The `ReplayRunner` SHALL expose `step()` which advances the game by exactly one recorded action and returns a `ReplayStepResult` describing what happened.

`step()` SHALL be idempotent on completion: calling it after `is_complete == true` SHALL return a "no-op" result without mutating the game.

`ReplayStepResult` SHALL include:

- `step_number: u32` — the action just applied
- `player_id: PlayerId`
- `action_id: u16`
- `memory_before: i16`, `memory_after: i16`
- `phase_before: GamePhase`, `phase_after: GamePhase`
- `is_game_over: bool`
- `winner_id: Option<PlayerId>`
- `divergence: Option<DivergenceReport>` — populated only when `verify == true`

#### Scenario: Step advances by one action

- **WHEN** `step()` is called on a fresh `ReplayRunner` constructed from a recording with N >= 1 actions
- **THEN** `current_step` becomes 1 and the resulting state reflects the first action's effect

#### Scenario: Step after completion is no-op

- **WHEN** `step()` is called after every recorded action has been replayed
- **THEN** the game state is unchanged and the result indicates no action was applied

---

### Requirement: Seek-To-Step

The `ReplayRunner` SHALL expose `seek(target_step)` which fast-forwards to a specified step.

If `target_step > current_step`, `seek` SHALL apply intermediate actions in order. If `target_step < current_step`, `seek` SHALL reconstruct the game from the initial state and re-apply actions up to `target_step` (the engine does NOT support backward stepping without rebuild).

`seek` SHALL clamp to `[0, total_steps]`. Calling `seek(0)` SHALL produce a game identical to a freshly-constructed `ReplayRunner`.

#### Scenario: Forward seek matches sequential stepping

- **WHEN** two `ReplayRunner` instances are constructed from the same recording, one calls `seek(50)` and the other calls `step()` fifty times
- **THEN** their game states are equal (verified by comparing all serialized views)

#### Scenario: Backward seek rebuilds from start

- **WHEN** a `ReplayRunner` is stepped to step 100, then `seek(25)` is called
- **THEN** the resulting game state equals what would be produced by constructing fresh and calling `step()` twenty-five times

#### Scenario: Seek out of range clamps

- **WHEN** `seek(usize::MAX)` is called on a recording with 50 actions
- **THEN** the runner ends at step 50 with `is_complete == true`

---

### Requirement: Verify Mode Divergence Detection

The `ReplayRunner` SHALL accept a `verify: bool` constructor argument. When `verify == true`, after each `step()` call the runner SHALL compare key replayed-state fields against the values recorded in the corresponding `RecordedAction` and emit a `DivergenceReport` if any field differs.

`DivergenceReport` SHALL include the step number, the diverging field, the recorded value, and the replayed value.

A divergence SHALL NOT halt replay; the runner continues stepping. Callers consume `DivergenceReport`s via `ReplayStepResult.divergence`.

#### Scenario: Verify catches memory divergence

- **WHEN** a recording's action 5 has `memory_after = 3` but replaying produces `memory == 4` and the runner is in verify mode
- **THEN** the step 5 result's `divergence` field contains a `DivergenceReport` naming `memory_after` with recorded=3 and replayed=4

#### Scenario: Verify mode off produces no reports

- **WHEN** verify is false and the same divergence would occur
- **THEN** `divergence` is always `None` and the runner does not perform comparisons

---

### Requirement: LiveGame Integration

`LiveGame::from_recording` and `LiveGame::from_recording_at_step` SHALL be implemented internally via `ReplayRunner`. The `LiveGame` constructed this way SHALL be indistinguishable from one built with `from_decks` for all subsequent state inspection and action submission operations.

#### Scenario: LiveGame from recording supports action submission

- **WHEN** a `LiveGame` is constructed via `from_recording_at_step(rec, 50)` and the game is mid-turn with legal plays available
- **THEN** `LiveGame::play(...)` advances the game beyond what the recording captured, mutating from the recorded line
