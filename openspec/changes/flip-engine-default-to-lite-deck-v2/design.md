## Context

The engine carries two competing tensor profiles in production simultaneously. `tensor::TENSOR_SIZE = 1375` (v1, `standard_compact_v1`) is the constant every "default" caller reads — Tauri's `engine_commands.rs` (9 call sites), `models.rs::EngineContract`, the manifest compatibility gate, engine integration tests, and the `tensor::build_tensor` function the Python parity tests exercise. But the active product all runs against `standard_lite_v2` (training default, 8410) or `standard_lite_deck_v2` (the deck-aware variant pinned by `decklist-aware-observation-profile`, 8850). The Python training stack avoids the conflict by going through `observation::build_observation(game, pid, registry, profile)` with an explicit profile id; the desktop Tauri build cannot, because its in-process `digimon-engine` library only knows the `DEFAULT_PROFILE` constant.

Result: today's desktop layer builds v1 tensors regardless of what the user wants, and its model-compatibility gate accepts only v1-trained ONNX exports — none of which the project actually produces anymore. Trained models the project ships are unreachable from desktop.

`docs/RUST_PYTHON_PARITY.md` and the runbook's S1.3/S1.4 entries both already accepted breaking-checkpoint costs to converge on `standard_lite_v2` action and tensor layouts. Flipping the engine default to `standard_lite_deck_v2` extends that same precedent and lets us delete the v1 default code path rather than continue dual-maintaining it.

## Goals / Non-Goals

**Goals:**
- A single canonical engine tensor: `tensor::TENSOR_SIZE == 8850`, `tensor::build_tensor` returns a `standard_lite_deck_v2` tensor, `DEFAULT_PROFILE` returns the lite_deck_v2 `TensorProfile`.
- Existing callers that rely on the engine default (Tauri commands, `EngineContract`, MCP binary, integration tests) compile and report the new shape without any per-call code changes.
- Python training and inference code paths that explicitly pass a profile id keep working with no semantic change.
- v1's profile, layout module, and tensor builder remain available behind `tensor_profiles::standard::v1::*` and `profile_by_id("standard_compact_v1")` so historical recordings and any v1-pinned tests can still target it explicitly.
- The v2 profile registry (`profile_by_id`, `all_profile_ids`) is unchanged.

**Non-Goals:**
- Adding a new tensor profile or modifying the `standard_lite_deck_v2` layout. That spec is `decklist-aware-observation-profile` and stays as-is.
- Removing v1 entirely from the codebase. v1 layout constants, `v1::PROFILE`, and the `v1::build_tensor` body stay reachable through their explicit module path.
- Changing the action space (`ACTION_SPACE_SIZE = 2192` is unaffected).
- Building the Tauri test-bench UI, model scanner, or replay viewer. Those land in the follow-up change `add-tauri-engine-test-bench`.
- Migrating already-trained-against-v1 ONNX checkpoints. They become incompatible (S1.3/S1.4 precedent applies).
- Touching PvP / hosted-API state filtering or any frontend-visible game state shape.

## Decisions

### D1. Flip `DEFAULT_PROFILE` rather than make it runtime-configurable

`tensor_profiles::standard::DEFAULT_PROFILE: TensorProfile = v1::PROFILE` becomes `= v2_lite_deck::PROFILE`. Compile-time constant, single source of truth.

**Alternatives considered:**
- *Runtime default via a `set_default_profile()` call:* would let the binary pick its profile, but introduces ordering hazards (what is the default before `set_default_profile`?), thread-safety hazards (atomic? once-cell?), and makes `tensor::TENSOR_SIZE` no longer constant-evaluable, breaking many `vec![0.0; TENSOR_SIZE]` patterns. Rejected.
- *Per-binary feature flags (`features = ["default-v2-lite-deck"]`):* would let `digimon-engine-py` and `src-tauri` pick different defaults via Cargo features. Adds combinatorial test surface for marginal benefit — every consumer that matters has converged on `standard_lite_deck_v2`. Rejected.

### D2. `tensor::build_tensor` delegates to `observation::build_observation`

`tensor::build_tensor(game, pid, registry)` body becomes a one-line call to `observation::build_observation(game, pid, registry, &DEFAULT_PROFILE)`. The v1 builder body that currently lives in `tensor.rs` moves into `tensor_profiles::standard::v1::build_tensor_standard_compact_v1` (parallel to the existing `tensor_v2_lite::build_tensor_standard_lite_v2`) and is reachable via `observation::build_observation(..., &v1::PROFILE)` or directly.

**Alternatives considered:**
- *Keep two separate top-level functions:* `tensor::build_tensor` (the default) and `observation::build_observation` (profile-aware). The current state, minus the v1 hardcoding. Rejected because it duplicates a code path — `build_tensor` would just be `observation::build_observation(..., DEFAULT_PROFILE)` anyway, and we end up with two entry points doing the same thing.

### D3. Drop v1 layout re-exports from `tensor.rs`

`tensor.rs` currently `pub use`s ~30 v1 constants (`OFF_MY_BATTLE`, `SLOT_SIZE`, etc.). After the flip these names are misleading — they would import from `v1::*` but a caller reading `tensor::SLOT_SIZE` would reasonably expect v2_lite_deck slot size. Drop the re-exports; callers must import from `tensor_profiles::standard::v1` explicitly.

This is the most aggressive part of the diff and the one most likely to cascade. Mitigation: grep for `tensor::` constant imports project-wide before merge; any caller that breaks gets either an explicit v1 import or migrates to introspecting the v2_lite_deck profile's `TensorSection` / `TensorSlotLayout` metadata.

**Alternatives considered:**
- *Keep re-exports but document they refer to v1:* leaves a footgun in place. Rejected.
- *Re-export the corresponding v2_lite_deck constants under the same names:* impossible — v2_lite_deck doesn't have a `SLOT_SIZE`-equivalent at the same offset semantics; the slot layout is different. Rejected.

### D4. Compatibility gate uses the engine constants — no per-call profile selection

`models.rs::EngineContract::current()` reads `digimon_engine::tensor::TENSOR_SIZE` and `ACTION_SPACE_SIZE` directly. After the flip it automatically reports 8850. The manifest gate (`download_to_cache` / `load_cached`) compares against these, so v1-trained entries in the hosted manifest are silently dropped from the "compatible" set.

This is the right semantics — desktop builds report what they actually run against. No new compatibility-tier concept (`accept_v1: bool`) is needed.

### D5. Python parity test pins v1 explicitly

`code/engine_py_legacy/tests/engine/test_rust_backend_parity.py` currently asserts behavior against the engine's default tensor. The Python legacy engine produced v1-shaped tensors, so this test's intent was always "v1 against v1." After the flip, the test must request `standard_compact_v1` via `digimon_engine.get_tensor_profile("standard_compact_v1")` and `RustHeadlessGame(..., tensor_profile="standard_compact_v1")` (if such a knob exists; if not, this test is the forcing function to add it to PyO3).

The test is excluded from default pytest collection (per CLAUDE.md rule 22) so this is a quality-of-reference fix, not a CI blocker.

### D6. `runners/replay.rs` and v2_lite_deck's decklist section

The lite_deck_v2 tensor reads from `Player::original_decklist` (or wherever the post-mulligan immutable decklist is stored). The replay runner's docstring says it injects `initial_state` after mulligan; that initial state must include the original decklist or v2_lite_deck tensors built mid-replay will read zeros. Verify pre-flip; if missing, add population to `from_recording` / `from_recording_at_step`.

**Risk:** if existing recordings don't carry an `original_decklist` field, lite_deck_v2 tensors during replay will show an empty decklist section. Mitigation in tasks: add a behavioral test that builds a tensor mid-replay, hashes the decklist section, and compares to the same game played live with `from_decks`.

## Risks / Trade-offs

- **[Risk] V1-trained checkpoints stop loading.** → **Mitigation:** This is the intended outcome and already-accepted precedent (S1.3, S1.4). Communicate via release notes / `RUST_PYTHON_PARITY.md` update. No technical mitigation possible without keeping dual defaults, which D1 rejects.

- **[Risk] `tensor::SLOT_SIZE` (and ~30 other re-exports) disappearing breaks consumers we haven't found.** → **Mitigation:** repo-wide grep for `tensor::` symbol imports before merge; explicit migration to `tensor_profiles::standard::v1::*` for any v1-shaped tests, or to v2_lite_deck `TensorSection` metadata for callers that just want layout info.

- **[Risk] `runners/replay.rs` doesn't populate `original_decklist`.** → **Mitigation:** D6 calls this out; the parity test described there is mandatory in tasks.md before merge. If recordings genuinely lack the field, this change blocks until `training-game-recordings` is extended to capture it.

- **[Risk] `digimon-engine-mcp` binary asserts on v1 shapes somewhere.** → **Mitigation:** run `cargo test -p digimon-engine-mcp` and `cargo run -p digimon-engine-mcp -- --help` smoke as part of the implementation tasks.

- **[Risk] `digimon-engine-py` has implicit dependencies on `tensor::TENSOR_SIZE`.** → **Mitigation:** grep confirms `digimon_engine_py` already routes everything through `profile_by_id` / `RustHeadlessGame`. Build the wheel and run `code/tests/rl/test_pilot_training_config.py` (which excludes legacy parity) to confirm.

- **[Trade-off] We keep the v1 builder source alive instead of deleting it.** Cost: ~600 lines stay in the tree. Benefit: explicit `standard_compact_v1` callers (historic recordings, parity tests, archived models) keep working. The cost is small and the alternative — deleting v1 — invalidates archived recordings and any model in user filesystems. Worth keeping.

## Migration Plan

1. **Engine code.** Move v1 builder to `tensor_profiles::standard::v1::build_tensor_standard_compact_v1`. Rewrite `tensor::build_tensor` as a one-liner. Drop v1 re-exports from `tensor.rs`. Flip `DEFAULT_PROFILE`. Update `TENSOR_SIZE` re-export to come from `v2_lite_deck`.

2. **Engine tests.** Update integration tests (`tensor_and_mask_*`, `mask_*_parity`, etc.) to either assert against the new constant or pin v1 via explicit profile id.

3. **Tauri.** Recompile; expect compile to succeed (the 9 references all go through engine consts). Update integration tests that hard-code `1375` to `8850`. Verify `cargo test --manifest-path code/src-tauri/Cargo.toml` passes.

4. **MCP binary.** `cargo test -p digimon-engine-mcp`; smoke-run the binary.

5. **PyO3.** `cd code/digimon-engine-py && maturin develop`. Run `python -c "from digimon_engine import RustHeadlessGame, get_tensor_profile; print(get_tensor_profile('standard_lite_deck_v2').tensor_size)"` and confirm 8850. Verify the default RustHeadlessGame matches.

6. **Python parity test.** Update `test_rust_backend_parity.py` to pin `standard_compact_v1` explicitly (D5). This test runs out-of-default so a separate `pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py` invocation confirms.

7. **Recordings + replay.** Run the new behavioral test that hashes the decklist section under both `from_decks` and `from_recording_at_step` for the same recorded game. If they diverge, extend the recording format before merging.

8. **Docs.** Update `docs/TENSOR_SPEC.md` (top-level tensor size), `docs/TRAINING_RUNBOOK.md` (any "default profile" mentions), `docs/RUST_PYTHON_PARITY.md` (close the divergence row).

**Rollback strategy:** revert `DEFAULT_PROFILE` constant; revert `tensor::build_tensor` body; restore `tensor.rs` re-exports. Each is a single-file revert. v1 builder relocation can stay — it's behavior-neutral and downstream callers don't import from the new path until the flip lands.

## Open Questions

- Does `Player::original_decklist` (or equivalent) exist today, and if so is it populated by both `Game::new(decks, ...)` and `runners/replay.rs::from_recording`? D6 calls out the parity test that pins this down. If the field doesn't exist, this change pulls in a small extension to the player state.

- Does the hosted-API manifest schema (`ManifestModel.tensor_size`) need to be backfilled for existing entries? The gate filters by exact match, so v1 entries naturally drop. The question is whether to keep the v1 entries in the manifest with `tensor_size: 1375` (passively incompatible) or remove them. Out of scope for this change but worth flagging to the deployment owner.

- The proposal's "Modified Capabilities" note for `live-game-surface` is conditional on existing scenarios hard-coding 1375. Specs for that capability use prose constructors (`LiveGame::from_decks`) and don't appear to hard-code tensor size. Confirm during spec authoring; if no scenarios change, drop the entry.
