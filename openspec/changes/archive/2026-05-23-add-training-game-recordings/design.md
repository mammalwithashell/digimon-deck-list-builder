## Context

Pilot training uses `DigimonEnv` and usually drives the Rust `RustHeadlessGame` runner. The Rust runner already has a `GameRecorder` behind `record_actions` and `record_tensors`, but `DigimonEnv` does not expose those flags and pilot training never retrieves or persists recordings. Training therefore keeps aggregate metrics, checkpoints, and action-validity rates, but not the game traces needed to reproduce bugs.

The server also has recording and replay endpoints, but those are separate from RL training and currently replay through the legacy Python runner. Training needs a backend-neutral artifact contract first; richer replay UI can build on it later.

## Goals / Non-Goals

**Goals:**

- Let training, evaluation, and smoke-test runs optionally persist deterministic game recordings.
- Include enough state to reproduce the played game without relying only on RNG seed.
- Attach explicit outcome metadata for winner, win reason, draw reason, truncation, crashes, and step counts.
- Keep recording disabled by default and cheap when disabled.
- Provide smoke-test coverage that proves recordings can be emitted and outcome metadata is present.

**Non-Goals:**

- Do not change observation tensor or action-space contracts.
- Do not make all training runs record every game by default.
- Do not require a database or hosted API service for local training recordings.
- Do not build a full Rust-native replay browser in this change, though artifacts should be suitable for a future replay tool.

## Decisions

### Recording is controlled by training configuration

Add config/CLI fields for recording mode, output directory, tensor snapshots, maximum saved recordings, and optional sample rate. `off` remains the default.

Alternatives considered:

- Always record every game: simplest for debugging, but unacceptable for long training runs because action and tensor artifacts can grow quickly.
- Only record crashes: useful, but misses draws, strange wins, and smoke-test replay validation.

### Use a run-scoped file artifact contract

Persist one JSON artifact per recorded game under the training run directory, for example `models/<run>/recordings/eval_step_000025000_game_000003_draw_step_limit.json`. Each artifact wraps the existing recorder payload with training metadata and terminal outcome metadata.

Alternatives considered:

- JSONL stream: efficient for large collections, but harder to hand-inspect, attach to bug reports, or replay one game.
- Database persistence: useful for hosted jobs, but adds service coupling to standalone training.

### Record post-shuffle initial state plus action IDs

The replay payload should preserve post-shuffle library, digitama, security, and opening-hand state. Seeds and deck lists may be included as metadata, but replay must not require RNG behavior to remain stable.

Alternatives considered:

- Seed plus action list only: smaller, but brittle if shuffling, mulligan setup, or RNG code changes.
- Full state snapshot every step: easier to inspect, but much larger and unnecessary for deterministic replay unless tensor snapshots are explicitly enabled.

### Outcome reason is first-class metadata

Add or expose engine-level terminal reason data instead of relying entirely on post-hoc inference. At minimum, classify security/direct attack wins, deck-out wins, engine-declared wins, step-limit draws, no-winner draws, and crash draws. If an engine outcome reason is unavailable for a legacy path, training may record `unknown` with enough final-state metadata to investigate.

Alternatives considered:

- Infer reason only from final state: avoids engine changes, but cannot reliably distinguish effect wins, direct security attacks, and timeout/declaration paths.
- Parse human-readable logs: fragile and unavailable in the default silent training runner.

### Recording lives below the wrapper boundary

Expose recording flags through `DigimonEnv` and pass them into runner construction. Use a small wrapper or callback around training/eval environments to collect completed episodes and write artifacts after terminal steps.

Alternatives considered:

- Modify SB3 internals: unnecessary and harder to maintain.
- Record only in callbacks from `locals`: callbacks can see actions/masks, but they do not own reset-time initial state or runner recordings.

## Risks / Trade-offs

- Recording every game can generate large artifacts -> Keep default `off`, add caps and sampling, and make tensor snapshots opt-in.
- Vectorized training has multiple concurrent episode streams -> Include environment index and per-env game counters in artifact names/metadata.
- Legacy Python and Rust recording schemas can drift -> Validate a shared minimal artifact schema in tests and document backend fields.
- Existing replay endpoints are legacy-runner based -> Treat compatibility as best-effort in this change and make future Rust replay tooling possible through deterministic artifacts.
- Outcome reasons may initially be incomplete for some win paths -> Use explicit `unknown` reason values rather than inventing explanations.

## Migration Plan

1. Add outcome reason data to Rust game-over surfaces without changing action IDs or tensor layouts.
2. Expose runner recording flags and `get_recording()` through `DigimonEnv`.
3. Add training config/CLI controls and recording artifact writer.
4. Add smoke and unit coverage for recording shape, outcome metadata, and disabled-by-default behavior.
5. Document how to enable recordings and where artifacts are written.

Rollback is straightforward: leave new config at default `off`, or remove the recording wrapper from training construction. Existing checkpoints and models are unaffected because observation/action contracts do not change.

## Open Questions

- Should hosted `TrainingJob` rows eventually link to saved recording artifacts, or should that wait for a storage policy?
- Should crashes save partial recordings by default when recording mode is `off`, or only when an anomaly/crash mode is explicitly selected?
- Should Rust-native replay validation be included in this change, or proposed as a follow-up once the artifact contract lands?
