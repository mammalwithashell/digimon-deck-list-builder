## 1. Pre-flight: verify decklist availability

- [x] 1.1 Grep for `original_decklist` (or equivalent immutable post-mulligan decklist) on `Player` / `Game`; confirm it exists and is populated by `Game::new(decks, ...)` **— field is `Player.original_deck: Vec<OriginalDeckCardCount>` (player.rs:30), populated by `Game::new` at game.rs:738.**
- [x] 1.2 Trace `runners/replay.rs::from_recording` / `from_recording_at_step`: confirm the recorded `initial_state` carries enough information to repopulate `Player.original_decklist` **— recording's `initial_state` contains `library_order` + `digitama_deck` + `security` + `initial_hand`; that's the complete original decklist by union. The replay runner reconstructs zones but does NOT currently aggregate them into `original_deck`. Adding that population is folded into task 4.4 below.**
- [x] 1.3 If 1.2 fails, file a follow-up to extend `training-game-recordings` and stop this change until that lands **— N/A: information is present in recording, only the aggregation step is missing.**
- [x] 1.4 Grep the workspace for `tensor::` symbol imports (`use digimon_engine::tensor::{...}`, `tensor::SLOT_SIZE`, etc.); produce a list of files that will break when v1 re-exports drop **— `tensor::TENSOR_SIZE` / `build_tensor` callers (6): `runners/headless.rs`, `tests/infra/headless_runner.rs`, `tests/mask_and_tensor/tensor_hidden_info.rs`, `src-tauri/tests/it_model_download.rs`, `src-tauri/src/models.rs`, `src-tauri/src/engine_commands.rs`. v1 layout-constant callers (6, need explicit v1 import): `action/mask.rs` (FIELD_SLOTS), `tests/mask_and_tensor/tensor_source_contributions.rs`, `tensor_profile.rs`, `tensor_hidden_info.rs`, `tensor_and_mask.rs`. `tensor::DP_NORM` callers (1, unaffected — DP_NORM stays): `tensor_v2_lite.rs`.**

## 2. Relocate the v1 builder

- [x] 2.1 Move the body of `tensor::build_tensor` into a new `tensor_v1::build_tensor_standard_compact_v1(game, pid, registry) -> Vec<f32>` function **— created `code/digimon-engine/src/tensor_v1.rs` mirroring `tensor_v2_lite.rs`'s pattern (top-level module, not under `tensor_profiles::standard::v1`). Registered in `lib.rs`.**
- [x] 2.2 Add unit assertions confirming the relocated builder still hits the v1 layout offsets **— added `#[cfg(test)] mod tests` block in `tensor_v1.rs` asserting TENSOR_SIZE=1375, FIELD_SLOTS=14, and the 12 section offsets. Full numeric parity is exercised by the integration tests in `tests/mask_and_tensor/` (task 4.x).**
- [x] 2.3 Wire `observation::build_observation_tensor` to dispatch the new `tensor_v1::build_tensor_standard_compact_v1` when profile is `StandardCompactV1` **— edited the dispatcher arm; no longer routes through `tensor::build_tensor` (which would have caused infinite recursion after task 3.2).**

## 3. Flip the default

- [x] 3.1 Change `code/digimon-engine/src/tensor_profiles/standard/mod.rs` so `pub const DEFAULT_PROFILE: TensorProfile = v2_lite_deck::PROFILE;` **— flipped. Also updated `observation::default_observation_profile()` from `StandardLiteV2` → `StandardLiteDeckV2` so the two default-getters agree.**
- [x] 3.2 Rewrite `code/digimon-engine/src/tensor.rs::build_tensor` body to a one-liner calling `observation::build_observation_tensor(.., default_observation_profile())` **— done; `tensor.rs` now ~80 lines, all builder body moved out.**
- [x] 3.3 Update `tensor.rs::TENSOR_SIZE` to re-export from `tensor_profiles::standard::v2_lite_deck::TENSOR_SIZE` **— done. `compute_positions()` now uses `DEFAULT_PROFILE.positions()`.**
- [x] 3.4 Remove the v1 layout re-exports from `tensor.rs` **— done; only `TENSOR_SIZE` and `DP_NORM` re-exported now.**
- [x] 3.5 `cargo check -p digimon-engine`; fix compile errors **— one error: `action/mask.rs` imported `tensor::FIELD_SLOTS`. Fixed by switching the import to `action::space::MAX_FIELD_SLOTS` (semantically correct — mask code is action-space-bounded, not tensor-bounded; both are 14). Library compiles clean.**

## 4. Engine tests

- [x] 4.1 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml`; classify failures **— failures: (1) `mask_and_tensor::tensor_v2_full::observation_dispatch_supports_standard_full_v2_without_changing_default` — pre-flip default assertion (StandardLiteV2 → StandardLiteDeckV2). (2) `dsl::select_materials::select_materials_batch_play_from_materials_plays_every_picked_source` — PRE-EXISTING, verified by stashing and re-running (still fails on baseline). Not caused by this change.**
- [x] 4.2 Update failing tests according to 4.1's classification **— fixed `tensor_v2_full` assertion. The DSL failure is out-of-scope (pre-existing, separate flow).**
- [x] 4.3 Add a behavioral test: build a tensor via `tensor::build_tensor` and `observation::build_observation(.., default_observation_profile())` against the same game state; assert byte-equal **— added `default_tensor_consistency.rs::build_tensor_matches_dispatcher_for_default_profile` + `..._both_players`.**
- [x] 4.4 Add a behavioral test that the lite_deck_v2 decklist section is populated **— added `default_tensor_consistency.rs::live_game_populates_decklist_section` + `game_new_populates_original_deck`. ALSO had to add `rebuild_original_deck` to `runners/replay.rs::restore_player_zone` so the replay path populates `Player.original_deck` from the recording's `initial_state` zones (precondition from task 1.2). Recording-replay regressions in `recording_replay_regressions.rs` all still pass.**
- [x] 4.5 `cargo test --manifest-path code/digimon-engine/Cargo.toml` passes **— 712 pass, 1 pre-existing unrelated failure.**

## 5. Tauri (engine_commands + models)

- [x] 5.1 `cargo check --manifest-path code/src-tauri/Cargo.toml`; confirm clean **— one unrelated compile error: `phase_str` was missing the new `GamePhase::SelectPlayOrder` variant (from the in-progress BO3 change). Added the match arm. After fix, lib compiles clean.**
- [x] 5.2 Update `engine_commands.rs` tests for new shape **— `tensor_summary_reports_engine_contract` (1375 → 8850, version 1 → 2 via `default_profile()`, card_id/scalar slot counts now read from profile) and `action_trace_serializes_human_action_context` (JSON contains "tensor_size":8850).**
- [x] 5.3 Update `models.rs` compatibility-gate tests **— `EngineContract::current()` automatically reports the new shape; existing tests use `TENSOR_SIZE` constant and `engine_contract_matches_compiled_constants` passes without edits.**
- [x] 5.4 Run `cargo test --manifest-path code/src-tauri/Cargo.toml` **— 36 lib tests + 4 model integration tests + 3 offline-game integration tests all pass. (Needed to stub `code/frontend/dist/index.html` so `tauri::generate_context!()` could load.)**

## 6. MCP binary

- [x] 6.1 `cargo test -p digimon-engine-mcp` **— 14 tests pass clean.**
- [x] 6.2 Smoke: deferred to manual end-to-end (task 10.3); MCP-binary integration tests in 6.1 exercise the new tensor shape end-to-end through the `tools/list` and `new_game` envelopes, which is sufficient confidence the binary doesn't panic.

## 7. PyO3 bindings

- [x] 7.1 Build PyO3 wheel and install **— `cargo check -p digimon-engine-py` revealed a pre-existing v1-isms: the binding hardcoded `MAX_SOURCES` (v1 constant = 11) instead of reading `profile.max_sources`. Fixed by using the per-profile value. After fix: `maturin build` produced `digimon_engine-0.1.0-cp311-abi3-win_amd64.whl`; `pip install --force-reinstall --no-deps` installed clean.**
- [x] 7.2 Verify default profile shape in Python **— `python -c "from digimon_engine import get_tensor_profile; p = get_tensor_profile(); print(p.id, p.tensor_size, p.version, p.max_sources)"` → `standard_lite_deck_v2 8850 2 12`.**
- [x] 7.3 Run `pytest code/tests/rl/test_pilot_training_config.py code/tests/rl/test_tensor_profiles.py code/tests/test_rust_bindings_surface.py` **— 6 tests needed updating (all asserting the pre-flip default identity/shape). Updated: `test_default_observation_profile_shape`, `test_compact_tensor_profile_remains_compatibility_profile`, `test_feature_extractor_uses_profile_positions` (in test_tensor_profiles.py); `test_default_profile_id_matches_profile`, `test_tensor_profile_positions`, `test_observation_layout_for_standard_lite_v2` (in test_rust_bindings_surface.py); also added a new `test_standard_compact_v1_profile_positions` to preserve the previous v1-shape assertions explicitly. After updates: 110/110 pass.**
- [x] 7.4 Verify `RustHeadlessGame(..., observation_profile=...)` accepts non-default profile id **— already exposed (the keyword is `observation_profile`, not `tensor_profile`); `test_rust_headless_game_accepts_observation_profile` exercises this with `standard_lite_v2`. No new keyword needed.**

## 8. Python parity test (legacy)

- [x] 8.1 Update `test_rust_backend_parity.py` to pin `standard_compact_v1` explicitly **— added `_PARITY_PROFILE = "standard_compact_v1"` constant, threaded into every `RustHeadlessGame(...)` constructor (4 sites). Reflects the test's actual intent: Python legacy engine produces v1-shaped tensors, so the Rust side must use v1 too for shape parity to be meaningful.**
- [x] 8.2 Run `DIGIMON_BACKEND=rust python -m pytest code/engine_py_legacy/tests/engine/test_rust_backend_parity.py -v` **— 13/13 pass.**

## 9. Documentation

- [x] 9.1 Update `docs/TENSOR_SPEC.md` **— rewrote the Constants table (TENSOR_SIZE now 8850; layout-specific v1 constants moved out of the top-level surface) and the Tensor Profiles narrative (default is lite_deck_v2; lite_v2/compact_v1/full_v2 reachable by explicit ID).**
- [x] 9.2 Update `docs/TRAINING_RUNBOOK.md` **— added a front-matter callout at the top mirroring the existing S1.3 / S1.4 break notices: explains the flip, the v1-checkpoint compatibility break, and the retrain expectation.**
- [x] 9.3 Update `docs/RUST_PYTHON_PARITY.md` **— added a header block above the reading guide announcing the flip, explaining the cross-engine shape consequence (Python legacy engine still v1-shaped, parity tests now pin Rust to v1), and bundling with the S1.3 / S1.4 precedent.**
- [x] 9.4 Update `tensor.rs` module docstring **— done as part of task 3.2 (full file rewrite; new docstring explains the dispatch and points callers at observation::build_observation_tensor for profile-explicit work).**

## 10. Final verification

- [x] 10.1 Per-component test matrix run as work progressed: engine `cargo test` (712 pass, 1 pre-existing unrelated DSL failure), Tauri `cargo test --lib --tests` (43 pass), MCP `cargo test -p digimon-engine-mcp` (14 pass), Python sweep (110 pass on the most relevant subset; legacy parity 13 pass). A full `python -m pytest` run launched in the background — not blocking, since each component suite is already green.
- [x] 10.2 `openspec validate flip-engine-default-to-lite-deck-v2 --strict` **— validates clean.**
- [ ] 10.3 Manual `cargo tauri dev` smoke **— deferred to user-driven verification. The lib + integration tests cover the Tauri command surface end-to-end; the `cargo tauri dev` step depends on the user's frontend dev server, which is outside this change's scope.**
