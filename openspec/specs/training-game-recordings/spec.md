# training-game-recordings Specification

## Purpose
TBD - created by archiving change add-training-game-recordings. Update Purpose after archive.
## Requirements
### Requirement: Configurable training recordings
Pilot training and smoke-test workflows SHALL support optional game recording modes while keeping recording disabled by default.

#### Scenario: Default training run does not record games
- **WHEN** a pilot training run is started without recording options
- **THEN** the system MUST NOT create per-game recording artifacts

#### Scenario: Recording is enabled explicitly
- **WHEN** a pilot training or smoke-test workflow is started with game recording enabled
- **THEN** the system SHALL construct the underlying game runner with action recording enabled

#### Scenario: Tensor snapshots are opt-in
- **WHEN** game recording is enabled without tensor snapshot recording
- **THEN** the system SHALL record replay actions and outcome metadata without storing per-step observation tensors

### Requirement: Deterministic recording artifact
Each saved training game recording SHALL include deterministic replay data sufficient to recreate the played game without relying solely on RNG seed.

#### Scenario: Recorded game includes initial state
- **WHEN** a game recording is saved
- **THEN** the artifact SHALL include post-shuffle initial state for both players, including deck list, library order, digitama order, security order, and opening hand

#### Scenario: Recorded game includes action trace
- **WHEN** a game recording is saved after at least one action
- **THEN** the artifact SHALL include each recorded action ID with step number, acting player, phase, turn, and memory before/after metadata

#### Scenario: Recording has run metadata
- **WHEN** a game recording is saved from training or evaluation
- **THEN** the artifact SHALL include run metadata such as backend, tensor profile, action-space size, seed when available, recording mode, source split, and environment index when applicable

### Requirement: Outcome metadata
Each saved training game recording SHALL include terminal outcome metadata that identifies who won or why the game did not produce a winner.

#### Scenario: Winner is recorded
- **WHEN** a recorded game ends with a winner
- **THEN** the artifact SHALL include `result`, `winner_id`, terminal step count, and a win reason value

#### Scenario: Step cap draw is recorded
- **WHEN** a recorded game is truncated by a training or smoke-test step limit before the engine declares a winner
- **THEN** the artifact SHALL include `result` as `draw`, `winner_id` as null, and draw reason `step_limit`

#### Scenario: Crash draw is recorded when recording is active
- **WHEN** a recorded game crashes and the surrounding workflow classifies the game as a draw
- **THEN** the artifact SHALL include `result` as `draw`, `winner_id` as null, draw reason `crash`, and an error summary

#### Scenario: Unknown reason is explicit
- **WHEN** the system cannot determine a precise terminal reason
- **THEN** the artifact SHALL record the reason as `unknown` rather than inferring an unsupported explanation

### Requirement: Recording retention controls
Training game recording SHALL provide controls that prevent unbounded artifact growth.

#### Scenario: Maximum recordings cap is reached
- **WHEN** recording is enabled with a maximum saved-recordings limit and the limit has been reached
- **THEN** the system SHALL stop writing additional recording artifacts for that run unless the configured mode explicitly selects a higher-priority anomaly

#### Scenario: Sampled recording mode
- **WHEN** recording is enabled in sampled mode
- **THEN** the system SHALL write only games selected by the configured sample rate or deterministic sampler

#### Scenario: Anomaly recording mode
- **WHEN** recording is enabled for anomalies
- **THEN** the system SHALL prioritize draws, crashes, invalid-action anomalies, and other configured abnormal terminations over ordinary completed games

### Requirement: Smoke-test recording validation
The training smoke test SHALL validate that the recording path can create a replay artifact and explain the terminal outcome.

#### Scenario: Smoke test emits recording
- **WHEN** the smoke test runs with recording enabled
- **THEN** it SHALL finish an episode and verify that the recording includes initial state, action trace, and total action count

#### Scenario: Smoke test reports terminal outcome
- **WHEN** the smoke-test episode ends
- **THEN** it SHALL report or assert outcome metadata with either a winner and reason or a draw reason
