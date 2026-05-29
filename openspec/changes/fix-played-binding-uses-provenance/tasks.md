## 1. Engine survey + provenance audit

- [x] 1.1 Read `code/digimon-engine/src/trigger_context.rs`. **Result**: `ProvenanceToken` is a trivial `From<CardHandle>` cast — no separate minting needed.
- [x] 1.2 Confirmed play primitives return `PermanentHandle`; played top-card handle is reachable via `permanent.top_card().handle()`.
- [x] 1.3 Audit `code/digimon-engine/cards/**/*.yaml`. **Result**: Only BT16-085 combines `bind_as` on a play verb with `schedule_delayed` in the current codebase. Audit comment included in `play_digivolve.rs`.

## 2. `BindingValue::PlayedPermanent` variant

- [x] 2.1 Added `BindingValue::PlayedPermanent { token, fallback }` in `bindings.rs`.
- [x] 2.2 Added `insert_played_permanent` + diagnostic `get_played_permanent`. No public handle-only getter.
- [x] 2.3 Round-trip unit tests `played_permanent_round_trip` and `played_permanent_clone_isolation`. PASS.

## 3. Strict resolver

- [x] 3.1 Added `Game::resolve_token_as_battle_area_top(token)` in `game.rs`.
- [x] 3.2 `resolve_binding_ref` intercepts `PlayedPermanent` and routes through the strict helper.
- [x] 3.3 Added `resolve_played_permanent_permissive` in `binding_ref.rs` for `ScheduleDeletePlayedAtTurnEnd`.
- [x] 3.4 `resolve_named` handles the new variant (returns `None`; production routes through the upstream intercept).
- [x] 3.5 Unit tests for `resolve_token_as_battle_area_top` (4 cases). PASS.

## 4. Bind sites — switch `bind_as` from positional to provenance

- [x] 4.1 Added `bind_played_with_provenance` helper in `play_digivolve.rs` + audit comment.
- [x] 4.2 `CompiledStep::PlayFromHandFree` switched.
- [x] 4.3 `CompiledStep::PlayFromRevealedFree` switched (both Card and CardList branches).
- [x] 4.4 `CompiledStep::PlayUnionBoundFree` switched.
- [x] 4.5 `CompiledStep::PlayFromMaterials` switched (all 3 branches).
- [x] 4.6 `CompiledStep::PlayToken` switched.
- [x] 4.7 `CompiledStep::ScheduleDeletePlayedAtTurnEnd` uses `resolve_played_permanent_permissive`.

## 5. Regression tests — BT16-085 Davis & Ken

- [x] 5.1 `bt16_085_dna_into_paildramon_skips_scheduled_return` — PASS.
- [x] 5.2 `bt16_085_regular_digivolve_skips_scheduled_return` — PASS.
- [x] 5.3 `bt16_085_played_digimon_deleted_skips_scheduled_return` — PASS.
- [x] 5.4 Existing happy-path `bt16_085_start_of_main_played_digimon_returns_at_opponent_eot` — PASS.

## 6. Existing test fixups

- [x] 6.1 `tests/dsl/play_token_bind_as.rs` — switched to `get_played_permanent`.
- [x] 6.2 `tests/dsl/phase2f1_play_steps.rs` `play_from_revealed_free_step_consumes_reveal_and_keeps_memory` — switched to `get_played_permanent`.

## 7. Documentation

- [x] 7.1 New section in `docs/RUST_ENGINE_API.md` documenting strict-vs-permissive resolver contract.

## 8. Full verification

- [x] 8.1 `cargo build --manifest-path code/digimon-engine/Cargo.toml` — clean.
- [x] 8.2 `cargo test --manifest-path code/digimon-engine/Cargo.toml --lib` — 2 new bindings tests + 4 new resolver tests PASS.
- [x] 8.3 BT16-085 behavioral suite — 30/30 PASS (27 existing + 3 new).
- [x] 8.4 EX11-022 / EX11-061 — PASS (permissive resolver path verified for Karakurumon, Mirai Kinosaki).
- [x] 8.5 Full `cargo test` run — only pre-existing failures remain. Baseline (`git stash`) confirmed identical 10 pre-existing failures: `ex7_030`, `p_134`, `p_197`, `select_materials_batch_*`, `step_variants_have_exec_arms`, `headless_runner` profile (×2), `behavioral_end_to_end::permutation_then_opponent_union_zone_tech_flow`, `opponent_permanent::mask_emits_only_valid_targets_plus_pass`, `opponent_permanent::tensor_reports_valid_count_and_selecting_player`. Zero NEW failures introduced.
