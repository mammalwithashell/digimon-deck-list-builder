# Q19 — Defer the [On Deletion] bundle past the causing effect (G-ON-DELETION-RESOLVES-MID-EFFECT)

**Goal:** flip judge-quiz Q19 (`tests/judge_quiz/d_activation_site.rs::q19_on_deletion_suppressed_when_returned_to_hand`) from BLOCKED → PASS without regressing the 3894-test deletion path. Judge answer: **0 draws** (Calling From the Darkness deletes the Eyesmon stack, then returns Eyesmon to hand → the top-most card leaves trash → the whole `[On Deletion]` bundle is suppressed). Engine currently draws **6** (bundle resolves nested inside the delete step, before the return).

**Keep green:** Q20 (`q20_all_on_deletion_fire_when_eyesmon_stays_in_trash`) = **8** (Eyesmon stays in trash → all fire). This is the discriminator: the fix must NOT just suppress everything.

## Root cause (confirmed 2026-06-04)

DCGO `CardEffectCommons.CanActivateOnDeletion` (`DCGO/.../CardEffectCommons/CanUseEffects/OnDeletion.cs:113`) gates the bundle on `IsExistOnTrash(TopCard)` — the **top-most card** still in trash — and it is a `CanActivate` check (re-evaluated at ACTIVATION, distinct from `CanTrigger` at queue time). DCGO stacks OnDeletion via `StackSkillInfos` and the **outer `TriggeredSkillProcess` drains it after the causing effect (incl. its return) resolves**.

The engine instead drains the bundle at delete-time:
- `combat.rs::delete_permanents_batch` (the batched flow, rule 25): `enter_deferred_drain` (3735) → `enqueue_batch_on_deletion` (3739) → `trash_batch_survivors` (3751) → `exit_deferred_drain_and_flush` (3763) **flushes OnDeletion now** → `drain_batch_on_any_deletion` (3781) flushes OnAnyDeletion/OnLeaveField.
- This runs inside CFtD's `delete_permanent` step, BEFORE CFtD's later `return_to_hand` step → Eyesmon still in trash → all 6 fire.

A `draining_deferred`-counter wrap around the causing effect **leaks across CFtD's return-selection PARK** (enter in `activate_hand_main`'s frame, body parks/unwinds, matching exit never runs; the resume callback `effect_queue.rs:3425` opens its own scope and can't close the leaked one). So the deferral must come from restructuring the batch, not from wrapping the caller.

## Two-part fix

### Part A — top-most-card-in-trash gate (additive, SAFE — do first)
DCGO `CanActivateOnDeletion`: the OnDeletion handler activates only if `snapshot.top_card` is still in `snapshot.former_controller`'s trash (or the top card is a token → always activate).

- Location: `effect_queue.rs::run_queued_effect_inner`, right after the source-liveness check (`queued_effect_source_is_live`, ~line 2394) and before / alongside the `effect.condition` check (~2420).
- Logic:
  ```rust
  if qe.timing == EffectTiming::OnDeletion {
      if let Some(snap) = qe.trigger_context.as_ref().and_then(|t| t.deleted_object.as_ref()) {
          let top = snap.top_card;
          let owner = snap.former_controller;
          let is_token = /* card_data_for_handle(top).card_kind == Token */;
          let in_trash = self.player(owner).trash.iter().any(|c| c.handle() == top);
          if !in_trash && !is_token { return; } // suppressed (DCGO CanActivateOnDeletion)
      }
  }
  ```
- Snapshot fields: `DeletedObjectSnapshot { top_card, former_controller, .. }` (see `trigger_context.rs`). `enqueue_batch_on_deletion` (`combat.rs:4189`) already threads the carrier snapshot into every OnDeletion entry (own + inherited share the SAME carrier snapshot → all gated on the top-most card). Verify the snapshot's `top_card` is the carrier's top card (Eyesmon), not a source.
- **Verification of Part A alone:** full regression must stay GREEN with NO behavior change (at delete-time the top card is always in trash, so the gate is a no-op for every existing test). Commit Part A on its own.

### Part B — defer the batch's trigger drain to the outer drain loop (RISKY — restructuring)
Make `delete_permanents_batch` ENQUEUE OnDeletion + OnAnyDeletion + OnLeaveField (in that order so drain order is preserved) but NOT synchronously flush them; let the outer `drain_effect_queue` loop process them after the causing effect's body completes.

- Today: `enqueue_batch_on_deletion` (OnDeletion) → flush; then `drain_batch_on_any_deletion` enqueues AND drains OnAnyDeletion/OnLeaveField. Refactor so OnAnyDeletion/OnLeaveField are ENQUEUED (snapshots threaded, same as OnDeletion) WITHOUT a synchronous drain, ordered AFTER the OnDeletion entries.
- Replace the `exit_deferred_drain_and_flush()` at `combat.rs:3763` so it does NOT force a flush while we're inside an outer drain. Two sub-options:
  - **B1 (preferred):** detect "an outer drain loop is running" via `self.effect_drain_depth > 0`. If so, enqueue-only and return (the outer `drain_effect_queue_inner` loop drains after the causing effect's body). If `effect_drain_depth == 0` (top-level deletion with no drain running — e.g. some direct API/test call), call `drain_effect_queue()` after enqueuing so the triggers still fire.
  - B2: a new `exit_deferred_drain_no_flush()` that decrements without flushing, paired with a top-level drain fallback.
- `active_deletion_batch` is cleared earlier now. Handlers read snapshots from the queued entry's `trigger_context` (already threaded for OnDeletion; thread for OnAnyDeletion/OnLeaveField too). Audit any handler/test that reads `active_deletion_batch` mid-drain.
- The drain loop's existing clause-condition filter (`effect_queue.rs:831`, `non_firing_queued_effect_indices_for`) + `run_queued_effect_inner`'s Part-A gate naturally suppress the bundle once the return has run.

## Risks / watch-list
1. **Reordering** OnDeletion / OnAnyDeletion / OnLeaveField relative to other queued triggers and the state-based ≤0-DP rules-check (`rules_check_between_queued_effects`). Combat "when deleted" effects + post-deletion state reads are the highest-risk callers.
2. `effect_drain_depth` detection for "is an outer loop running." Verify combat-damage deletions and rules-check deletions are inside `effect_drain_depth > 0`.
3. Parked OnDeletion handlers (a handler that installs a selection) must still resume — the batch no longer owns the drain, so the resume path must pick them up via the standard queue.
4. `commit_post_replacement_single` (`combat.rs:3795`) has its own deferred-drain block — apply the same treatment.

## Procedure
1. Part A gate → full regression (must be a no-op) → commit.
2. Part B restructure → iterate on Q19 (→0) + Q20 (→8) → full regression. If widespread breakage that can't be contained quickly, REVERT Part B, keep Part A, report.
3. Trackers on success: `qa/archetype-qa/engine-gaps.md` (G-ON-DELETION-RESOLVES-MID-EFFECT → RESOLVED), `qa/qa-reports/judge-quiz.md` ledger row Q19 → PASS + tally (PASS 18→19, BLOCKED-PRIMITIVE 5→4), coverage line.

## Regression gate (run all on Part B)
```
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test judge_quiz
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test cards_behavioral   # 3894 baseline
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test combat            # 213
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --test option_flow       # 93
cargo test --manifest-path code/digimon-engine/Cargo.toml --features dsl-yaml-loader --lib                    # 212
```

## Key files
- `code/digimon-engine/src/combat.rs` — `delete_permanents_batch` (3621), stages (3722-3781), `enqueue_batch_on_deletion` (4189), `drain_batch_on_any_deletion` (4225), `commit_post_replacement_single` (3795).
- `code/digimon-engine/src/effect_queue.rs` — `run_queued_effect_inner` (2356, condition check ~2420), drain loop + clause filter (798/831), `enter/exit_deferred_drain` (707/714).
- `code/digimon-engine/src/deletion_batch.rs` — `DeletionBatch` / `BatchStage`.
- `code/digimon-engine/src/trigger_context.rs` — `DeletedObjectSnapshot` (`top_card`, `former_controller`).
- `code/digimon-engine/tests/judge_quiz/d_activation_site.rs` — Q19 (un-ignore to test) + Q20 (must stay 8).
- DCGO ref: `DCGO/Assets/Scripts/CardEffect/CardEffectCommons/CanUseEffects/OnDeletion.cs:113`.
