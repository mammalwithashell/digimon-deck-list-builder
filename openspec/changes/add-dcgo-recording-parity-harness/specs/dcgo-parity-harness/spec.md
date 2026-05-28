## ADDED Requirements

### Requirement: DCGO Recording JSONL Schema

The system SHALL define a versioned JSONL recording schema produced by the modded DCGO client and consumed by the Rust replay harness and the behavioral-cloning dataset emitter.

Each recording file SHALL contain a sequence of newline-delimited JSON objects. The first object SHALL be a `game_start` row containing the schema version, both decks (post-shuffle order for both players when both are known, `null` for opponent in PvP mode), the recording-side player ID, the DCGO build identifier, and a UTC timestamp. The final object SHALL be a `game_end` row containing the winning player ID and a terminal-reason string. All intermediate rows SHALL be one of: `action` (a decision encoded as a 2192-space action ID), `reveal` (an opponent-deck reveal observed during PvP play — draws, security pops, mill), or `phase_marker` (an optional human-readable phase transition for debugging, never required by the replay harness).

Action rows SHALL include `actor` (0 or 1, indexing players), `action_id` (0..2191), and `phase` (a string naming the engine `GamePhase` that contextualized the encoding). Reveal rows SHALL include `card_id`, `source` (`draw` | `security` | `mill` | `effect`), and the step index at which the reveal occurred.

The schema version SHALL be bumped whenever the action-space size, the JSONL row shape, or the reveal model changes. The replay harness SHALL reject recordings whose version it does not understand with an explicit error.

#### Scenario: Bot-vs-bot recording carries both deck orders

- **WHEN** the modded DCGO client records a bot-vs-bot game with `isAuto + IsAI = true`
- **THEN** the resulting JSONL `game_start` row contains `my_deck_post_shuffle` AND `opp_deck_post_shuffle` as fully-ordered card-ID arrays

#### Scenario: PvP recording omits opponent deck order

- **WHEN** the modded DCGO client records a PvP random-match game
- **THEN** the resulting JSONL `game_start` row contains `my_deck_post_shuffle` as a fully-ordered card-ID array AND `opp_deck_post_shuffle` is `null`
- **AND** every observed opponent-side reveal during the game produces a `reveal` row with the revealed card ID and source

#### Scenario: Replay harness rejects unknown schema version

- **WHEN** the replay harness loads a recording whose `version` field exceeds the harness's supported version
- **THEN** the harness exits with a non-zero status and an error message naming the recording file, the offending version, and the highest version supported

### Requirement: DCGO Recorder Intercepts All Player Decisions

The modded DCGO client SHALL emit one `action` row per decision made by either player. Decision sources covered SHALL include: (a) all six `MainPhaseAction` subclasses (`PlayCardAction`, `AttackPermanentAction`, `ActivateCardAction`, `ActivatePermanentAction`, `PassAction`, `CheatAction`); (b) every selection response routed through `UserSelectionManager.SetIntForPlayer` and `SetBoolForPlayer`; (c) mulligan keep/redraw decisions; (d) optional-trigger accept/decline prompts (the `SkipSameEffect` / `OptionalSkill` flow); (e) target selections, count selections, and source selections for all `SelectXEffect` subclasses.

Each decision SHALL be encoded into a 2192-space action ID using the codegen-aligned `ActionSpace` table. The recorder SHALL be source-agnostic with respect to the actor: bot decisions, AI decisions, and human decisions all flow through the same intercepts and produce identical row shapes.

If a decision is made that the encoder cannot map to a 2192-space ID, the recorder SHALL write a sentinel `encoder_failure` row containing the DCGO state context and SHALL continue recording subsequent decisions. It SHALL NOT crash the host client or abort the recording.

#### Scenario: Bot's play-card action is captured identically to a human's

- **WHEN** the bot opponent in a Bot Match queues a `PlayCardAction` via `TurnStateMachine.QueueMainPhaseAction`
- **THEN** the recorder emits one `action` row with `actor` set to the bot's player ID and `action_id` encoding the play

#### Scenario: Selection response is captured

- **WHEN** a player resolves a `SelectPermanentEffect` prompt by clicking a target, sending `SetIntForPlayer(pid, value)`
- **THEN** the recorder emits one `action` row encoding the selection as the appropriate `pending_selection` action ID for the current selection phase

#### Scenario: Encoder failure does not crash the client

- **WHEN** the recorder encounters a decision it cannot encode (e.g., a DCGO state with no representation in the current action space)
- **THEN** the recorder writes one `encoder_failure` row with diagnostic context AND the DCGO game continues unaffected AND subsequent decisions in the same game continue to be recorded

### Requirement: Action-Space C# Table is Codegen-Aligned with Rust

The C# `ActionSpace` table used by the DCGO encoder SHALL be generated from `code/digimon-engine/src/action/space.rs` by the `action-space-export` tool. Hand-edited C# constants for any 2192-space ID range SHALL NOT exist.

The generated file SHALL contain: every `pub const` from `space.rs`; encoder helper functions for every formula-based range (`EncodeAttack`, `EncodeDigivolve`, `EncodeFieldEffect`, `EncodeSourceSelect`, `EncodeBreedingSourceSelect`); the `ACTION_SPACE_SIZE` constant; and a `SCHEMA_VERSION` constant matching the action-space version.

CI SHALL run the regeneration and fail if the generated file differs from the committed copy.

#### Scenario: Regeneration produces no diff

- **WHEN** CI runs `cargo run -p action-space-export | <emitter> --out DCGO/Assets/Scripts/Script/Recording/ActionSpace.cs`
- **AND** the resulting file is diffed against the committed copy
- **THEN** the diff is empty AND CI passes the drift check

#### Scenario: Rust constant change without regeneration fails CI

- **WHEN** a developer modifies `code/digimon-engine/src/action/space.rs` (e.g., adds a new constant or changes a range bound) without re-running the emitter
- **THEN** the CI drift check produces a non-empty diff AND the workflow fails with an actionable message pointing at the regeneration command

### Requirement: Rust Replay Harness Validates Action Legality and Game Outcome

The `dcgo-replay` Rust binary SHALL consume a DCGO JSONL recording and replay it through `digimon-engine`. For each `action` row, the harness SHALL: (a) query the engine's action mask for the actor; (b) assert the recorded `action_id` is legal under the mask; (c) call `runner.step(action_id)`; (d) on failure, emit a per-game parity report row containing the recording path, the step index, the actor, the offending action ID, the engine's current phase, and a diagnostic excerpt of the mask.

After consuming the final `action` row, the harness SHALL assert that the engine reports `game_over = true` AND `winner` matches the recording's `game_end.winner`. Mismatches SHALL be reported as `winner_mismatch` failures with both engines' final state summaries.

The harness SHALL run in batch mode over a directory of recordings, aggregating failures into a single parity report keyed by failure type, by card involved (where determinable from the offending action), and by step distribution.

#### Scenario: Recording with all-legal actions and matching winner passes

- **WHEN** the harness replays a recording whose action stream is fully legal in the Rust engine AND whose game-end winner matches the engine's `winner()`
- **THEN** the harness reports the recording as `pass` AND no failure row is emitted

#### Scenario: Illegal action surfaces at first divergence

- **WHEN** the harness replays a recording AND at step N the recorded `action_id` is not in the engine's legal-action mask for the current actor
- **THEN** the harness halts replay of that recording AND emits a failure row with `kind = "illegal_action"`, the step index, the offending action ID, and the actor's current mask

#### Scenario: Winner mismatch surfaces after full replay

- **WHEN** the harness replays a recording AND every action in the stream is legally consumed AND the engine's final `winner()` differs from the recording's `game_end.winner`
- **THEN** the harness emits one failure row with `kind = "winner_mismatch"`, the engine's winner, the recording's winner, and the terminal step index

### Requirement: Per-Card Parity Report

The replay harness SHALL aggregate replay failures over a corpus and emit a per-card parity report. The report SHALL list, for each card involved in at least one failure, the count of failures attributed to that card, the failure types, and a representative recording-and-step reference for the first observed failure.

Card attribution SHALL be derived from the action context — for `play_hand` actions the card being played; for `digivolve` actions the digivolution-source card; for `activate_effect` actions the effect-source card; for `selection` actions the card driving the selection prompt where determinable, or "selection-context" otherwise.

The report SHALL be a deterministic, machine-readable artifact (JSON) at a stable path under `recordings/dcgo/parity_reports/<timestamp>.json` to permit diffing across runs.

#### Scenario: Card with repeated divergence is surfaced in the report

- **WHEN** the harness replays a corpus of 1000 recordings AND 17 of them fail at an action involving card `BT15-104`
- **THEN** the parity report's `per_card` section contains an entry for `BT15-104` with `failure_count: 17` AND a representative `first_failure` reference

#### Scenario: Report is deterministic across runs over the same corpus

- **WHEN** the harness is run twice against the same corpus
- **THEN** both runs produce per-card sections that are byte-identical after sorting card IDs

### Requirement: Bot-Vs-Bot Recording Mode Operates Without Network Activity

When the modded DCGO client is launched in Bot Match mode with `isAuto = true` AND `IsAI = true`, the recorder SHALL emit recordings deterministically and SHALL NOT initiate any Photon network activity for the gameplay session. This mode SHALL be the canonical Phase 1 development and CI loop.

#### Scenario: Bot-vs-bot session produces a recording offline

- **WHEN** the modded DCGO client is launched with `isAuto = IsAI = true` AND no network is available
- **THEN** at least one complete recording is written to disk under the recording output directory
- **AND** the recording's `game_start` row contains both decks fully ordered
- **AND** the recording's `game_end` row contains a winner

#### Scenario: Bot-vs-bot game does not contact the Photon cloud

- **WHEN** the modded DCGO client runs a Bot Match game with recording enabled
- **THEN** no Photon Realtime connection is established for the gameplay session
