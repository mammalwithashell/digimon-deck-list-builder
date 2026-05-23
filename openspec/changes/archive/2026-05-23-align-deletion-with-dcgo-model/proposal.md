## Why

The Rust engine deletes permanents one at a time and fires `OnDeletion` while the carrier is still on the field, which (a) repeatedly trips the single-outstanding-invariant panic when a board-wipe deletes two permanents whose handlers both park selections (`G-DELETION-RESUME-NESTED`, ~21+ panics per generalist training run), and (b) forces post-trash keywords like `<Fortitude>` and `<Partition>` to use a special-case workaround slot (`pending_post_deletion_replays`) because the natural OnDeletion timing is wrong. DCGO's `DestroyPermanentsClass` solves both with a single architectural choice — batch the kill list, trash before draining triggers, attach pre-removal snapshots to the trigger context — and the team has decided to adopt that model verbatim.

## What Changes

- **NEW** `Game::delete_permanents_batch(handles, cause)` as the primary deletion entrypoint. Existing single-target `delete_permanent_with_cause(handle)` is reduced to a one-element-list shim.
- **NEW** Two-stage batched replacement cut-in (`WhenWouldLeaveBattleArea` → drain → `WhenWouldBeDeleted` → drain → re-filter). Substitutes (`<Decoy>`, `<Barrier>`) mutate the active batch's kill list rather than recursing into a fresh `delete_permanent_with_cause` call.
- **BREAKING** OnDeletion handlers fire *after* the carrier's top card has moved to trash. Today they fire while the carrier is still on field — the per-permanent finalize is deferred until after the handler's selection resolves. Card scripts and behavioral tests that read live carrier state via `ctx.game.player(handle.player).battle_area.get(handle.index)` inside OnDeletion bodies will no longer find the carrier there.
- **NEW** `DeletedObjectSnapshot` extended with pre-removal fields (`dp_just_before`, `level_just_before`, `cost_just_before`, `names_just_before`, `traits_just_before`, `source_count_just_before`, `digisources_just_before`). Snapshot is attached per-permanent to the batch and threaded into the OnDeletion / OnAnyDeletion / OnLeaveField trigger contexts.
- **NEW** `EffectContext` snapshot accessors (`deleted_self_dp`, `deleted_self_level`, `deleted_self_cost`, `deleted_self_names`, `deleted_self_traits`, `deleted_self_source_count`, `deleted_self_digisources`) for OnDeletion handlers that today read live state.
- **MODIFIED** `Keyword::Save` rewrite — body finds `self_card` in trash via the snapshot's `top_card` handle, lifts via `place_card_under_permanent_bottom`. Removes the live-`card_sources`-walk pattern.
- **MODIFIED** `Keyword::Fortitude` rewrite — gate reads `snapshot.source_count_just_before >= 2`, body plays `self_card` from trash via `play_from_trash_free_unsuspended`. No longer pushes to `pending_post_deletion_replays`.
- **MODIFIED** `Keyword::Partition` rewrite — pick N from `snapshot.digisources_just_before`, play them from trash. No longer pushes to `pending_post_deletion_replays`.
- **REMOVED** `Game::pending_post_deletion_replays` slot and its drain site in `finalize_permanent_deletion`. The pattern it modeled is now the natural OnDeletion timing.
- **MODIFIED** `Game::pending_deletion_resume: Option<...>` → `Vec<...>`. An OnDeletion handler running inside the batched drain that parks a selection still uses this slot; multiple permanents in the same batch can each independently park, so it must be a stack.
- **MODIFIED** Behavioral tests in `keyword_phase_d/` (Save, Save+Decoy, Partition, Save+Fortitude integration_smoke) — rewrite "carrier still on field while selection parked" assertions to "carrier already trashed; selection still parks."

**Out of scope (explicitly deferred):**

- Snapshot-on-`CardSource` (DCGO `PermanentJustBeforeRemoveField` cross-source reference identity). Wait for a card that needs the "we belonged to the same stack" predicate.
- Stack-ification of other single-outstanding slots (`parked_replacement`, etc.). Address when/if they panic in training.

## Capabilities

### New Capabilities

- `permanent-deletion-semantics`: Defines the batched, post-trash, snapshot-based contract for deleting permanents from the battle area. Covers the kill-list lifecycle (filter → mark → cut-in drain → re-filter → snapshot → trash → trigger drain), the two-stage replacement window, the OnDeletion-fires-post-trash invariant, the `DeletedObjectSnapshot` shape carried into trigger contexts, and the printed-keyword semantics (`<Save>`, `<Fortitude>`, `<Partition>`) that depend on it.

### Modified Capabilities

None. The "permanent deletion" contract is new — today's behavior is implicit in the engine, not specified.

## Impact

**Affected code (engine):**
- `code/digimon-engine/src/combat.rs` — `commit_permanent_deletion`, `delete_permanent_with_cause`, `delete_permanent_with_effects`, `finalize_permanent_deletion`, `resume_pending_deletion`. New batch entrypoint.
- `code/digimon-engine/src/replacement.rs` — `commit_permanent_deletion_no_replace`, `try_replace`. Batched two-stage cut-in.
- `code/digimon-engine/src/game.rs` — `pending_post_deletion_replays` field deleted; `pending_deletion_resume` becomes `Vec`. Constructor + accessors updated.
- `code/digimon-engine/src/trigger_context.rs` — `DeletedObjectSnapshot` struct extended.
- `code/digimon-engine/src/effect_context/mod.rs` — new snapshot accessors.
- `code/digimon-engine/src/cards/keyword_effects.rs` — `Keyword::Save`, `Keyword::Fortitude`, `Keyword::Partition` bodies rewritten.
- `code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs` — `DeleteBoundPermanents` step routes through the new batch API.

**Affected tests:**
- `code/digimon-engine/tests/keyword_phase_d/save.rs` — all four tests rewrite the "carrier still on field" assertions.
- `code/digimon-engine/tests/keyword_phase_d/partition.rs` — same pattern in `partition_plays_two_picked_sources_on_opponent_effect_deletion`.
- `code/digimon-engine/tests/keyword_phase_d/integration_smoke.rs::save_and_fortitude_compose_when_save_is_accepted` — no real card prints both keywords; rewrite as a unit-fixture composition test or delete.
- New regression tests under `tests/deletion_batching/` covering: multi-target battle tie, AoE Option deleting ≥2 `<Save>` permanents, mid-batch `<Decoy>` substitution, Fortitude-from-trash, Partition-from-trash.
- Survey across `tests/cards_behavioral/` for OnDeletion handlers reading live-handle state.

**Affected card scripts:**
- Any DSL or hand-rolled OnDeletion body that today reads `ctx.game.player(handle.player).battle_area.get(handle.index)` for "this Digimon's" state must shift to the `deleted_self_*` snapshot accessors. Sweep is targeted by the `keyword_effects.rs::Save` pattern grep.

**Documentation:**
- `docs/RUST_ENGINE_API.md` — new section on the deletion batch lifecycle + snapshot accessors.
- `docs/RUST_PYTHON_PARITY.md` — flag the OnDeletion-timing alignment with DCGO (was previously a known divergence).
- `qa/archetype-qa/engine-gaps.md` — close `G-DELETION-RESUME-NESTED`; remove the family-wide note's deletion-resume sibling entry.

**No impact on:**
- Hosted API (`code/server/`), RL training pipeline (`code/digimon_gym/`), frontend, Tauri desktop, PyO3 bindings (`code/digimon-engine-py/`).
- The deferred-drain infrastructure (`enter_deferred_drain` / `maybe_drain_effect_queue`) — already present; this change consumes it.

**Dependencies:**

None new. Builds on the deferred-drain infrastructure that landed in this branch.

**Out of scope (v1.x):**

- Snapshot-on-`CardSource` for cross-source identity predicates (DCGO's `PermanentJustBeforeRemoveField` ref-equality).
- Stack-ification of other single-outstanding `Option<T>` slots on `Game`.
