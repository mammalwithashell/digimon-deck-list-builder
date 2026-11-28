## ADDED Requirements

### Requirement: Dual-Seat Live Play With Real Card Effects
The Studio SHALL support transitioning from Compose mode into Play & Record mode on a live `RustDebugGame` session configured with `p1_human = true` and `p2_human = true` and driven through the real Rust engine action mask and `step` / `step_checked` pipeline. All card effects, triggered effects, pending selections, timing windows, and memory seesaw rules MUST process normally without auto-selections or shortcuts.

In Play & Record mode:
- Board orientation SHALL remain fixed by default with **both Player 1 and Player 2 hands rendered face-up and interactive**, clearly highlighting whichever seat currently holds decision priority (`pendingSelection.selectingPlayer ?? currentPlayer`).
- The user SHALL be able to toggle a manual "Flip View" control to swap visual top/bottom seats without altering engine player IDs.

#### Scenario: Controlling both sides across an interrupt selection
- **WHEN** Player 1 declares an attack from a composed board and the Rust engine enters `BlockTiming` / `CounterTiming` with `selectingPlayer = 2`
- **THEN** the board stays in its current orientation, highlights Player 2 as the active decision-maker, enables Player 2's legal blocker/counter actions from the engine's action mask, and resolves Player 2's choice through the Rust engine

#### Scenario: Card effects trigger and resolve normally from a staged board
- **WHEN** the user transitions from Compose mode to Play & Record mode and plays a card with an `[On Play]` selection effect from Player 1's hand
- **THEN** the Rust engine fires the `[On Play]` trigger, opens the corresponding selection panel/board highlights, and applies the resulting state delta identically to a normal match

### Requirement: Deterministic Move Recording and Timeline Rewind
While in Play & Record mode, the Studio SHALL record every action taken after the base staged board as a timeline entry containing `step_index`, `action_id`, `actor`, `phase_before`, `phase_after`, `memory_after`, human-readable action label, and emitted `GameEvent` summary.

The user SHALL be able to step backward, undo the last move, or click any earlier step in the timeline to seek to that state. Rewinding SHALL reconstruct the `RustDebugGame` deterministically from the base staged board snapshot + seed and replay `actions[0..target]`.

#### Scenario: Step back mid-recording and take a different action
- **WHEN** the user records 5 actions from a staged board, clicks step 2 in the timeline, and submits a different legal action
- **THEN** the backend rebuilds the staged board, replays actions 0..2, applies the new action as step 3, truncates the old tail on the active line, and updates the board and action mask accurately

### Requirement: Snapshotting Current Board as New Puzzle Start
The Studio SHALL provide a **"Snapshot current board as puzzle start"** action during Play & Record mode that captures the current live `Game` state via `Game::to_scenario()`, replaces the session's base staged board with that post-effect state, and clears the active recording tape so the user can author puzzles starting mid-sequence after setup effects have resolved.

#### Scenario: Freeze a post-effect board state as the puzzle start
- **WHEN** the user plays 3 setup actions to resolve triggers and then clicks "Snapshot current board as puzzle start"
- **THEN** the Studio captures the post-step-3 state as the new base staged board, resets the recorded line length to 0, and subsequent saves start from that post-effect board

### Requirement: Multi-Line Recording and Assertion/Expectation Authoring
From a single base staged board, the Studio SHALL allow the user to save and switch between multiple named recorded lines, each tagged with a role (`target`, `avoid`, or `alternative`), without losing the base board or previously saved lines.

The Studio SHALL also allow authoring:
- Search/policy expectations (`visits` with `min_share`, `avoid`, `choose`) bound directly to a recorded line's action sequence or a manual action plan.
- Engine assertions (`memory_equals`, `stack_top`, `effective_dp`, `zone_count`, `zone_contains`, `effect_triggered`, `action_legal`, `legal_selection_options`) testable live against the current board via `/debug/games/{id}/evaluate`.

#### Scenario: Record a target line and an avoid line from the same board
- **WHEN** the user records a 2-move combo, saves it as `"Training -> Coredramon"` (`role: "target"`), resets to step 0, records a 1-move blunder, and saves it as `"Hard-play Wingdramon"` (`role: "avoid"`)
- **THEN** both named lines are attached to the same base staged board and can be one-click promoted into `choose`/`visits` and `avoid` expectations
