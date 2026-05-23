## Context

The Rust engine currently deletes permanents one at a time. `Game::delete_permanent_with_cause(handle, cause)` ([combat.rs:3214](../../../code/digimon-engine/src/combat.rs:3214)) is the only entrypoint: it runs the replacement window per-permanent, then `commit_permanent_deletion` synchronously enqueues `OnDeletion` and drains. If the drain parks a player selection (Save's Tamer-pick), the rest of the deletion is stashed in `Game::pending_deletion_resume: Option<...>` and finalized when the selection resolves.

This shape is the source of three accumulating problems:

1. **Nested-park panic (`G-DELETION-RESUME-NESTED`).** A board-wipe Option that deletes two `<Save>` permanents calls `delete_permanent_with_cause` twice in sequence; the second call enters `commit_permanent_deletion` while the first's `pending_deletion_resume` is still occupied, tripping the `debug_assert!` at [replacement.rs:1382](../../../code/digimon-engine/src/replacement.rs:1382). Observed at ~21+ panics per generalist training run. The Python-side crash-resilience wrapper catches them, but each loses ~one game's training samples.
2. **Post-trash workaround slot.** `<Fortitude>` and `<Partition>` semantically fire post-trash (DCGO `IsExistOnTrash` predicate). Rust today fires OnDeletion pre-trash, so these keywords stash a deferred replay in `Game::pending_post_deletion_replays: Vec<...>` ([game.rs:531-563](../../../code/digimon-engine/src/game.rs:531)) which `finalize_permanent_deletion` drains after `delete_permanent` moves the carrier to trash. The slot exists because the natural OnDeletion timing is wrong; any future post-trash keyword needs its own bespoke channel.
3. **Per-permanent replacement recursion.** A `<Decoy>` redirecting deletion to self goes through `delete_permanent_with_cause(substituted_handle, cause)` recursively ([combat.rs:3290](../../../code/digimon-engine/src/combat.rs:3290)), depth-guarded. DCGO instead mutates the active batch's kill list during the cut-in; the recursion is a Rust-only complication.

The recently-landed deferred-drain infrastructure (`Game::draining_deferred: u32`, `enter_deferred_drain` / `exit_deferred_drain_and_flush` / `maybe_drain_effect_queue` at [effect_queue.rs:637-672](../../../code/digimon-engine/src/effect_queue.rs:637)) — adopted to fix the sibling bug `G-DSL-OUTER-TAIL-NESTED-PARK` — provides the substrate this change consumes. The pattern is already proven against the same kind of nested-park problem.

DCGO's `DestroyPermanentsClass.Destroy()` ([CardController.cs:3692-3899](../../../DCGO/Assets/Scripts/Script/CardController.cs:3692)) is the reference. It runs a 10-step batched flow:

```
1. Filter kill list (CanBeDestroyedBySkill, CanNotBeAffected)
2. willBeRemoveField = true on all survivors
3. autoProcessing_CutIn.StackSkillInfos(WhenPermanentWouldBeDeleted)
4. autoProcessing_CutIn.StackSkillInfos(WhenRemoveField)
5. autoProcessing_CutIn.TriggeredSkillProcess()                ← cut-in drain
6. Re-filter (willBeRemoveField may have been cleared by cancel)
7. Snapshot DP/Level/Cost/Names/Traits + PermanentJustBeforeRemoveField
8. autoProcessing.StackSkillInfos(OnDestroyedAnyone)           ← NOT drained
9. autoProcessing.StackSkillInfos(OnLeaveFieldAnyone)          ← NOT drained
10. For each survivor: DiscardEvoRoots, RemoveField, AddTrashCard(topCard)
    — clear willBeRemoveField
    — exit; outer autoProcessing later drains OnDestroyedAnyone
```

Three architectural properties matter:

- **Trash before triggers.** Steps 7-10 put the card in trash before the OnDeletion equivalent (`OnDestroyedAnyone`) drains. Save's predicate `IsTopCardInTrashOnDeletion` ([OnDeletion.cs:144](../../../DCGO/Assets/Scripts/Script/CardEffectCommons/CanUseEffects/OnDeletion.cs:144)) literally requires this.
- **Snapshot the dying state.** Step 7 captures DP, level, cost, names, traits before the trash move. Each `CardSource` under the dying perm gets `PermanentJustBeforeRemoveField = permanent` so handlers can answer "what was my predecessor."
- **Batch is the unit.** The kill list is filtered once at step 1, and substitutions during cut-in (Decoy, Barrier) mutate that list rather than spawning recursive deletions.

The user's explicit decision (explore §10): mimic DCGO. Do not Arc-wrap permanents — the snapshot struct provides the needed predicate semantics. Defer cross-source `PermanentJustBeforeRemoveField` reference identity for now (no card in the current pool needs it).

## Goals / Non-Goals

**Goals:**

- Close `G-DELETION-RESUME-NESTED` without a stack-the-slot stop-gap. The panic is a symptom; the architecture is the fix.
- Match DCGO's printed-rules-correct deletion semantics: batched kill list, two-stage replacement cut-in, trash before OnDeletion drain, snapshot-based predicates.
- Retire `pending_post_deletion_replays` as a workaround slot. Fortitude/Partition become normal post-trash OnDeletion handlers.
- Preserve all already-correct game outcomes. Behavioral tests that pin DCGO-faithful end states keep passing; only tests that pin Rust-internal mid-deletion implementation details get rewritten.
- Consume the existing deferred-drain infrastructure; do not add a parallel substrate.

**Non-Goals:**

- **Arc-wrap `Permanent`.** Out. Snapshot semantics on a struct match DCGO predicate behavior; ownership-model parity is not the goal.
- **Cross-source `PermanentJustBeforeRemoveField` reference identity.** Out. No current card uses the "we belonged to the same stack" predicate. Add when the first card needs it.
- **Stack-ifying `parked_replacement` and other Option-shaped slots on Game.** Out. Address when/if they panic.
- **Save+Fortitude composition semantics on a single card.** Out. No printed card carries both keywords; the `integration_smoke.rs:409` test that pins the current race outcome will be either deleted or rewritten as a unit-fixture composition test.
- **Python-engine parity.** The Python engine is sunset; this change is Rust-only.
- **Behavioral change to non-deletion zone moves (return-to-hand, return-to-deck).** Out. The two-stage cut-in batching is deletion-specific in v1.

## Decisions

### D1. Snapshot, not Arc, for post-removal identity

**Choice:** Extend `crate::trigger_context::DeletedObjectSnapshot` (already exists at [combat.rs:3429](../../../code/digimon-engine/src/combat.rs:3429)) with pre-removal fields. Attach the snapshot to the per-permanent record inside the batch, thread into the `TriggerContext` for `OnDeletion`/`OnAnyDeletion`/`OnLeaveField`.

```rust
pub struct DeletedObjectSnapshot {
    // existing
    pub former_controller: PlayerId,
    pub top_card: CardHandle,
    pub card_kind: CardKind,
    pub traits: Vec<String>,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub cause: EventCause,
    // added (DCGO PermanentJustBeforeRemoveField-equivalent state)
    pub dp_just_before: Option<i32>,
    pub level_just_before: Option<u8>,
    pub cost_just_before: Option<u8>,
    pub names_just_before: Vec<String>,
    pub traits_just_before: Vec<String>,
    pub source_count_just_before: usize,
    pub digisources_just_before: Vec<CardHandle>,
}
```

**Why:**
- DCGO's `PermanentJustBeforeRemoveField` is a managed-reference identity that survives field removal because C# heap allocations are GC-tracked. Rust's `Permanent` is owned inline in `Vec<Permanent>` and gone after `Vec::remove`. The only way to preserve identity is either Arc-wrap (deep ownership refactor) or capture-by-value (snapshot).
- All DCGO predicates that reach through `PermanentJustBeforeRemoveField` only consult primitive state (DP, level, cost, names, traits) plus the digi-source list. Snapshot covers every observed predicate in the current card pool.
- The snapshot struct already exists; this is incremental.

**Alternatives considered:**
- `Arc<Permanent>` — deeper refactor. Many accessor changes. Defeats the "consume existing infra" goal.
- Snapshot inside a side-table on `Game` — extra indirection without benefit. The trigger context already carries a `DeletedObjectSnapshot`; just extend it.

**Risk:** A future card needing the cross-source "did we belong to the same stack" predicate will need either a `BatchId` per snapshot or a new structure. Captured in Non-Goals; revisit when a card forces the question.

### D2. `delete_permanents_batch` as the primary deletion API

**Choice:** Add `Game::delete_permanents_batch(handles: Vec<PermanentHandle>, cause: ReplacementCause) -> DeletionBatchOutcome`. Reduce `Game::delete_permanent_with_cause(handle, cause)` to `self.delete_permanents_batch(vec![handle], cause)`. `delete_permanent_with_effects(handle)` infers the cause and shims through the same path.

```rust
pub struct DeletionBatchOutcome {
    pub completed: Vec<PermanentHandle>,   // were trashed
    pub cancelled: Vec<PermanentHandle>,   // replaced (e.g. by Decoy redirect)
    pub substituted_in: Vec<PermanentHandle>,  // added mid-batch via Decoy
}
```

**Why:**
- DCGO's `DestroyPermanentsClass` takes a list. Single-target is a special case of batch.
- Battle tie at [combat.rs:3146-3163](../../../code/digimon-engine/src/combat.rs:3146) is already manually sequenced (delete defender, then attacker if handle still valid). Becomes `delete_permanents_batch([defender, attacker])`.
- DSL `DeleteBoundPermanents` ([permanent_mutations.rs:34-52](../../../code/digimon-engine/src/dsl_cards/step/permanent_mutations.rs:34)) currently iterates with index-shift-avoidance reverse-sorting. Becomes a single batch call.
- Effects that delete a single permanent keep working with no script change (shimmed through `delete_permanent_with_cause(handle, cause)`).

**Alternatives considered:**
- Keep single-target primary, add `delete_permanents_batch` as a sibling. Two coexisting models — the historical source of the bugs this change is fixing.

### D3. Two-stage batched replacement cut-in; substitutes mutate the active batch

**Choice:** Inside `delete_permanents_batch`:

```
enter_deferred_drain();
  // Stage 1
  for h in kill_list: enqueue WhenWouldLeaveBattleArea(h)
  flush via exit_deferred_drain_and_flush() — but stay inside an outer scope
  re-filter kill_list against batch.cancelled
  // Stage 2
  for h in surviving: enqueue WhenWouldBeDeleted(h)
  flush again
  re-filter
  // Snapshot
  for h in surviving: build DeletedObjectSnapshot { …just_before fields }
  // Trash
  for h in surviving: linked-card cascade, DiscardEvoRoots, RemoveField, AddTrashCard
  // OnDeletion + OnAnyDeletion + OnLeaveField drain
  for h in survived: enqueue OnDeletion (with snapshot in TriggerContext)
  enqueue OnAnyDeletion (global, per pid) with each snapshot
  enqueue OnLeaveField with each snapshot
exit_deferred_drain_and_flush();
```

When a replacement handler in stage 1 or 2 calls `rctx.substitute(other_handle)`, the substitute is appended to the active batch's kill list. The `ReplacementOutcome::Substituted` arm at [combat.rs:3286-3291](../../../code/digimon-engine/src/combat.rs:3286) is replaced by an "append to active batch" hook on `Game` — the original `delete_permanent_with_cause(source_h, cause)` recursion goes away.

**Why:**
- Matches DCGO's two-stage `WhenPermanentWouldBeDeleted` → `WhenRemoveField` cut-in.
- Eliminates the depth-guarded recursion at [combat.rs:3290](../../../code/digimon-engine/src/combat.rs:3290).
- The deferred-drain scope is already proven against nested-park. The new "active batch" slot on Game is a single `Option<DeletionBatch>` (in v1 — batches don't nest across distinct `delete_permanents_batch` calls; an OnDeletion handler that calls `delete_permanents_batch` recursively starts a fresh batch, which is fine because by that point we're past stage 1/2 and snapshots have been taken).

**Alternatives considered:**
- Single-stage cut-in (collapse WhenWouldLeaveBattleArea + WhenWouldBeDeleted). Diverges from DCGO; would lose the distinction that some replacements only fire on "leaving by any route" vs "specifically deletion." Today's engine already distinguishes them.
- Per-permanent cut-in inside a batched outer scope. Hybrid; gets the snapshot/trash semantics but doesn't match DCGO's stack-and-drain shape for the replacement window.

**Risk:** Mid-batch substitution semantics — does the substitute also go through stage 1 cut-in, or only stage 2? DCGO: substitutes are added to the kill list at any time during the cut-in, and the *next* `StackSkillInfos` pass picks them up. Resolution: in Rust, substitutions during stage 1 get queued for stage 1 re-pass before moving to stage 2; substitutions during stage 2 join the stage 2 pass. Bounded by a depth guard (same shape as today's) to catch pathological loops.

### D4. OnDeletion fires post-trash; `pending_post_deletion_replays` retired

**Choice:** Move the `Player::delete_permanent` + linked-card cascade + AddTrashCard step *before* the OnDeletion drain inside `delete_permanents_batch`. Delete the `Game::pending_post_deletion_replays` field, its drain in `finalize_permanent_deletion`, and the push sites in `Keyword::Fortitude`/`Keyword::Partition`.

**Why:**
- Trash-before-drain is the DCGO architectural choice. Save's `IsTopCardInTrashOnDeletion` predicate is the cleanest evidence.
- Removing `pending_post_deletion_replays` retires a workaround channel that exists *only* because the previous timing was wrong. Future post-trash keywords no longer need a bespoke side-channel.
- `OnAnyDeletion`'s global broadcast becomes simpler — it scans surviving battle-area permanents *plus* uses each snapshot's `top_card` as event context. The "deleted carrier missed by `TriggerSource::PlayerBattleArea` scan" problem documented at [game.rs:542-548](../../../code/digimon-engine/src/game.rs:542) is gone: the OnDeletion drain at end-of-batch fires for each snapshot directly.

**Alternatives considered:**
- Keep `pending_post_deletion_replays` as a generic "work to flush at finalize-time" hook. Rejected — the slot only exists because of the timing bug; removing the bug removes the need.

### D5. `pending_deletion_resume` stays, becomes `Vec<...>`

**Choice:** Convert `Game::pending_deletion_resume: Option<(PermanentHandle, Option<CardHandle>)>` → `Vec<(PermanentHandle, Option<CardHandle>)>`. Push on OnDeletion-handler park; pop LIFO on selection resolve via `resume_pending_deletion`.

**Why:**
- Even under the DCGO batched model, an OnDeletion handler can still park a selection (Save's optional Tamer-pick). If two `<Save>` permanents in the same batch both park, the second push must succeed.
- Single-slot was already documented as "if this assumption ever breaks, replace with a stack" at [game.rs:514-518](../../../code/digimon-engine/src/game.rs:514). It broke.
- Stack depth is bounded by the kill-list size (each permanent parks at most once per batch). Sanity cap matches the field-slot count (10).

**Alternatives considered:**
- Delete the slot entirely; replace with batch-level "for handler in handlers: handle inline." Rejected — the parked-selection-then-resume pattern is load-bearing; can't synchronously call the OnDeletion handler closures while waiting on player input.

### D6. Snapshot accessors on `EffectContext`

**Choice:** Add to `EffectContext`:

```rust
pub fn deleted_self_dp(&self) -> Option<i32>;
pub fn deleted_self_level(&self) -> Option<u8>;
pub fn deleted_self_cost(&self) -> Option<u8>;
pub fn deleted_self_names(&self) -> &[String];
pub fn deleted_self_traits(&self) -> &[String];
pub fn deleted_self_source_count(&self) -> usize;
pub fn deleted_self_digisources(&self) -> &[CardHandle];
```

Each reads from the current `TriggerContext`'s `deleted_object` snapshot.

**Why:**
- OnDeletion handlers that today read live state via `ctx.game.player(handle.player).battle_area.get(handle.index)` need a migration target. Snapshot accessors are the obvious shape.
- DSL `event_target_*` predicates already read snapshot fields ([engine-gaps.md:187](../../../qa/archetype-qa/engine-gaps.md:187)); this extends the same idea to "this Digimon."

**Alternatives considered:**
- Force handlers to walk trash for the top card and reconstruct DP/level/etc. by reading the card data. Rejected — modifier-aware DP is non-trivial; the snapshot is the right cache.

### D7. Test-rewrite scope: targeted, not exhaustive

**Choice:** Rewrite only the four named tests in `keyword_phase_d/` plus a survey across `tests/cards_behavioral/` for OnDeletion handlers reading live state. New regression coverage under `tests/deletion_batching/`.

**Why:**
- The four named tests pin specific implementation-tied assertions ("carrier still on field while parked"). Direct rewrite to "carrier already trashed; selection still parks."
- Most card-behavioral tests don't care about the carrier's mid-deletion location — they pin end states (carrier in trash, target gone, memory delta). Those tests should pass unchanged.
- A grep-driven survey across `cards_behavioral/` covers DSL-authored OnDeletion bodies that might read live state.

**Alternatives considered:**
- Survey-every-test rewrite. Rejected — high effort, mostly redundant. The four named tests are the visible-canary; the survey catches outliers.

## Risks / Trade-offs

- **[Risk]** OnDeletion handlers across hand-rolled cards may read live carrier state without going through `EffectContext`. → Mitigation: grep `cards/` for `battle_area.get(handle.index)` and `ctx.source_permanent`-paired live reads; migrate to snapshot accessors. Survey is part of Phase 1.

- **[Risk]** Substituting into a batch during stage 1 vs stage 2 has subtle ordering — the substituted handle didn't go through stage 1's `WhenWouldLeaveBattleArea` if substituted during stage 2. → Mitigation: design choice (D3) requeues stage 1 for substitutes-during-stage-1, runs only stage 2 for substitutes-during-stage-2 (since the substitute was *added* in stage 2 and is being deleted *as part of* a deletion path, not a leave-field path). Matches DCGO ordering. Behavioral test covers the case.

- **[Risk]** Recordings produced by training reference action IDs computed against today's pre-trash OnDeletion timing. If a recording's Save selection was parked at turn N before the trash move, replaying against the new engine might encounter a different action set at the same step. → Mitigation: replay-mode `LiveGame` verification (commit `eec791dd`) catches divergence. Existing recordings older than this change are forensic artifacts only; they're not regression fixtures. Quarantine if needed.

- **[Risk]** `OnAnyDeletion` global broadcast logic moves from `finalize_permanent_deletion` into the batched flow. → Mitigation: Existing observers (Puppet self-refire on `OnAnyDeletion` — see [engine-gaps.md:187](../../../qa/archetype-qa/engine-gaps.md:187)) operate on the snapshot, which is now richer. End-state assertions in those tests pass.

- **[Risk]** Replacement-batch substitution introduces a "kill list mutation during cut-in" path that's harder to reason about than today's recursion. → Mitigation: depth guard preserved (matches today's at [combat.rs:3289](../../../code/digimon-engine/src/combat.rs:3289)); active-batch slot is `Option<DeletionBatch>` with explicit "started" / "stage" markers; a behavioral test specifically covers `<Decoy>` substituting into a 2-permanent batch.

- **[Trade-off]** Snapshot accessors duplicate state at deletion time. → Acceptable: the alternative (Arc-wrap permanents) is materially more invasive, and the snapshot fields are small (DP/level/cost/names/traits = ~50 bytes per dying permanent).

- **[Trade-off]** Card scripts that today read "this Digimon's DP at deletion time" via the live handle need migration to `ctx.deleted_self_dp()`. The compiler enforces this when the live read becomes a `None` lookup, but DSL-authored OnDeletion bodies may need scan + rewrite. → Acceptable: the Phase 1 survey is bounded.

## Migration Plan

This is an in-tree refactor — no migration outside the engine crate, no schema changes, no API versioning. Phases:

1. **Phase 0 — substrate.** Extend `DeletedObjectSnapshot`. Add `EffectContext::deleted_self_*` accessors. Add `Game::active_deletion_batch: Option<DeletionBatch>` slot. Convert `pending_deletion_resume` to `Vec<...>`. No behavioral change yet — accessors return `None` until Phase 1 builds them.
2. **Phase 1 — batch entrypoint.** Add `delete_permanents_batch`. Implement the 10-step DCGO flow with deferred-drain wrapper. Wire `delete_permanent_with_cause` and `delete_permanent_with_effects` as one-element shims. Survey `cards/` for OnDeletion handlers reading live carrier state; migrate to snapshot accessors.
3. **Phase 2 — replacement window.** Rewrite `delete_permanent_with_cause`'s replacement-window logic into two-stage batched cut-in inside `delete_permanents_batch`. Replace the `ReplacementOutcome::Substituted` recursion with an active-batch-append hook.
4. **Phase 3 — keyword rewrites.** Rewrite `Keyword::Save`, `Keyword::Fortitude`, `Keyword::Partition` to read from snapshot + trash. Remove `pending_post_deletion_replays` field and its drain site.
5. **Phase 4 — test churn.** Rewrite the four named `keyword_phase_d/` tests. Add `tests/deletion_batching/` for new coverage. Survey + spot-fix any `cards_behavioral/` test that pinned mid-deletion state.
6. **Phase 5 — close out.** Close `G-DELETION-RESUME-NESTED` in `engine-gaps.md`. Document the batch lifecycle in `docs/RUST_ENGINE_API.md`. Verify generalist training run produces zero deletion-resume panics.

Rollback: each phase is reviewable / revertable independently. Phase 1 alone is enough to close the panic if Phases 2-3 need more bake time.

## Open Questions

None remaining for v1 scope. Q1-Q7 from the explore document were closed (see explore §10). Two questions to revisit *if* the implementation turns up surprises:

- **OQ-A.** Does the active-batch slot need to be a `Vec<DeletionBatch>` if an OnDeletion handler inside the batched drain itself starts a new `delete_permanents_batch`? Initial design (D3) says no — nested batches are a fresh top-level call, and the inner call's stages run to completion before the outer batch's OnDeletion drain continues (deferred-drain scope absorbs the queueing). Verify with a test in Phase 1.
- **OQ-B.** Will any existing `OnAnyDeletion` observer regress because the broadcast moved? The Puppet self-refire test and the `BT22-*` deletion-context tests should catch this. If any do, the snapshot threading may need extra fields.
