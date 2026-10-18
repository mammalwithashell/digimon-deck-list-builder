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
    ///    `level_just_before`, etc.) while the carrier is still on field,
    ///    then (still in this stage, inside a deferred-drain scope) enqueue
    ///    per-survivor OnDeletion triggers carrying the snapshot in the
    ///    trigger context, since their effects are read from the live slot.
    /// 4. **Trash** — linked-card cascade, `delete_permanent`, modifier
    ///    cleanup, and ONE ACE <Overflow> pass for every leaving card, turn
    ///    player first (§4-19-5). After this step the carriers are gone from
    ///    `battle_area` and their top cards are in trash.
    /// 5. **OnAnyDeletion / OnLeaveField** — global broadcast with each
    ///    snapshot, enqueued (not drained).
    /// 6. **One flush** — the OnDeletion and observer entries drain as ONE
    ///    simultaneous trigger pool (§16-13-2 / §15-4-3-2). Handlers that
    ///    park selections (printed `<Save>`) unwind through
    ///    `pending_selection`; the post-selection drain resumes the rest.
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
    pub(crate) fn with_deletion_receipt<T>(
        &mut self,
        receipt: Option<u64>,
        body: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let previous = self.current_deletion_receipt;
        self.current_deletion_receipt = receipt;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(self)));
        self.current_deletion_receipt = previous;
        match result {
            Ok(value) => value,
            Err(panic) => std::panic::resume_unwind(panic),
        }
    }

    pub fn delete_permanents_batch(
        &mut self,
        handles: Vec<PermanentHandle>,
        cause: crate::replacement::ReplacementCause,
    ) -> crate::deletion_batch::DeletionBatchOutcome {
        self.delete_permanents_batch_with_receipt(handles, cause, None)
    }

    pub(crate) fn delete_permanents_batch_with_receipt(
        &mut self,
        handles: Vec<PermanentHandle>,
        cause: crate::replacement::ReplacementCause,
        receipt: Option<u64>,
    ) -> crate::deletion_batch::DeletionBatchOutcome {
        self.with_deletion_receipt(receipt, |game| {
            game.delete_permanents_batch_scoped(handles, cause)
        })
    }

    fn delete_permanents_batch_scoped(
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

        if crate::replacement::start_deletion_replacement_batch(self) { return; }
        // The impending deletion offers leave and delete immediate effects
        // in one owned ordering window. Other movement destinations still use
        // their existing independent leave event.
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

        self.complete_deletion_batch_after_replacements();
    }

    pub(crate) fn complete_deletion_batch_after_replacements(&mut self) {
        use crate::deletion_batch::BatchStage;
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
        // modifier cleanup; then one <Overflow> pass for the whole batch
        // (§4-19-5). Highest-index-first within each player so removals
        // don't shift later handles.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into trash stage");
            batch.stage = BatchStage::Trashing;
        }
        self.trash_batch_survivors();

        // Stage 4c: OnAnyDeletion / OnLeaveField observers of the same
        // deletion, enqueued post-trash (the observers are the permanents
        // still on the field when the deletion happened) into the SAME
        // deferred scope as the stage-4a `[On Deletion]` entries.
        self.enqueue_batch_on_any_deletion();

        // Stage 4d: a battle's own deletion also triggers the effects of the
        // battle result -- "wins a battle" / "deletes in battle" observers and
        // <Piercing> (`EffectTiming::BattleResult`). §14-2-4 / §15-4-3-2 /
        // §16-7-4: they trigger together with the loser's [On Deletion], so
        // they are enqueued into the same deferred scope and pend in the one
        // pool below, the turn player's first (§15-4-3-5-1; official Q&A
        // Q3931). Only the battle's last deletion sub-batch does this (a
        // member parked on a replacement leaves the rest pending), so every
        // combatant's deletion is known. DCGO: the battle stacks its
        // `OnEndBattle` effects onto the `OnDestroyedAnyone` list `Destroy()`
        // stacked, then one `AutoProcessCheck` resolves both.
        // G-ENGINE-BATTLE-WIN-OBSERVERS-SEQUENCED-AFTER-ON-DELETION.
        if self.active_deletion_batch.as_ref().is_some_and(|b| {
            b.cause == crate::replacement::ReplacementCause::Battle
        }) && self.pending_deletion_batch_rest.is_empty()
        {
            self.enqueue_pending_battle_result_triggers();
        }

        // Stage 5: one flush for the whole deletion. §16-13-2 / §15-4-3-2:
        // every effect this deletion triggered -- `[On Deletion]` (incl.
        // <Retaliation>) and the "when a Digimon is deleted" observers --
        // triggers simultaneously and pends in ONE pool: the drainer stamps
        // them as one trigger batch, the turn player orders theirs first and
        // a player with 2+ gets a `TriggerOrder` pick (§15-4-3-5). DCGO: one
        // `OnDestroyedAnyone` stack (`CardController.cs`). Handlers run
        // post-trash and read pre-removal state via the snapshot threaded
        // into each entry's trigger context (rule 25).
        // G-ENGINE-ON-DELETION-DRAINED-BEFORE-DELETION-OBSERVERS.
        {
            let batch = self
                .active_deletion_batch
                .as_mut()
                .expect("active batch must persist into the deletion-trigger stage");
            batch.stage = BatchStage::TriggerFlush;
        }
        self.exit_deferred_drain_and_flush();
        // A park unwinds with every remaining entry (either kind) still
        // queued; the post-selection drain resumes them.
        self.reevaluate_until_condition_modifiers_if_dirty();
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
            let cause = crate::replacement::deletion_cause_for_controller(cause, self.effect_source_player, handle.player);
            let outcome = self.try_replace(timing, subject, cause, Some(destination));
            if self.pending_selection.is_some() {
                // Parked — caller unwinds and resumes via parked_replacement.
                // The replacement callback commits only the parked member;
                // stash the batch's OTHER members so they are still deleted
                // once the selection settles
                // (G-ENGINE-BATCH-DELETION-PARK-DROPS-REST).
                let rest: Vec<PermanentHandle> = self
                    .active_deletion_batch
                    .as_ref()
                    .map(|b| {
                        b.kill_list
                            .iter()
                            .copied()
                            .filter(|&h| h != handle)
                            .collect()
                    })
                    .unwrap_or_default();
                if rest.is_empty() {
                    return true;
                }
                // Which optional replacement parked? (The installer leaves an
                // `OptionalReplacement` resume frame.)
                let parked_id = self.pending_selection_resume.as_ref().and_then(|stack| {
                    stack.frames.iter().find_map(|f| match f {
                        crate::resume::ResumeFrame::OptionalReplacement(st)
                            if st.order.is_none() =>
                        {
                            Some(crate::replacement::candidate_identity(
                                st.source_card,
                                &st.candidate_kind,
                            ))
                        }
                        _ => None,
                    })
                });
                self.pending_deletion_batch_parked = self
                    .player(handle.player)
                    .battle_area
                    .get(handle.index as usize)
                    .filter(|p| !p.card_sources.is_empty())
                    .map(|p| (handle.player, p.top_card().handle()));
                for h in rest {
                    let Some(card) = self
                        .player(h.player)
                        .battle_area
                        .get(h.index as usize)
                        .filter(|p| !p.card_sources.is_empty())
                        .map(|p| p.top_card().handle())
                    else {
                        continue;
                    };
                    let covered_by_parked = parked_id.is_some_and(|id| {
                        crate::replacement::active_candidate_identities(
                            self,
                            timing,
                            ReplacementSubject::Permanent(h),
                            cause,
                        )
                        .contains(&id)
                    });
                    self.pending_deletion_batch_rest
                        .push(crate::game::PendingDeletionBatchRest {
                            receipt: self.current_deletion_receipt,
                            parked_member: self.pending_deletion_batch_parked,
                            player: h.player,
                            card,
                            cause,
                            covered_by_parked,
                        });
                }
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
                    retained_kind: None,
                    traits: Vec::new(),
                    level: None,
                    dp: None,
                    cause: self
                        .observed_deletion_event_cause_for_controller(handle.player)
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
    /// Snapshots retain controller-relative effect causes across a deferred
    /// trigger drain. A mixed batch cannot copy the first victim's relation.
    fn observed_deletion_event_cause_for_controller(&self, controller: PlayerId) -> Option<crate::trigger_context::EventCause> {
        use crate::trigger_context::EventCause;
        self.observed_deletion_event_cause().map(|cause| match (cause, self.effect_source_player) {
            (EventCause::OwnEffect | EventCause::OpponentEffect, Some(actor)) =>
                if actor == controller { EventCause::OwnEffect } else { EventCause::OpponentEffect },
            _ => cause,
        })
    }

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
        // Last-known names are the RULES names (synth identity): a Digimon whose
        // base name was changed (`ChangeBaseCardName`, e.g. EX13-031 KingSukamon
        // turning it into [Sukamon] until their turn ends) is deleted AS that
        // name, so "when a Digimon with [Sukamon] in its name is deleted"
        // observers see it. Without modifiers this is the top card's own names.
        let identity = perm.synth_identity(&self.card_data, &self.modifiers, handle);
        Some(crate::trigger_context::DeletedObjectSnapshot {
            former_controller: handle.player,
            top_card: top_handle,
            card_kind: identity.kind,
            retained_kind: identity.retained_kind,
            traits: identity.traits.clone(),
            level: identity.level,
            dp: dp_now,
            cause: self
                .observed_deletion_event_cause_for_controller(handle.player)
                .unwrap_or(crate::trigger_context::EventCause::Rule),
            dp_just_before: dp_now,
            level_just_before: identity.level,
            cost_just_before: Some(data.play_cost),
            names_just_before: identity.card_names.clone(),
            traits_just_before: identity.traits.clone(),
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
        // Every card leaving with the batch, for ONE <Overflow> pass below.
        let mut overflow_sources: Vec<crate::card_source::CardSource> = Vec::new();
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
            overflow_sources.extend(self.trash_single_for_batch(handle));
        }
        // §4-19-5: the batch's <Overflow> instances are processed
        // simultaneously -- every instance on the turn player's cards, then
        // the non-turn player's (`apply_ace_overflow_for_sources` orders
        // them) -- not per permanent in removal order (which is player
        // DESCENDING for index-safe removal). §4-19-2: immediately, i.e.
        // after the removals and before the batch's trigger flush.
        // G-ENGINE-OVERFLOW-SIMULTANEOUS-ORDER-PER-PERMANENT.
        if !overflow_sources.is_empty() {
            self.apply_ace_overflow_for_sources(&overflow_sources);
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
        self.capture_pending_battle_survival();
    }

    /// Trash one permanent: linked-card cascade, delete, modifier cleanup.
    /// No OnDeletion enqueue here — that's stage 4a. Returns the stack's
    /// cards for the batch's single <Overflow> pass (empty for a token
    /// stack or a vanished slot); the caller applies it once for the whole
    /// batch (§4-19-5).
    pub(crate) fn trash_single_for_batch(
        &mut self,
        handle: PermanentHandle,
    ) -> Vec<crate::card_source::CardSource> {
        let mut overflow_sources = Vec::new();
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
            // Allocate provenance only for an actual payment receipt. Ordinary
            // deletion must retain its existing identity allocation history.
            let deleted_identity = self.current_deletion_receipt.map(|_| (
                self.player(handle.player).battle_area[handle.index as usize].top_card().handle().0,
                self.capture_digivolve_host_identity(handle),
            ));
            let deleted_dp = self.effective_dp(handle);
            let deleted_digimon = self.permanent_is_digimon_for_rules(handle);
            let sources = self.player(handle.player).battle_area[handle.index as usize]
                .card_sources
                .clone();
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
            // Route stack-to-trash through emission helper so every
            // card surfaces a `GameEvent::Trash` (capability
            // `engine-event-emission`). Token-skip + empty-stack
            // semantics match `Player::delete_permanent`.
            self.trash_permanent_stack(handle.player, handle.index as usize);
            if let Some(receipt) = self.current_deletion_receipt {
                if let Some((_, completed)) = self
                    .deletion_receipts
                    .iter_mut()
                    .find(|(id, _)| *id == receipt)
                {
                    completed.push((handle, deleted_dp, deleted_digimon, deleted_identity.expect("receipt identity captured before removal")));
                }
            }
            if self
                .active_deletion_batch
                .as_ref()
                .is_some_and(|batch| batch.cause == crate::replacement::ReplacementCause::Battle)
            {
                if let Some(top) = sources.last() {
                    self.record_battle_deletion(handle.player, top.handle());
                }
            }
            self.modifiers
                .shift_after_battle_area_remove(handle.player, handle.index);
            self.shift_pending_attack_after_battle_area_remove(handle.player, handle.index);
            self.shift_effect_queue_after_battle_area_remove(handle.player, handle.index);
            // The leaving stack, for the batch's single <Overflow> pass
            // (a token stack processes none).
            if !sources.first().is_some_and(|source| source.is_token) {
                overflow_sources = sources;
            }
        } else {
            self.clear_permanent_full(handle);
            self.modifiers.expire_player_on_permanent_leave(handle);
        }
        self.mark_until_condition_dirty();
        overflow_sources
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
        self.observe_deletion_dispatch_batch(&kill_list);
        for (handle, snapshot) in kill_list.iter().zip(snapshots.iter()) {
            // 16-13: capture WHO this carrier battled now, while the attack is
            // live and the carrier is still in its slot -- see
            // `TriggerContext::battle_opponent_card`.
            let battle_opponent_card = (snapshot.cause
                == crate::trigger_context::EventCause::BattleDeletion)
                .then(|| self.live_battle_opponent_of(*handle))
                .flatten()
                .and_then(|opp| {
                    self.player(opp.player)
                        .battle_area
                        .get(opp.index as usize)
                        .and_then(|perm| perm.card_sources.last())
                        .map(|c| c.handle())
                });
            // Thread the snapshot into the just-enqueued entries so OnDeletion
            // handlers can read `ctx.deleted_self_*()` accessors (and a
            // trigger qualifier is decided on it).
            self.enqueue_triggered_with_context(
                crate::enums::EffectTiming::OnDeletion,
                crate::selection::TriggerSource::Permanent(*handle),
                |trigger| {
                    trigger.deleted_object = Some(snapshot.clone());
                    trigger.cause = Some(snapshot.cause);
                    trigger.affected_player = Some(snapshot.former_controller);
                    trigger.subject =
                        Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    trigger.battle_opponent_card = battle_opponent_card;
                },
            );
        }
    }

    /// The other combatant of the LIVE `pending_attack`, if `handle` is one of
    /// its two sides. `None` with no attack in flight, on a direct player
    /// attack, or for a non-combatant.
    pub(crate) fn live_battle_opponent_of(
        &self,
        handle: PermanentHandle,
    ) -> Option<PermanentHandle> {
        let pa = self.pending_attack.as_ref()?;
        if pa.effective_target_removed_by_dna { return None; }
        let defender = match pa.effective_target {
            crate::AttackTarget::Digimon(h) => h,
            crate::AttackTarget::Player(_) => return None,
        };
        if handle == pa.attacker {
            Some(defender)
        } else if handle == defender {
            Some(pa.attacker)
        } else {
            None
        }
    }

    /// Resolve the battle opponent for the effect currently resolving.
    ///
    /// Prefers the identity captured at TRIGGER time
    /// (`TriggerContext::battle_opponent_card`) and finds where that Digimon is
    /// NOW; if it was captured but is no longer in a battle area (mutual
    /// destruction, or it left play while a prompt was parked) the answer is
    /// `None` -- never a fall-through to whatever attack is live at resolution.
    /// Only with no captured identity does it read the live `pending_attack`.
    pub(crate) fn resolving_battle_opponent_of(
        &self,
        handle: PermanentHandle,
    ) -> Option<PermanentHandle> {
        if let Some(card) = self
            .current_trigger_context
            .as_ref()
            .and_then(|t| t.battle_opponent_card)
        {
            for (pid, player) in self.players.iter().enumerate() {
                for (index, perm) in player.battle_area.iter().enumerate() {
                    if perm.card_sources.iter().any(|c| c.handle() == card) {
                        return Some(PermanentHandle {
                            player: pid as crate::enums::PlayerId,
                            index: index as u8,
                        });
                    }
                }
            }
            return None;
        }
        self.live_battle_opponent_of(handle)
    }

    /// Enqueue global OnAnyDeletion and OnLeaveField per survivor with
    /// snapshots. No drain: the entries pool with the batch's `[On
    /// Deletion]` entries and are flushed together by the caller's single
    /// `exit_deferred_drain_and_flush` (§16-13-2 / §15-4-3-2). Phase 5
    /// (2026-05-23) retired the legacy `pending_post_deletion_replays` slot
    /// — Fortitude/Partition now play from trash inline during their
    /// OnDeletion handlers, so no post-finalize drain hook is needed here.
    pub(crate) fn enqueue_batch_on_any_deletion(&mut self) {
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
                self.enqueue_triggered_with_context(
                    crate::enums::EffectTiming::OnAnyDeletion,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                        // Deletion observers read the cause from the
                        // snapshot (`trigger.cause`), not this bit.
                        effect_initiated: false,
                    },
                    |trigger| {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    },
                );

                self.enqueue_triggered_with_context(
                    crate::enums::EffectTiming::OnLeaveField,
                    crate::selection::TriggerSource::EventObserved {
                        player: handle.player,
                        permanent: *handle,
                        card: *card,
                        // Deletion observers read the cause from the
                        // snapshot (`trigger.cause`), not this bit.
                        effect_initiated: false,
                    },
                    |trigger| {
                        trigger.deleted_object = Some(snapshot.clone());
                        trigger.cause = Some(snapshot.cause);
                        trigger.affected_player = Some(snapshot.former_controller);
                        trigger.subject =
                            Some(crate::trigger_context::EventSubject::Permanent(*handle));
                    },
                );
            }
        }
        // The post-deletion trigger drain is the caller's scope exit
        // (`exit_deferred_drain_and_flush`), which drains only when the
        // deferred counter reaches 0. Inside an effect's resolution window
        // (e.g. an option like Calling From the Darkness that deletes a
        // Digimon and THEN returns cards from trash) the OnDeletion /
        // OnAnyDeletion / OnLeaveField entries stay queued so they resolve
        // only AFTER the causing effect's later steps complete. Combined with
        // the top-card-in-trash gate in `run_queued_effect_inner` (Q19 Part
        // A), an [On Deletion] bundle whose top card was returned to hand by
        // the same effect is then suppressed — DCGO `TriggeredSkillProcess`
        // drains the [On Deletion] stack after the deleting effect resolves,
        // and `CanActivateOnDeletion` re-checks `IsExistOnTrash(TopCard)` at
        // that point. At a top-level deletion (e.g. combat) the flush drains
        // immediately.
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
    /// the batch is mid-stage. Resume by continuing the batch's trigger
    /// drain until either the queue is empty or another handler parks. The
    /// OnAnyDeletion / OnLeaveField observers were enqueued with the
    /// `[On Deletion]` entries before the flush (one pool, §16-13-2), so
    /// they are already in the queue: there is no later stage to advance to.
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
            .is_some_and(|b| matches!(b.stage, BatchStage::TriggerFlush));
        if in_batch {
            self.drain_effect_queue();
            if self.pending_selection.is_some() {
                // Another handler parked; the next `resume_pending_deletion`
                // call (after this selection resolves) continues the drain.
                return;
            }
            // The batch's trigger pool settled: close the batch.
            self.reevaluate_until_condition_modifiers_if_dirty();
            self.active_deletion_batch = None;
        }

        // G-ENGINE-BATCH-DELETION-PARK-DROPS-REST: a multi-member batch that
        // parked on one member's optional replacement now deletes its other
        // members (re-found by top card; same cause).
        if self.pending_selection.is_none() {
            self.resume_deletion_batch_rest_for_receipt(None);
        }
        if self.pending_selection.is_none() {
            self.resume_pending_battle_resolutions();
        }
    }
    /// Resume only the deletion operation whose DSL result is about to be read.
    /// `None` is the ordinary global post-selection drain. A receipt-specific
    /// call leaves other operations queued and never advances battle resolution.
    pub(crate) fn resume_deletion_batch_rest_for_receipt(&mut self, only_receipt: Option<u64>) {
        if self.pending_selection.is_some() {
            return;
        }
        let pending = std::mem::take(&mut self.pending_deletion_batch_rest);
        let (mut rest, unrelated): (Vec<_>, Vec<_>) = pending
            .into_iter()
            .partition(|r| only_receipt.is_none() || r.receipt == only_receipt);
        self.pending_deletion_batch_rest = unrelated;
        if only_receipt.is_none() {
            self.pending_deletion_batch_parked.take();
        }
        while !rest.is_empty() {
            let receipt = rest[0].receipt;
            let cause = rest[0].cause;
            let parked_member = rest[0].parked_member;
            let (group, remaining): (Vec<_>, Vec<_>) = rest.into_iter().partition(|r| {
                r.receipt == receipt && r.cause == cause && r.parked_member == parked_member
            });
            rest = remaining;
            let parked_survived = parked_member.is_some_and(|(player, card)| {
                self.player(player)
                    .battle_area
                    .iter()
                    .any(|p| !p.card_sources.is_empty() && p.top_card().handle() == card)
            });
            let handles: Vec<_> = group
                .iter()
                .filter(|r| !(parked_survived && r.covered_by_parked))
                .filter_map(|r| {
                    self.player(r.player)
                        .battle_area
                        .iter()
                        .position(|p| !p.card_sources.is_empty() && p.top_card().handle() == r.card)
                        .map(|i| PermanentHandle {
                            player: r.player,
                            index: i as u8,
                        })
                })
                .collect();
            if !handles.is_empty() {
                self.delete_permanents_batch_with_receipt(handles, cause, receipt);
            }
            if self.pending_selection.is_some() {
                self.pending_deletion_batch_rest.extend(rest);
                break;
            }
        }
    }
}

#[cfg(test)]
mod deletion_receipt_capture_scope_tests {
    use crate::{debug_runner::{DebugRunner, make_test_card}, replacement::ReplacementCause};
    fn fixture() -> DebugRunner {
        let mut r = DebugRunner::builder().add_card(make_test_card("RECEIPT-BODY", "Body"))
            .deck(0, &["RECEIPT-BODY"; 20]).deck(1, &["RECEIPT-BODY"; 20]).memory(3).start();
        r.skip_mulligan(); r.game.enter_main_phase(); r
    }
    #[test]
    fn ordinary_deletion_does_not_allocate_unused_receipt_identity() {
        let mut r = fixture(); let h = r.place_on_field(0, "RECEIPT-BODY", None);
        assert!(r.game.players[0].battle_area[0].battle_identity.is_none());
        let next = r.game.next_battle_identity;
        let mut cloned = r.game.clone();
        r.game.delete_permanents_batch(vec![h], ReplacementCause::OwnEffect);
        cloned.delete_permanents_batch(vec![h], ReplacementCause::OwnEffect);
        assert_eq!(r.game.next_battle_identity, next, "ordinary deletion must not allocate unused payment provenance");
        assert!(r.game.deletion_receipts.is_empty());
        assert_eq!(r.game.verification_digest(), cloned.verification_digest());
        let survivor = r.place_on_field(0, "RECEIPT-BODY", None);
        assert_eq!(r.game.capture_digivolve_host_identity(survivor), next);
    }
    #[test]
    fn receipt_deletion_keeps_exact_identity_and_digest_discrimination() {
        let mut r = fixture(); let h = r.place_on_field(0, "RECEIPT-BODY", None);
        let card = r.game.players[0].battle_area[0].top_card().handle().0;
        let next = r.game.next_battle_identity;
        r.game.deletion_receipts.push((17, Vec::new()));
        r.game.delete_permanents_batch_with_receipt(vec![h], ReplacementCause::OwnEffect, Some(17));
        assert_eq!(r.game.deletion_receipts[0].1.len(), 1);
        assert_eq!(r.game.deletion_receipts[0].1[0].3, (card, next));
        assert_eq!(r.game.next_battle_identity, next + 1);
        let mut different = r.game.clone();
        assert_eq!(r.game.verification_digest(), different.verification_digest());
        different.deletion_receipts[0].1[0].3.1 += 1;
        assert_ne!(r.game.verification_digest(), different.verification_digest());
    }
}
