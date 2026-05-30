# recording-replay

## Purpose

Defines deterministic game reconstruction from a recording via the adapter-driven `ReplaySession` (native `GameRecorder` JSON and DCGO `RecordingV1`): step-forward replay, forward/backward seek via in-place reset-and-replay, verify-mode divergence detection, reset-and-replay restore-to-step (including opaque reveal handling and counterfactual replay), and `LiveGame` integration.

## Requirements

### Requirement: Deterministic Game Reconstruction From Recording

The system SHALL provide a `ReplaySession` type in `digimon-engine` that deterministically reconstructs a `Game` from a recording, producing engine state byte-identical to what the recording captured at any chosen step. `ReplaySession` SHALL be parameterized over a `RecordingSource` adapter that owns the recording-format specifics; `ReplayRunner`'s prior responsibilities are subsumed by `ReplaySession` + a `NativeAdapter`.

The system SHALL ship two adapters:

- `NativeAdapter` — reconstructs from a `GameRecorder` recording, preserving the prior `ReplayRunner` behavior exactly. It SHALL NOT call `Game::start_game()` (which would re-shuffle); instead it SHALL restore post-shuffle libraries, security stacks, opening hands, digitama libraries, and `first_player_id` directly from the recording's `InitialState`.
- `DcgoAdapter` — reconstructs from a DCGO `RecordingV1`, dispatching on `opp_deck_post_shuffle`: a standard `Game::new` (both deck orders known) or `Game::new_with_opaque_opponent` with a `RevealQueue` preloaded from the recording's reveal stream.

`ReplaySession::new(source, verify)` SHALL return `Err` if the recording lacks the fields its adapter requires, references card IDs absent from the loaded card pool, or has an internally inconsistent action log.

#### Scenario: Native replay reproduces initial state

- **WHEN** a `ReplaySession` is constructed via `NativeAdapter` from a recording and no `step_forward()` calls are made
- **THEN** the resulting `Game`'s libraries, hands, security stacks, digitama, and turn order match the recording's `InitialState` exactly

#### Scenario: Construction rejects missing initial_state

- **WHEN** a `ReplaySession` is constructed via `NativeAdapter` from a recording dict lacking an `initial_state` field
- **THEN** construction returns `Err` with a descriptive error

#### Scenario: Construction rejects unknown cards

- **WHEN** a `ReplaySession` is constructed from a recording referencing a card not in the loaded card pool
- **THEN** construction returns `Err` naming the unknown card IDs

#### Scenario: DCGO adapter reconstructs deterministic and opaque games

- **WHEN** a `ReplaySession` is constructed via `DcgoAdapter` from a bot-vs-bot recording (`opp_deck_post_shuffle` present) and separately from an opaque PvP recording (`opp_deck_post_shuffle == null` with a reveal stream)
- **THEN** the first reconstructs through `Game::new` with both ordered decks and the second through `Game::new_with_opaque_opponent` with a `RevealQueue` built from the reveal rows

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

The `ReplaySession` SHALL expose `seek(target_step)` which moves the cursor to a specified step.

If `target_step > current_step`, `seek` SHALL apply intermediate actions in order. If `target_step < current_step`, `seek` SHALL reset the existing `Game` instance's mutable state in place to the recording's initial state — **reusing the already-built `card_data` and registries (no `CardData` clone, no registry rebuild)** — and replay forward to `target_step`. It SHALL NOT reconstruct the game via `Game::new` on a backward seek.

`seek` SHALL clamp to `[0, total_steps]`. Calling `seek(0)` SHALL produce a game identical to a freshly-constructed `ReplaySession`.

#### Scenario: Forward seek matches sequential stepping

- **WHEN** two `ReplaySession` instances are constructed from the same recording, one calls `seek(50)` and the other calls `step_forward()` fifty times
- **THEN** their game states are equal (verified by comparing all serialized views)

#### Scenario: Backward seek resets in place, not via Game::new

- **WHEN** a `ReplaySession` is stepped to step 100, then `seek(95)` is called
- **THEN** the resulting game state equals what would be produced by constructing fresh and calling `step_forward()` ninety-five times, AND the backward seek reused the existing game's `card_data` + registries rather than cloning `CardData` / rebuilding registries

#### Scenario: Seek out of range clamps

- **WHEN** `seek(usize::MAX)` is called on a recording with 50 actions
- **THEN** the session ends at step 50 with `is_complete == true`

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

---

### Requirement: Reset and Replay (Restore to Step)

A full-state snapshot of `Game` is not implementable: the mutable game graph is pervasively closure-bearing (`ModifierEntry` is non-`Clone`, `pending_selection` carries a boxed callback, several parked continuations hold boxed closures), so the graph can neither be cloned nor serialized. The engine SHALL instead provide an in-place `Game` reset-for-replay that returns the existing `Game` instance's mutable state to a clean initial state **without** reconstructing `card_data` or the registries, and `ReplaySession` SHALL expose a `restore(step_n)` primitive implemented as reset-for-replay + replay forward to `step_n`.

`restore(step_n)` SHALL produce game state equal to a freshly-constructed session replayed to `step_n`. For opaque games it SHALL re-attach a fresh `RevealQueue` positioned at the corresponding cursor and replay forward, consuming reveals in order; `OpaqueDeckState` SHALL be reset in place along with the rest of the mutable state. After a `restore`, submitting a legal action different from the recorded one SHALL be supported (counterfactual replay), letting the session diverge from the recorded line.

The reset SHALL be guarded by a test asserting that reset-and-replay-to-N is byte-identical (across all serialized views) to a freshly-constructed game replayed to N, so a missed mutable field is caught.

#### Scenario: Restore returns to a prior step's state

- **WHEN** a session is advanced to step 100 and `restore(40)` is called
- **THEN** the resulting game state is byte-identical (all serialized views equal) to a freshly-constructed session stepped forward forty times

#### Scenario: Opaque restore replays the same reveals

- **WHEN** an opaque-game session advances past several reveals, then `restore` returns it to an earlier step
- **THEN** a fresh reveal queue is positioned so subsequent draws/security pops consume the same reveals in the same order as the original forward pass

#### Scenario: Counterfactual action after restore

- **WHEN** `restore(N)` is called and a legal action different from the recorded one is submitted
- **THEN** the action applies and the session state diverges from the recorded line without error
