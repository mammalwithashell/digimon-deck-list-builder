## 1. Engine — soft-fail `trash_card_source`

- [x] 1.1 Change `EffectContext::trash_card_source` signature in [code/digimon-engine/src/effect_context/mod.rs:4074](code/digimon-engine/src/effect_context/mod.rs:4074) from `-> ()` to `-> bool`.
- [x] 1.2 Replace the `permanent not found` `.expect(...)` with a `match ... { Some(p) => p, None => return false }` early-exit (no `soft_remove`, no observer dispatch).
- [x] 1.3 Add an explicit guard `if permanent.card_sources.is_empty() { return false; }` BEFORE the `top_card()` call (per DCGO `HasNoDigivolutionCards` yield-break).
- [x] 1.4 Replace the `card not in this permanent's stack` `.expect(...)` with a `match ... { Some(p) => p, None => return false }` early-exit.
- [x] 1.5 At the end of the success path (after `soft_remove_if_emptied(perm)`), `return true`.
- [x] 1.6 ~~Optionally add a `tracing::debug!` on each soft-fail path~~ — **SKIPPED** (no `tracing` dep in `code/digimon-engine/Cargo.toml`; adding the dep is out of scope. Tracked as a follow-up under design Q3.)
- [x] 1.7 Update the doc comment to state the new bool semantics and link the DCGO reference (CardController.cs:5181).

## 2. Engine — update direct callers to discard the bool

- [x] 2.1 `code/digimon-engine/src/cards/keyword_effects.rs:326` (`<Fragment>` closure): `let _ = ctx.trash_card_source(subject, handle);`.
- [x] 2.2 `code/digimon-engine/src/effect_context/mod.rs:4206` (`trash_all_sources` loop): `let _ = self.trash_card_source(target, source);`.
- [x] 2.3 `code/digimon-engine/src/effect_context/mod.rs:5351` (`trash_top_n_digivolution_cards_of_each` loop): `let _ = self.trash_card_source(handle, source_card);`.
- [x] 2.4 Build clean: `cargo build --manifest-path code/digimon-engine/Cargo.toml`. Fix any other callers the compiler surfaces. **Result**: no other callers surfaced; build green in 1m 09s.

## 3. Engine — picker live revalidation in `install_source_multi_selection`

- [x] 3.1 In the pick callback at [code/digimon-engine/src/effect_context/selections.rs:2586](code/digimon-engine/src/effect_context/selections.rs:2586), after `find(...).copied().expect(...)` retrieves `source_ref`, add a "still present in live `card_sources`" check.
- [x] 3.2 If the live check fails: skip the `next_picked.push(source_ref)`; the recursive `install_source_multi_selection` call below runs with `next_picked` unchanged (== prior `picked_for_pick`) and refreshed candidates. Cleaner than a separate `clone()` branch — same recursive install handles both paths.
- [x] 3.3 Confirm `source_multi_candidates` re-enumerates from the live game state at re-install (already does — [selections.rs:2547](code/digimon-engine/src/effect_context/selections.rs:2547)); no further changes needed there.
- [x] 3.4 Verify the empty-candidates branch ([selections.rs:2548](code/digimon-engine/src/effect_context/selections.rs:2548)) handles `picked.len() < min` correctly when re-install lands with no fresh candidates — **confirmed in source**: `if candidates.is_empty() || picked.len() == max { if picked.len() >= min { final_callback(...); } return; }` covers both branches (final-callback with prior picks OR clean no-op). Picker test in task 6.4 exercises both paths.

## 4. DSL — `TrashSelectedSources` and `TrashUnionBound` consume the bool

- [x] 4.1 [code/digimon-engine/src/dsl_cards/step/zone_moves.rs:211](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:211) (`CompiledStep::TrashSelectedSources`): wrap the `ctx.trash_card_source(...)` call in `let _ = ...`. No other change.
- [x] 4.2 [code/digimon-engine/src/dsl_cards/step/zone_moves.rs:315](code/digimon-engine/src/dsl_cards/step/zone_moves.rs:315) (`CompiledStep::TrashUnionBound`, the `UnionZoneOrigin::Material` arm): wrap `ctx.trash_card_source(carrier, card)` in `let _ = ...`. (Line number drifted from spec — actual site at :315.)
- [x] 4.3 Confirm `cargo build --manifest-path code/digimon-engine/Cargo.toml` is still clean. **Result**: green in 25.07s.

## 5. Tests — invert existing assertions

- [x] 5.1 Open [code/digimon-engine/tests/effect_context/trash_card_source.rs](code/digimon-engine/tests/effect_context/trash_card_source.rs); identify every test that triggers the panic on stale handle / missing carrier / empty stack. **Result**: no existing `#[should_panic]` tests in this file — only 3 happy-path tests. Tasks 5.2 was a no-op.
- [x] 5.2 ~~Convert each such test from `#[should_panic(...)]`...~~ — **N/A**, no such tests existed.
- [x] 5.3 Verify the existing "happy path" tests still pass by adding `assert!(ctx.trash_card_source(perm, card));` — added the bool assertion to all 3 existing happy-path tests (`trash_card_source_removes_mid_stack_card`, `trash_card_source_removes_bottom_card`, `trash_card_source_emptying_carrier_removes_slot`).
- [x] 5.4 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test effect_context` — **9/9 tests pass**, including the 3 new soft-fail tests from group 6.

## 6. Tests — new regression coverage

- [x] 6.1 Add unit test `trash_card_source_returns_false_on_stale_card_handle` to `code/digimon-engine/tests/effect_context/trash_card_source.rs` — **PASS**.
- [x] 6.2 Add unit test `trash_card_source_returns_false_on_missing_carrier` — **PASS**. Used direct `battle_area.clear()` to simulate carrier removal (simpler than `delete_permanents_batch` and exercises the same `battle_area.get(idx).is_none()` codepath).
- [x] 6.3 Add unit test `trash_card_source_returns_false_on_empty_stack` — **PASS**. Verifies `top_card()` is not called on empty stack.
- [x] 6.4 Add picker unit test in `code/digimon-engine/tests/selection/source_multi.rs` — **split into two tests** (`source_multi_picker_re_installs_on_stale_pick_min_one_no_op` and `source_multi_picker_re_installs_on_stale_pick_min_zero_finalizes_empty`) to exercise both branches of the empty-candidates path (no-op vs final-callback-with-empty). Both **PASS**.

## 7. Tests — recording-replay regression

- [x] 7.1 Extract the inner `recording` object from the captured crash. **Done** — fixture is `code/digimon-engine/tests/recordings/g_dsl_trash_sources_stale_handle.json` (17 KB, 87 actions).
- [x] 7.2 Commit the fixture under `code/digimon-engine/tests/recordings/g_dsl_trash_sources_stale_handle.json`. **Done** — 17 KB, tree-friendly.
- [x] 7.3 Add an integration test `replay_g_dsl_trash_sources_stale_handle_does_not_panic`. **Done** at [code/digimon-engine/tests/recording_replay_regressions.rs](code/digimon-engine/tests/recording_replay_regressions.rs) (flat-file layout per project convention; `tests/runners/` subdir does not exist). Uses `ReplayRunner::new(recording, &db, false).seek(total)`.
- [x] 7.4 Run `cargo test --manifest-path code/digimon-engine/Cargo.toml --test recording_replay_regressions` — **PASS** in 0.97s. The exact scenario that produced the panic in production training (game 9728) now seeks to the end cleanly.

## 8. Family registry

- [x] 8.1 Added the `G-DSL-TRASH-SOURCES-STALE-HANDLE` entry to [qa/archetype-qa/panic-families.json](qa/archetype-qa/panic-families.json) — set directly to `status: resolved` since the fix landed in this same change. `resolved_by: openspec/changes/fix-trash-card-source-stale-handle`.
- [x] 8.2 Added §DSL Trash-Selected-Sources Stale Handle prose entry to [qa/archetype-qa/engine-gaps.md](qa/archetype-qa/engine-gaps.md) — covers first-seen context, sibling-class link, root cause, fix landed, regression tests, and out-of-scope follow-ups. Cross-references DCGO source citations.

## 9. Verification

- [x] 9.1 Full engine test suite: `cargo test --manifest-path code/digimon-engine/Cargo.toml` — **3391 pass, 3 pre-existing failures** (`bt24_008_on_play_decline_does_not_trash_or_draw`, `ex9_024_decline_discard_does_not_return_trash_card`, `st19_04_on_play_decline_does_not_trash_or_draw`). Verified pre-existing by `git stash && cargo test ...` — same 3/3 failures on baseline. Unrelated to this change (optional-cost-decline DSL gap, separate engine bug filed as chip).
- [x] 9.2 Tauri-layer regression: `cargo test --manifest-path code/src-tauri/Cargo.toml` — **pre-existing build failure** (`missing field 'also_treated_as' in CardData` at `engine_commands.rs:302`). Verified pre-existing by `git stash` — same failure on baseline. Engine/Tauri schema drift, unrelated to this change. Filed as chip.
- [ ] 9.3 PyO3 build (`maturin develop`) — **DEFERRED**: requires `maturin` in PATH + Python env setup that's not provisioned in this worktree. Out-of-scope for the engine-only change; the PyO3 binding doesn't expose `trash_card_source` directly and the existing `RustHeadlessGame` shim consumes the engine via the unchanged `step`/`submit` surface.
- [ ] 9.4 Python-side parity test — **DEFERRED**: requires the same env setup as 9.3. No behavior change expected since the panicking path was unreachable via Python before the fix (it would crash with `PanicException`, not run wrong).
- [x] 9.5 CLI replay smoke: built `digimon-engine-cli`, replayed fixture `code/digimon-engine/tests/recordings/g_dsl_trash_sources_stale_handle.json --step 87` — **clean exit**, emits state JSON (event_seq=14, memory=6, ...). Pre-fix this same command panicked at the same step.
- [ ] 9.6 Short pilot training (~10k steps) — **DEFERRED to user**: requires the training CLI's full env, deck pool snapshot, and a non-trivial amount of compute/wall-time. Suggested follow-up command from the change description: `python -m digimon_gym.agents.pilot_training --timesteps 10000 --deck-pool-snapshot models/generalist_1m_v2/deck_pool_snapshot.json` (or equivalent). The replay regression test in 7.4 is the equivalent deterministic check for this specific panic.
- [x] 9.7 OpenSpec validation: `openspec validate fix-trash-card-source-stale-handle` → **"Change 'fix-trash-card-source-stale-handle' is valid"**.
- [x] 9.8 Panic-families entry already created with `status: resolved`, `resolved_at: 2026-05-24`, `resolved_by: openspec/changes/fix-trash-card-source-stale-handle`. When this change lands as a PR, update `resolved_by` to `PR #<N>` in a follow-up commit.
