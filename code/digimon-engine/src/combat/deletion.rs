//! Permanent deletion lifecycle (Tier 1, rule 25).

#![allow(unused_imports)]
use super::*;
use crate::aura::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::effect::*;
use crate::enums::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::selection::*;
use crate::trigger_context::*;

impl Game {
    /// Delete a permanent, firing its OnDeletion effects first.
    /// Also clears any modifiers attached to the handle.
    ///
    /// Phase 7: this entry point infers the `ReplacementCause` from live game
    /// state (security-resolution / pending-attack / effect_source_player) and
    /// delegates to `delete_permanent_with_cause`. Callers that already know
    /// the cause (e.g. `resolve_battle` → `Battle`) should invoke
    /// `delete_permanent_with_cause` directly.
    ///
    /// **Post-batched-refactor (2026-05-23):** both this entrypoint and
    /// `delete_permanent_with_cause` are shims over `delete_permanents_batch`
    /// — the unified deletion API that runs the DCGO-modeled batched flow
    /// (replacement window → snapshot → trash → OnDeletion → OnAnyDeletion).
    pub fn delete_permanent_with_effects(&mut self, handle: PermanentHandle) {
        let cause = self.infer_deletion_cause(handle);
        self.delete_permanent_with_cause(handle, cause);
    }

    /// Batched deletion entrypoint — DCGO `DestroyPermanentsClass.Destroy()`
    /// equivalent. Accepts a list of permanent handles and processes them as
    /// a single batched unit:
    ///
    /// 1. **Filter** — drop handles whose battle-area slot is empty.
    /// 2. **Per-handle replacement window** — fire `WhenWouldLeaveBattleArea`
    ///    then `WhenWouldBeDeleted` per handle. (Phase 3 batches these into
    ///    a single two-stage cut-in across the kill list.) Cancelled,
    ///    redirected, and substituted outcomes mutate the surviving list.
    /// 3. **Snapshot** — capture each surviving permanent's pre-removal
    ///    state (`DeletedObjectSnapshot` with `dp_just_before`,
    ///    `level_just_before`, etc.) while the carrier is still on field.
    /// 4. **Trash** — linked-card cascade, ACE overflow, `delete_permanent`,
    ///    modifier cleanup. After this step the carrier is gone from
    ///    `battle_area` and its top card is in trash.
    /// 5. **OnDeletion drain** — enqueue per-survivor OnDeletion triggers
    ///    carrying the snapshot in the trigger context. Handlers that park
    ///    selections (printed `<Save>`) unwind through `pending_selection`;
    ///    the resume hook (`Game::resume_pending_deletion`) continues the
    ///    drain via the active batch after each selection resolves.
    /// 6. **OnAnyDeletion / OnLeaveField** — global broadcast with each
    ///    snapshot. Drain.
    ///
    /// Callers:
    /// - Single-target callers (`delete_permanent_with_effects`,
    ///   `delete_permanent_with_cause`) pass a one-element kill list.
    /// - Battle resolution passes `[defender]` (winner) or
    ///   `[defender, attacker]` (mutual destruction).
    /// - DSL `DeleteBoundPermanents` passes the resolved binding list.
    ///
    /// Returns a `DeletionBatchOutcome` describing which handles trashed,
    /// were cancelled, or were substituted in. Most callers ignore this.
    ///
    /// See `openspec/changes/align-deletion-with-dcgo-model/design.md` D2
    /// for the rationale; `specs/permanent-deletion-semantics/spec.md` for
    /// the requirement contracts this implements.
    pub fn delete_permanents_batch(
        &mut self,
        handles: Vec<PermanentHandle>,
        cause: crate::replacement::ReplacementCause,
    ) -> crate::deletion_batch::DeletionBatchOutcome {
        use crate::deletion_batch::{DeletionBatch, DeletionBatchOutcome};

        // Stage: Filtering — drop handles whose battle_area slot is empty.
        let kill_list: Vec<PermanentHandle> = handles
            .into_iter()
            .filter(|h| {
                self.player(h.player)
                    .battle_area
                    .get(h.index as usize)
                    .is_some()
            })
            .collect();
        if kill_list.is_empty() {
            return DeletionBatchOutcome::default();
        }

        // Track that a batch is in flight. Save+restore the outer batch
        // so nested `delete_permanents_batch` calls inside an OnDeletion
        // handler don't clobber the outer batch's state.
        let prior_batch = self.active_deletion_batch.take();
        self.active_deletion_batch = Some(DeletionBatch::new(kill_list.clone(), cause));

        // Carry the cause across the OnDeletion drain via the existing
        // `current_deletion_cause` slot, matching the pre-batched panic-safe
        // save/restore at the old single-handle entrypoint.
        let prior_cause = self.current_deletion_cause;
        self.current_deletion_cause = Some(cause);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.run_deletion_batch_stages()
        }));

        self.current_deletion_cause = prior_cause;
        let outcome = match self.active_deletion_batch.take() {
            Some(batch) => DeletionBatchOutcome {
                completed: batch.completed,
                cancelled: batch.cancelled,
                substituted_in: batch.substituted_in,
            },
            None => DeletionBatchOutcome::default(),
        };
        self.active_deletion_batch = prior_batch;

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
        outcome
    }

    /// Run the batched deletion stages against `self.active_deletion_batch`.
    /// Called by `delete_permanents_batch` inside its panic-safe scope.
    ///
    /// **Phase 2 implementation note.** The replacement window stages run
    /// per-handle here using the existing `try_replace` machinery. Phase 3
    /// will batch them into a single two-stage cut-in across the kill list.
    /// The snapshot + trash + OnDeletion stages already run as a batched
    /// unit so trash-before-drain semantics hold for single-target callers
    /// (the dominant case) and the per-handle replacement loop preserves
    /// today's substitute/redirect/cancel outcomes.
    pub(crate) fn run_deletion_batch_stages(&mut self) {
        use crate::deletion_batch::BatchStage;
        use crate::enums::{EffectTiming, Zone};

        // Stage 1: WhenWouldLeaveBattleArea (per-handle for Phase 2).
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("run_deletion_batch_stages called without active batch");
            batch.stage = BatchStage::Stage1ReplacementDrain;
        }
        if self.run_replacement_stage(EffectTiming::WhenWouldLeaveBattleArea, Zone::Trash) {
            return;
        }

        // Stage 2: WhenWouldBeDeleted.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist across stages");
            batch.stage = BatchStage::Stage2ReplacementDrain;
        }
        if self.run_replacement_stage(EffectTiming::WhenWouldBeDeleted, Zone::Trash) {
            return;
        }

        // Stage 3: Snapshotting. Capture each survivor's pre-removal state
        // while the carrier is still on field.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into snapshot stage");
            batch.stage = BatchStage::Snapshotting;
        }
        self.capture_batch_snapshots();

        // DCGO-faithful enqueue-before-trash ordering. `enqueue_from_permanent`
        // reads the permanent's effects from the live `battle_area` slot,
        // which means OnDeletion must be enqueued while the carrier is
        // still on field. The drain then runs post-trash so handlers see
        // the carrier in trash. We use the deferred-drain scope to hold
        // the drain across the trash mutation.
        //
        // DCGO equivalent: `DestroyPermanentsClass.Destroy()` step 8 stacks
        // OnDestroyedAnyone via `autoProcessing.StackSkillInfos` BEFORE the
        // trash loop at step 10; the outer `TriggeredSkillProcess` drains
        // after step 10 returns.
        self.enter_deferred_drain();

        // Stage 4a: Enqueue OnDeletion for each survivor with its snapshot,
        // while the carrier is still on field (so its effects can be read).
        self.enqueue_batch_on_deletion();

        // Stage 4b: Trashing. Move carriers to trash; linked-card cascade;
        // modifier cleanup. Highest-index-first within each player so
        // removals don't shift later handles.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into trash stage");
            batch.stage = BatchStage::Trashing;
        }
        self.trash_batch_survivors();

        // Stage 5: OnDeletion drain. Exit the deferred-drain scope to flush
        // the queued OnDeletion handlers. Handlers run post-trash and read
        // pre-removal state via the snapshot.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into OnDeletion stage");
            batch.stage = BatchStage::OnDeletionDrain;
        }
        self.exit_deferred_drain_and_flush();

        // If an OnDeletion handler parked a selection, control unwinds
        // here with `pending_selection.is_some()`. The resume hook
        // (`resume_pending_deletion`) continues into the OnAnyDeletion
        // stage when the selection resolves.
        if self.pending_selection.is_some() {
            return;
        }

        // Stage 6: OnAnyDeletion / OnLeaveField global broadcasts.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into OnAnyDeletion stage");
            batch.stage = BatchStage::OnAnyDeletionDrain;
        }
        self.drain_batch_on_any_deletion();
    }

    /// Run one replacement stage (`WhenWouldLeaveBattleArea` or
    /// `WhenWouldBeDeleted`) over the active batch's kill list. Returns
    /// `true` if a handler parked a selection — caller unwinds.
    ///
    /// Phase 2: per-handle dispatch using the existing `try_replace`
    /// machinery. Outcomes mutate the active batch's kill list / cancelled /
    /// substituted_in vectors.
    pub(crate) fn run_replacement_stage(
        &mut self,
        timing: crate::enums::EffectTiming,
        destination: crate::enums::Zone,
    ) -> bool {
        use crate::replacement::{ReplacementOutcome, ReplacementSubject};

        let cause = self
            .active_deletion_batch
            .as_ref()
            .expect("active batch in replacement stage")
            .cause;

        // Process kill_list in a copy so we can mutate the batch's list as
        // we go (substitutes append, cancels mark).
        let mut i = 0;
        loop {
            let handle = {
                let batch = self
                    .active_deletion_batch
                    .as_ref()
                    .expect("active batch persists through replacement loop");
                if i >= batch.kill_list.len() {
                    return false;
                }
                batch.kill_list[i]
            };

            // Skip handles already trashed via OnDeletion side-effects
            // (defensive — shouldn't happen this early but guarded).
            if self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_none()
            {
                if let Some(batch) = self.active_deletion_batch.as_mut() {
                    batch.cancelled.push(handle);
                    batch.kill_list.remove(i);
                }
                continue;
            }

            let subject = ReplacementSubject::Permanent(handle);
            let outcome = self.try_replace(timing, subject, cause, Some(destination));
            if self.pending_selection.is_some() {
                // Parked — caller unwinds and resumes via parked_replacement.
                return true;
            }

            match outcome {
                ReplacementOutcome::None => {
                    i += 1;
                }
                ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => {
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(crate::enums::Zone::Deck) => {
                    self.return_to_deck(handle, crate::enums::StackPosition::Bottom);
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(crate::enums::Zone::Hand) => {
                    self.return_to_hand(handle);
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Redirected(other) => {
                    debug_assert!(
                        false,
                        "unexpected redirect destination for {:?}: {:?}",
                        timing, other
                    );
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                    }
                }
                ReplacementOutcome::Substituted(ReplacementSubject::Permanent(source_h)) => {
                    // Substitute: drop original from kill list, append
                    // substitute. Bound recursion via batch.depth.
                    if let Some(batch) = self.active_deletion_batch.as_mut() {
                        batch.cancelled.push(handle);
                        batch.kill_list.remove(i);
                        batch.depth = batch.depth.saturating_add(1);
                        if batch.depth >= 16 {
                            debug_assert!(
                                false,
                                "deletion batch substitute depth exceeded — pathological loop"
                            );
                            return false;
                        }
                        // Only add if substitute is on field and not already
                        // in the kill list.
                        let already_present = batch.kill_list.contains(&source_h)
                            || batch.substituted_in.contains(&source_h);
                        if !already_present {
                            batch.kill_list.push(source_h);
                            batch.substituted_in.push(source_h);
                        }
                    }
                    // Don't increment i — the slot we removed shifts later
                    // entries down by one. Next iter checks the new entry
                    // at index i.
                }
                ReplacementOutcome::Substituted(_) => {
                    debug_assert!(false, "non-Permanent substitute subject for {:?}", timing);
                    i += 1;
                }
            }
        }
    }

    /// Capture `DeletedObjectSnapshot` for each survivor in the active
    /// batch. Populates `batch.snapshots` and `batch.top_cards`.
    pub(crate) fn capture_batch_snapshots(&mut self) {
        let kill_list = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in snapshot stage");
            batch.kill_list.clone()
        };
        let mut snapshots: Vec<crate::trigger_context::DeletedObjectSnapshot> =
            Vec::with_capacity(kill_list.len());
        let mut top_cards: Vec<Option<crate::card_source::CardHandle>> =
            Vec::with_capacity(kill_list.len());
        for handle in &kill_list {
            let snapshot_opt = self.build_snapshot_for_handle(*handle);
            let top = snapshot_opt.as_ref().map(|s| s.top_card);
            top_cards.push(top);
            if let Some(snap) = snapshot_opt {
                snapshots.push(snap);
            } else {
                // Carrier vanished between filter and snapshot — defensive.
                // Build a placeholder snapshot using just the cause so the
                // batch arrays stay aligned with kill_list indices.
                snapshots.push(crate::trigger_context::DeletedObjectSnapshot {
                    former_controller: handle.player,
                    top_card: crate::card_source::CardHandle(0),
                    card_kind: crate::enums::CardKind::Digimon,
                    traits: Vec::new(),
                    level: None,
                    dp: None,
                    cause: self
                        .observed_deletion_event_cause()
                        .unwrap_or(crate::trigger_context::EventCause::Rule),
                    dp_just_before: None,
                    level_just_before: None,
                    cost_just_before: None,
                    names_just_before: Vec::new(),
                    traits_just_before: Vec::new(),
                    source_count_just_before: 0,
                    digisources_just_before: Vec::new(),
                    is_token: false,
                });
            }
        }
        let batch = self
            .active_deletion_batch
            .as_mut()
            .expect("active batch persists through snapshot capture");
        batch.snapshots = snapshots;
        batch.top_cards = top_cards;
    }

    /// Build a `DeletedObjectSnapshot` for a live battle-area handle.
    /// Returns `None` if the slot is empty.
    pub(crate) fn build_snapshot_for_handle(
        &self,
        handle: PermanentHandle,
    ) -> Option<crate::trigger_context::DeletedObjectSnapshot> {
        let perm = self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)?;
        let top_handle = perm.top_card().handle();
        let data = self.card_data_for_handle(top_handle)?;
        let mut digisources: Vec<crate::card_source::CardHandle> = Vec::new();
        for src in perm.card_sources.iter() {
            let h = src.handle();
            if h != top_handle {
                digisources.push(h);
            }
        }
        let source_count = digisources.len();
        let dp_now = self.effective_dp(handle);
        let is_token = perm.top_card().is_token;
        Some(crate::trigger_context::DeletedObjectSnapshot {
            former_controller: handle.player,
            top_card: top_handle,
            card_kind: data.card_kind,
            traits: data.traits.clone(),
            level: data.level,
            dp: dp_now,
            cause: self
                .observed_deletion_event_cause()
                .unwrap_or(crate::trigger_context::EventCause::Rule),
            dp_just_before: dp_now,
            level_just_before: data.level,
            cost_just_before: Some(data.play_cost),
            names_just_before: vec![data.card_name.clone()],
            traits_just_before: data.traits.clone(),
            source_count_just_before: source_count,
            digisources_just_before: digisources,
            is_token,
        })
    }

    /// Trash every survivor in the active batch. Processes within each
    /// player's `battle_area` in highest-index-first order so removals
    /// don't shift later handles' indices.
    pub(crate) fn trash_batch_survivors(&mut self) {
        // Group kill_list by player and sort high-to-low.
        let kill_list = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in trash stage");
            batch.kill_list.clone()
        };
        // Sort: descending player, descending index. Stable iter so the
        // batch's snapshot/top_cards arrays don't have to be reordered —
        // we look up by handle, not by position.
        let mut sorted = kill_list.clone();
        sorted.sort_by(|a, b| b.player.cmp(&a.player).then(b.index.cmp(&a.index)));
        for handle in sorted {
            // Skip if already gone (defensive — substitute targets that
            // were already cancelled, etc.).
            if self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .is_none()
            {
                continue;
            }
            self.trash_single_for_batch(handle);
        }
        // Record what completed (everything that's now gone from battle_area
        // among the kill_list).
        let batch = self
            .active_deletion_batch
            .as_mut()
            .expect("active batch persists through trash stage");
        let mut completed = Vec::new();
        for h in &batch.kill_list {
            // Use a heuristic: if the handle's slot is now empty, it
            // trashed. Index-shift across permanents in the same player
            // makes this approximate; the snapshot is the authoritative
            // record of what died.
            completed.push(*h);
        }
        batch.completed = completed;
    }

    /// Trash one permanent: linked-card cascade, ACE overflow, delete,
    /// modifier cleanup. No OnDeletion enqueue here — that's stage 5.
    pub(crate) fn trash_single_for_batch(&mut self, handle: PermanentHandle) {
        // Linked-card cascade — drain BEFORE removing the permanent so
        // OnLinkedCardTrashed observers see the host still in place.
        let had_linked = {
            let linked = self
                .player(handle.player)
                .battle_area
                .get(handle.index as usize)
                .map(|p| !p.linked_cards.is_empty())
                .unwrap_or(false);
            if linked {
                let taken = std::mem::take(
                    &mut self.player_mut(handle.player).battle_area[handle.index as usize]
                        .linked_cards,
                );
                // Route through emission helper so each linked card surfaces
                // a `GameEvent::Trash` (capability `engine-event-emission`).
                for card in taken {
                    self.trash_card(handle.player, card);
                }
                true
            } else {
                false
            }
        };
        if had_linked {
            // Keep the already-stacked OnDeletion drain separate from this
            // immediate linked-card trash event; they are different trigger
            // windows and should not collapse into one TriggerOrder prompt.
            let mut deferred_queue = std::mem::take(&mut self.effect_queue);
            for pid in 0..self.players.len() {
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLinkedCardTrashed,
                    crate::selection::TriggerSource::PlayerBattleArea(pid as PlayerId),
                );
            }
            self.drain_effect_queue();
            let mut immediate_queue = std::mem::take(&mut self.effect_queue);
            immediate_queue.append(&mut deferred_queue);
            self.effect_queue = immediate_queue;
        }

        // Now actually trash.
        if self
            .player(handle.player)
            .battle_area
            .get(handle.index as usize)
            .is_some()
        {
            let sources = self.player(handle.player).battle_area[handle.index as usize]
                .card_sources
                .clone();
            if !sources.first().is_some_and(|source| source.is_token) {
                self.apply_ace_overflow_for_sources(&sources);
            }
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
            // Route stack-to-trash through emission helper so every
            // card surfaces a `GameEvent::Trash` (capability
            // `engine-event-emission`). Token-skip + empty-stack
            // semantics match `Player::delete_permanent`.
            self.trash_permanent_stack(handle.player, handle.index as usize);
            self.modifiers
                .shift_after_battle_area_remove(handle.player, handle.index);
        } else {
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
        }
        self.mark_until_condition_dirty();
    }

    /// Enqueue OnDeletion for each survivor in the active batch with its
    /// snapshot threaded into each entry's trigger context.
    ///
    /// Called BEFORE the trash stage so `enqueue_from_permanent` can read
    /// the carriers' effects from their live `battle_area` slots. The
    /// surrounding `enter_deferred_drain` scope holds the actual drain
    /// until `exit_deferred_drain_and_flush` runs after trash — matching
    /// DCGO `DestroyPermanentsClass.Destroy()`'s step-8 stack-before-trash
    /// ordering.
    pub(crate) fn enqueue_batch_on_deletion(&mut self) {
        let (kill_list, snapshots) = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in OnDeletion-enqueue stage");
            (batch.kill_list.clone(), batch.snapshots.clone())
        };
        for (handle, snapshot) in kill_list.iter().zip(snapshots.iter()) {
            let queue_start = self.effect_queue.len();
            self.enqueue_triggered(
                crate::enums::EffectTiming::OnDeletion,
                crate::selection::TriggerSource::Permanent(*handle),
            );
            // Thread the snapshot into the just-enqueued entries so OnDeletion
            // handlers can read `ctx.deleted_self_*()` accessors.
            for queued in self.effect_queue.iter_mut().skip(queue_start) {
                if queued.timing != crate::enums::EffectTiming::OnDeletion {
                    continue;
                }
                if let Some(trigger) = queued.trigger_context.as_mut() {
                    trigger.deleted_object = Some(snapshot.clone());
                    trigger.cause = Some(snapshot.cause);
                    trigger.affected_player = Some(snapshot.former_controller);
                    trigger.subject =
                        Some(crate::trigger_context::EventSubject::Permanent(*handle));
                }
            }
        }
    }

    /// Enqueue global OnAnyDeletion and OnLeaveField per survivor with
    /// snapshots, drain. Phase 5 (2026-05-23) retired the legacy
    /// `pending_post_deletion_replays` slot — Fortitude/Partition now play
    /// from trash inline during their OnDeletion handlers, so no
    /// post-finalize drain hook is needed here.
    pub(crate) fn drain_batch_on_any_deletion(&mut self) {
        let (kill_list, snapshots, top_cards) = {
            let batch = self
                .active_deletion_batch
                .as_ref()
                .expect("active batch in OnAnyDeletion stage");
            (
                batch.kill_list.clone(),
                batch.snapshots.clone(),
                batch.top_cards.clone(),
            )
        };

        // OnAnyDeletion + OnLeaveField per survivor.
        for ((handle, snapshot), top_card_opt) in
            kill_list.iter().zip(snapshots.iter()).zip(top_cards.iter())
        {
            if let Some(card) = top_card_opt {
                let queue_start = self.effect_queue.len();
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnAnyDeletion,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                    },
                );
                for queued in self.effect_queue.iter_mut().skip(queue_start) {
                    if queued.timing != crate::enums::EffectTiming::OnAnyDeletion {
                        continue;
                    }
                    if let Some(trigger) = queued.trigger_context.as_mut() {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    }
                }

                let queue_start_lf = self.effect_queue.len();
                self.enqueue_triggered(
                    crate::enums::EffectTiming::OnLeaveField,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                    },
                );
                for queued in self.effect_queue.iter_mut().skip(queue_start_lf) {
                    if queued.timing != crate::enums::EffectTiming::OnLeaveField {
                        continue;
                    }
                    if let Some(trigger) = queued.trigger_context.as_mut() {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    }
                }
            }
        }
        // Defer the post-deletion trigger drain to the outermost deferred
        // scope. `maybe_drain_effect_queue` drains only when
        // `draining_deferred == 0`; inside an effect's resolution window
        // (e.g. an option like Calling From the Darkness that deletes a
        // Digimon and THEN returns cards from trash) it leaves the
        // OnDeletion / OnAnyDeletion / OnLeaveField entries queued so they
        // resolve only AFTER the causing effect's later steps complete.
        // Combined with the top-card-in-trash gate in `run_queued_effect_inner`
        // (Q19 Part A), an [On Deletion] bundle whose top card was returned to
        // hand by the same effect is then suppressed — DCGO `TriggeredSkillProcess`
        // drains the [On Deletion] stack after the deleting effect resolves,
        // and `CanActivateOnDeletion` re-checks `IsExistOnTrash(TopCard)` at
        // that point. At a top-level deletion (`draining_deferred == 0`, e.g.
        // combat) this drains immediately, matching prior behavior.
        self.maybe_drain_effect_queue();
        self.reevaluate_until_condition_modifiers_if_dirty();
    }

    /// Cause-aware deletion entry point. **Post-batched-refactor (2026-05-23):**
    /// shimmed through `delete_permanents_batch(vec![handle], cause)`. The
    /// batched flow runs replacement window → snapshot → trash → OnDeletion →
    /// OnAnyDeletion as a unit, so a single-target deletion exhibits the
    /// DCGO-modeled trash-before-drain semantics: OnDeletion handlers fire
    /// AFTER the carrier's top card has moved to trash.
    ///
    /// Callers that need to know whether the deletion completed (vs. was
    /// cancelled/redirected/substituted) should call `delete_permanents_batch`
    /// directly and inspect the returned `DeletionBatchOutcome`.
    pub fn delete_permanent_with_cause(
        &mut self,
        handle: PermanentHandle,
        cause: crate::replacement::ReplacementCause,
    ) {
        let _ = self.delete_permanents_batch(vec![handle], cause);
    }

    /// Resume any deferred deletion work after a `pending_selection`
    /// resolves. Called by `effect_queue::resolve_generic_selection` after
    /// the parked selection's callback runs and the post-callback drain
    /// returns without re-parking.
    ///
    /// **Post-batched-refactor (2026-05-23):** when an OnDeletion handler
    /// parked a selection during the OnDeletion drain of
    /// `delete_permanents_batch`, `active_deletion_batch.is_some()` and
    /// the batch is mid-stage. Resume by continuing the OnDeletion drain
    /// until either the queue is empty or another handler parks. If
    /// drained cleanly, advance to the OnAnyDeletion stage.
    pub(crate) fn resume_pending_deletion(&mut self) {
        use crate::deletion_batch::BatchStage;

        // When an OnDeletion handler installs a `pending_selection` during
        // `exit_deferred_drain_and_flush`, the deferred-drain scope is
        // already closed (counter back to 0). `drain_effect_queue` is the
        // right primitive to continue the drain; further parks unwind
        // again until the queue is empty.
        let in_batch = self
            .active_deletion_batch
            .as_ref()
            .is_some_and(|b| matches!(b.stage, BatchStage::OnDeletionDrain));
        if in_batch {
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                // Another handler parked; the next `resume_pending_deletion`
                // call (after this selection resolves) continues the drain.
                return;
            }
            // OnDeletion drain settled — advance to OnAnyDeletion stage.
            if let Some(batch) = self.active_deletion_batch.as_mut() {
                batch.stage = BatchStage::OnAnyDeletionDrain;
            }
            self.drain_batch_on_any_deletion();
            // After OnAnyDeletion stage, clear the batch.
            self.active_deletion_batch = None;
        }
    }
}
