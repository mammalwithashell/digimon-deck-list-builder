## 1. Engine Outcome Metadata

- [x] 1.1 Add a Rust terminal outcome reason type that can represent security/direct attack wins, deck-out wins, engine-declared wins, unknown wins, step-limit draws, crash draws, and unknown draws.
- [x] 1.2 Populate outcome reason data from Rust game-over paths such as `handle_deckout` and `declare_winner` without changing action IDs or observation tensor layouts.
- [x] 1.3 Expose outcome metadata through `HeadlessRunner` and the PyO3 `RustHeadlessGame` binding.
- [x] 1.4 Add Rust tests proving deck-out and declared-winner paths expose winner and reason metadata.

## 2. DigimonEnv Recording Surface

- [x] 2.1 Add `record_actions` and `record_tensors` options to `DigimonEnv` and pass them through `_make_runner`.
- [x] 2.2 Preserve existing default behavior by keeping both recording options disabled unless explicitly requested.
- [x] 2.3 Add a backend-neutral `get_recording()` helper or property on `DigimonEnv` that returns the runner recording when available.
- [x] 2.4 Add RL tests proving default env construction does not record and explicit recording captures initial state plus actions.

## 3. Training Artifact Writer

- [x] 3.1 Add training config and CLI fields for recording mode, recordings directory, tensor snapshots, maximum saved recordings, and sample rate.
- [x] 3.2 Implement a small recording collector/writer that wraps completed episode recordings with run metadata and outcome metadata.
- [x] 3.3 Integrate the writer into single-env training and evaluation loops.
- [x] 3.4 Integrate the writer into vectorized training with environment index and per-env game counters in artifact metadata.
- [x] 3.5 Ensure anomaly modes prioritize draws, crashes, invalid-action anomalies, and abnormal terminations over ordinary games.

## 4. Smoke Tests and Validation

- [x] 4.1 Extend `code/tools/train_smoke_test.py` to run at least one recording-enabled episode.
- [x] 4.2 Validate smoke-test recording artifacts include initial state, nonempty action trace, total action count, and outcome metadata.
- [x] 4.3 Add tests for step-limit draw metadata and unknown-reason fallback behavior.
- [x] 4.4 Add a schema or helper assertion for the minimal training recording artifact shape.

## 5. Documentation and Operations

- [x] 5.1 Document recording CLI/config options in the training runbook or tools documentation.
- [x] 5.2 Document artifact file locations, naming, retention controls, and tensor snapshot trade-offs.
- [x] 5.3 Document current replay limitations, including any legacy-runner compatibility caveats and Rust replay follow-up work.
- [x] 5.4 Run targeted Rust, RL, and smoke-test verification for the completed change.
