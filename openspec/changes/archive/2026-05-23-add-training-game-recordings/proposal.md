## Why

Training runs currently expose aggregate metrics and checkpoints, but they do not persist the actual game action traces that produced wins, losses, draws, or crashes. This makes it hard to reproduce training-discovered bugs, inspect draws, or explain how an episode ended.

## What Changes

- Add optional training game recording for pilot training, evaluation, and smoke-test workflows.
- Persist deterministic replay artifacts that include post-shuffle initial state, decks, action IDs, player IDs, phase and memory metadata, and optional tensor/mask snapshots.
- Add outcome metadata to each recorded game so consumers can identify the winner, win reason, and draw reason without inferring solely from terminal state.
- Add smoke-test coverage that exercises recording, validates replay artifact shape, and reports terminal outcome details.
- Keep recording disabled by default so normal training performance and artifact volume are unchanged unless explicitly enabled.

## Capabilities

### New Capabilities

- `training-game-recordings`: Optional deterministic game recordings for RL training/evaluation/smoke runs, including terminal outcome explanation.

### Modified Capabilities

- None.

## Impact

- Affected Rust engine surfaces: `HeadlessRunner`, `GameRecorder`, game-over event/outcome metadata, and PyO3 `RustHeadlessGame` bindings.
- Affected Python RL surfaces: `DigimonEnv`, pilot training config/CLI, training callbacks or wrappers, eval loops, and smoke tests.
- Affected artifacts: new JSON replay files under a run-scoped recordings directory, with optional tensor snapshots gated by configuration.
- Affected docs: training runbook and/or tools documentation should explain how to enable recording and inspect artifacts.
