## Why

The engine's canonical default observation profile is still `standard_compact_v1` (1375 floats), but every consumer that matters has moved on: pilot training runs `standard_lite_v2`, models compatible with desktop are trained against `standard_lite_deck_v2` (8850), and the existing `decklist-aware-observation-profile` spec already pins the deck-aware variant as the target. The Tauri desktop layer is forced to read `tensor::TENSOR_SIZE` and `tensor::build_tensor` to talk to ONNX models, so today it builds v1 tensors and rejects every modern model at the shape gate. This change retires the v1 default so the engine has one source-of-truth tensor profile and trained models can actually run on desktop.

## What Changes

- **BREAKING** Flip `tensor_profiles::standard::DEFAULT_PROFILE` from `v1::PROFILE` to `v2_lite_deck::PROFILE` (8850 floats).
- **BREAKING** `tensor::TENSOR_SIZE` becomes 8850; `tensor::build_tensor(game, player, registry)` returns a `standard_lite_deck_v2` tensor instead of a `standard_compact_v1` tensor.
- **BREAKING** Re-exports in `tensor.rs` that surfaced v1 layout constants (`OFF_MY_BATTLE`, `SLOT_SIZE`, etc.) are removed; callers either import them explicitly from `tensor_profiles::standard::v1` or migrate to the v2_lite_deck layout via the existing `TensorProfile`/`TensorSection` introspection API.
- `tensor::build_tensor` delegates to `observation::build_observation(game, player, registry, DEFAULT_PROFILE)` so the canonical builder routes through the dispatcher every other caller already uses.
- The Tauri `EngineContract::current()`, `validate_shapes`, `default_profile()` call site in `engine_commands.rs`, and the manifest-gate check in `models.rs` all report 8850 / `standard_lite_deck_v2` without code changes (they read the engine constants).
- The Python-side parity test (`code/engine_py_legacy/tests/engine/test_rust_backend_parity.py`) is updated to expect 8850 or pinned to `standard_compact_v1` via `profile_by_id`, whichever matches its current intent.
- Models trained against the v1 profile no longer load on desktop or via PyO3 fallback paths. The hosted manifest's compatibility filter (which already reads the engine's reported `tensor_size`) automatically drops v1 entries — no manifest schema change needed.
- Documentation under `docs/TENSOR_SPEC.md`, the Track-C / generalist sections of `TRAINING_RUNBOOK.md`, and `RUST_PYTHON_PARITY.md` are updated to reflect the new default.

## Capabilities

### New Capabilities
- `engine-default-observation-profile`: defines which observation profile the engine reports as its canonical default — the value behind `tensor::TENSOR_SIZE`, `tensor::build_tensor`, `tensor_profiles::standard::DEFAULT_PROFILE`, and `EngineContract::current()`. Pins the contract that desktop builds, the MCP binary, and any tool reading "the engine's tensor" all see the same profile.

### Modified Capabilities
- `decklist-aware-observation-profile`: no behavior change to the profile itself, but the spec adds a scenario asserting that this profile is the value returned by `default_profile()` (closing the loop between "the profile exists" and "it is the engine default").

## Impact

- **Affected code (Rust)**
  - `code/digimon-engine/src/tensor.rs` — re-exports, `TENSOR_SIZE`, `build_tensor` body
  - `code/digimon-engine/src/tensor_profiles/standard/mod.rs` — `DEFAULT_PROFILE` constant
  - `code/digimon-engine/src/observation.rs` — confirm `build_observation` dispatches lite_deck_v2 path correctly
  - `code/digimon-engine/src/runners/replay.rs` — verify recorded initial state populates the own-original-decklist section the new default tensor reads
  - `code/digimon-engine/tests/*.rs` — any test that asserts `TENSOR_SIZE == 1375` (e.g., `tensor_and_mask_*`, `mask_*_parity`)
  - `code/src-tauri/src/engine_commands.rs` — 9 references compile against the new constants; integration tests that assert `tensor_size, 1375` become `8850`
  - `code/src-tauri/src/models.rs` — `EngineContract::current()` automatically reports 8850; compatibility-gate tests update
  - `code/digimon-engine-mcp/` — confirm MCP binary's tensor accessors compile and report new shapes
  - `code/digimon-engine-py/src/lib.rs` — Python bindings already pick profile by explicit ID; verify no implicit `tensor::TENSOR_SIZE` references
- **Affected code (Python)**
  - `code/engine_py_legacy/tests/engine/test_rust_backend_parity.py` — pin or update tensor-size expectations
  - `code/digimon_gym/agents/pilot_training.py` and friends — already pass `--tensor-profile` explicitly; no functional change
- **Affected docs**
  - `docs/TENSOR_SPEC.md`, `docs/TRAINING_RUNBOOK.md`, `docs/RUST_PYTHON_PARITY.md`
- **Model compatibility**
  - **Existing v1-trained ONNX checkpoints stop loading on desktop and via PyO3 fallback paths.** This is the intended outcome — those models are already incompatible with the engine the project ships. Action S1.3 / S1.4 retrains in `TRAINING_RUNBOOK.md` are the formal precedent for accepting compatibility breaks of this shape.
- **No changes to**
  - Action space (still `ACTION_SPACE_SIZE = 2192`)
  - PvP / hosted-API state filtering
  - Card-script behavior
  - Replay file format
