//! Misc Tier-2 operations — `impl Game`.

#![allow(unused_imports)]
use super::*;
use crate::card_data::*;
use crate::card_source::*;
use crate::combat::*;
use crate::digixros::*;
use crate::effect::*;
use crate::effect_context::*;
use crate::enums::*;
use crate::game::*;
use crate::modifiers::*;
use crate::permanent::*;
use crate::player::*;
use crate::replacement::*;
use crate::rules::*;
use crate::scheduled_effects::*;
use crate::selection::*;
use crate::token_registry::*;
use crate::trigger_context::*;
use rand::seq::SliceRandom;

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn finish_play_from_hand_after_reductions(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: crate::card_source::CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> PlayFromHandCostResult {
        if self.install_pending_digixros_material_selection_or_finish(
            player_id,
            hand_index,
            target_card,
            cost_delta,
            source,
            origin,
            suppress_on_play,
            total_reduction,
        ) {
            return PlayFromHandCostResult::Pending;
        }

        self.commit_play_from_hand_after_reductions(
            player_id,
            hand_index,
            target_card,
            cost_delta,
            source,
            origin,
            suppress_on_play,
            total_reduction,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn commit_play_from_hand_after_reductions(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: crate::card_source::CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> PlayFromHandCostResult {
        let field_slots = self.rules.field_slots;
        let printed_cost = {
            let player = self.player(player_id);
            if player.battle_area.len() >= field_slots as usize {
                return PlayFromHandCostResult::Failed;
            }
            let Some(card) = player.hand.get(hand_index) else {
                return PlayFromHandCostResult::Failed;
            };
            if card.handle() != target_card {
                return PlayFromHandCostResult::Failed;
            }
            card.play_cost(&self.card_data)
        };
        let base_cost = cost_delta.resolve(printed_cost) as i32;
        // Observer dispatch (G-BEFORE-PAY-COST-GAIN-MEMORY) — fires AFTER
        // the cost-reduction chain finishes (`total_reduction` is the sum
        // of accepted reducers) but BEFORE the WhenPermanentWouldPlay
        // replacement and final memory deduction.
        let cost_target_ctx = CostTargetContext {
            card: target_card,
            from_hand: true,
            is_digivolve: false,
            target_permanents: [None, None],
        };
        self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
        let transaction_cost = self
            .pending_digixros_transaction
            .as_ref()
            .map(|transaction| transaction.final_cost() as i32)
            .unwrap_or(base_cost);
        let effective_cost = (transaction_cost - total_reduction).max(0) as u16;

        self.pending_would_play_resume = Some(PendingWouldPlayResume {
            player: player_id,
            card: target_card,
            effective_cost,
            origin,
            effect_initiated: source == PlaySource::ByEffect,
            suppress_on_play,
        });
        let cause = match source {
            PlaySource::ByEffect => crate::replacement::ReplacementCause::OwnEffect,
            PlaySource::ByHand | PlaySource::ByDigivolve => {
                crate::replacement::ReplacementCause::OwnEffect
            }
        };
        let outcome = self.try_replace(
            EffectTiming::WhenPermanentWouldPlay,
            crate::replacement::ReplacementSubject::Card(target_card, Zone::Hand),
            cause,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return PlayFromHandCostResult::Pending;
        }
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.pending_would_play_resume = None;
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.pending_would_play_resume = None;
                self.pending_digixros_transaction = None;
                return PlayFromHandCostResult::Failed;
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.pending_would_play_resume = None;
                self.pending_digixros_transaction = None;
                return PlayFromHandCostResult::Failed;
            }
        }

        // DigiXros declare-then-pay: before placing the host and paying memory,
        // fire the `WhenWouldLeaveBattleArea` replacement window for each
        // battle-area material that is about to be consumed (cause = DigiXros,
        // NOT Battle). A replacement (e.g. BT17-095) may redirect/substitute a
        // material away (consume it into a DNA-evo) — in which case the material
        // is no longer available to the DigiXros host, the reduced cost is
        // recomputed, and an unpayable play returns to hand for 0 memory
        // (judge-quiz Q25/Q26/Q27, G-DIGIXROS-DEPARTURE-LEAVE-TRIGGER +
        // G-DIGIXROS-UNPAYABLE-RETURN-TO-HAND).
        if self.pending_digixros_transaction.is_some() {
            return self.run_digixros_leave_windows_then_commit(
                player_id,
                target_card,
                total_reduction,
                source,
                suppress_on_play,
                0,
            );
        }

        let committed = self.commit_play_from_hand_card_no_replace(
            player_id,
            target_card,
            effective_cost,
            source == PlaySource::ByEffect,
            suppress_on_play,
        );
        self.pending_digixros_transaction = None;
        committed
            .map(PlayFromHandCostResult::Played)
            .unwrap_or(PlayFromHandCostResult::Failed)
    }

    /// Fire the `WhenWouldLeaveBattleArea` replacement window over each
    /// battle-area DigiXros material (cause = `DigiXros`) starting at
    /// `start_material`, then either commit the host play or — if a material was
    /// redirected away and the reduced cost became unpayable — return the played
    /// card to hand for 0 memory.
    ///
    /// Re-entrant: if a replacement parks a `pending_selection` (e.g. BT17-095's
    /// optional `<Delay>` accept / DNA-evo target picks), the selection's
    /// callback + on_decline are wrapped to resume this loop at the next
    /// material. The played card stays in `player_id`'s hand throughout — the
    /// host is only placed once every window has resolved.
    pub(crate) fn run_digixros_leave_windows_then_commit(
        &mut self,
        player_id: PlayerId,
        target_card: crate::card_source::CardHandle,
        total_reduction: i32,
        source: PlaySource,
        suppress_on_play: bool,
        start_material: usize,
    ) -> PlayFromHandCostResult {
        use crate::replacement::{ReplacementCause, ReplacementSubject};

        // Snapshot the battle-area material origins (in selection order) so the
        // index `i` is stable across re-entries. We re-resolve the live
        // permanent handle (by card identity) each iteration, because earlier
        // windows may have shifted battle-area indices.
        let battle_materials: Vec<crate::card_source::CardHandle> = self
            .pending_digixros_transaction
            .as_ref()
            .map(|tx| {
                tx.pre_attached_materials
                    .iter()
                    .chain(tx.selected_materials.iter())
                    .filter_map(|m| match m.origin {
                        DigiXrosMaterialOrigin::BattleArea { card, .. } => Some(card),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        let mut i = start_material;
        while i < battle_materials.len() {
            let material_card = battle_materials[i];
            i += 1;

            // Locate the live permanent carrying this material's top card.
            let handle = self
                .player(player_id)
                .battle_area
                .iter()
                .position(|p| p.top_card().handle() == material_card)
                .map(|idx| PermanentHandle {
                    player: player_id,
                    index: idx as u8,
                });
            let Some(handle) = handle else {
                // Material already gone (consumed by an earlier window's
                // redirect) — nothing to observe; the prune below drops it.
                continue;
            };

            let _ = self.try_replace(
                EffectTiming::WhenWouldLeaveBattleArea,
                ReplacementSubject::Permanent(handle),
                ReplacementCause::DigiXros,
                None,
            );

            if self.pending_selection.is_some() {
                // The leave observer (e.g. BT17-095) parked its optional
                // `<Delay>` accept. Rather than commit the host eagerly (which
                // would tuck the leaving material UNDER the host, burying it so
                // the parked reward could no longer extract it), move the leaving
                // material OUT to the leaving/limbo slot: it is no longer any
                // permanent's top card (so "the material has left" holds — Q25),
                // yet it remains resolvable + EXTRACTABLE by the parked reward's
                // DNA-evo (G-DIGIXROS-REDIRECT-EXTRACTION). The host stays in hand
                // until every leave window has resolved.
                //
                // Arm a continuation so that once the parked reward settles
                // (accept-and-extract OR decline), the loop re-enters at the next
                // material and ultimately reaches `finalize_...`, which either
                // commits the host (decline → material consumed under host) or
                // returns it to hand for 0 memory (extract → cost unpayable —
                // judge-quiz Q26/Q27).
                self.move_battle_permanent_to_limbo(player_id, material_card);
                self.arm_digixros_resume_after_parked_leave(DigiXrosResumeContinuation {
                    player: player_id,
                    target_card,
                    total_reduction,
                    source,
                    suppress_on_play,
                    next_material: i,
                });
                return PlayFromHandCostResult::Pending;
            }
            // Whatever the outcome (None / Cancelled / Redirected / Substituted),
            // the material's presence on the battle area is re-checked at commit
            // time: a redirected/consumed material is pruned from the
            // transaction (cost recompute) and skipped by the material take.
        }

        self.finalize_digixros_play_after_leave_windows(
            player_id,
            target_card,
            total_reduction,
            source,
            suppress_on_play,
        )
    }

    /// After every DigiXros leave window has resolved: prune materials that left
    /// the battle area mid-window (redirected/consumed), recompute the reduced
    /// cost, and either commit the host play (if payable) or return the played
    /// card to hand for 0 memory (if the cost became unpayable — judge Q26/Q27).
    /// Wrap the live `pending_selection`'s callback + on_decline so that once the
    /// parked DigiXros leave reward settles (and any nested selections drain),
    /// the leave-window loop re-enters at `cont.next_material`. Mirrors the
    /// `<Delay>` continuation arming in `lower_replacement.rs`.
    pub(crate) fn arm_digixros_resume_after_parked_leave(
        &mut self,
        cont: DigiXrosResumeContinuation,
    ) {
        let Some(mut selection) = self.pending_selection.take() else {
            self.continue_digixros_after_parked_leave(cont);
            return;
        };

        // Resume-aware (make-engine-cloneable Batch 2): defer onto the resume
        // channel when the parked select is data-driven (closure bypassed).
        if self.pending_selection_resume.is_some() {
            self.pending_selection = Some(selection);
            self.after_selection_resume_hooks
                .0
                .push(crate::resume::AfterSelectionHook::DigiXrosLeaveContinuation { cont });
            return;
        }

        let original_callback = selection.callback;
        let cont_cb = cont.clone();
        selection.callback = Box::new(move |game, action_id| {
            original_callback(game, action_id);
            game.continue_digixros_after_parked_leave(cont_cb);
        });

        let original_decline = selection.on_decline.take();
        selection.on_decline = Some(Box::new(move |game| {
            if let Some(original_decline) = original_decline {
                original_decline(game);
            }
            game.continue_digixros_after_parked_leave(cont);
        }));

        self.pending_selection = Some(selection);
    }

    /// Re-enter the DigiXros leave-window loop after a parked reward settled. If
    /// the reward surfaced further selections, re-arm onto them; otherwise resume
    /// the loop at the next material (eventually reaching `finalize_...`).
    pub(crate) fn continue_digixros_after_parked_leave(
        &mut self,
        cont: DigiXrosResumeContinuation,
    ) {
        if self.pending_selection.is_some() {
            self.arm_digixros_resume_after_parked_leave(cont);
            return;
        }
        // The transaction may have been cleared if the play already aborted.
        if self.pending_digixros_transaction.is_none() {
            self.restore_digixros_limbo_to_battle_area(cont.player);
            return;
        }
        let _ = self.run_digixros_leave_windows_then_commit(
            cont.player,
            cont.target_card,
            cont.total_reduction,
            cont.source,
            cont.suppress_on_play,
            cont.next_material,
        );
    }

    pub(crate) fn finalize_digixros_play_after_leave_windows(
        &mut self,
        player_id: PlayerId,
        target_card: crate::card_source::CardHandle,
        total_reduction: i32,
        source: PlaySource,
        suppress_on_play: bool,
    ) -> PlayFromHandCostResult {
        // Restore any leaving material STILL parked in limbo (the leave observer
        // declined / was ineligible to extract it) back to the battle area so it
        // is consumed under the DigiXros host as a normal material. Materials the
        // observer DID extract were already re-materialized into their merged
        // permanent and removed from limbo, so this only restores the declines.
        self.restore_digixros_limbo_to_battle_area(player_id);

        // Prune battle-area materials that are no longer on the field (a
        // replacement redirected/consumed them away). Dropping them from the
        // transaction both fixes `final_cost()` (their negative cost_delta no
        // longer applies) and prevents `commit_digixros_material_sources` from
        // trying to consume a vanished permanent.
        let materials_pruned = {
            let live_on_field: std::collections::HashSet<crate::card_source::CardHandle> = self
                .player(player_id)
                .battle_area
                .iter()
                .map(|p| p.top_card().handle())
                .collect();
            if let Some(tx) = self.pending_digixros_transaction.as_mut() {
                let before = tx.material_count();
                tx.retain_materials(|origin| match origin {
                    DigiXrosMaterialOrigin::BattleArea { card, .. } => {
                        live_on_field.contains(&card)
                    }
                    _ => true,
                });
                tx.material_count() < before
            } else {
                false
            }
        };

        // Recompute the reduced cost from the materials that actually remain,
        // mirroring `commit_play_from_hand_after_reductions`:
        // `(transaction.final_cost() - total_reduction).max(0)`. The
        // (now-pruned) transaction's `final_cost()` reflects the materials that
        // truly survived the leave windows.
        let transaction_cost = self
            .pending_digixros_transaction
            .as_ref()
            .map(|tx| tx.final_cost() as i32)
            .unwrap_or(0);
        let effective_cost = (transaction_cost - total_reduction).max(0) as u16;

        // Declare-then-pay re-check (judge Q26/Q27): the play is unpayable — and
        // returns to hand for 0 memory — if EITHER the recomputed memory cost
        // overdraws past the memory floor, OR a material that was present at
        // declaration LEFT mid-resolution (a `WhenWouldLeaveBattleArea` window
        // redirected/DNA-extracted it) and the DigiXros recipe is now no longer
        // satisfied. The recipe check is gated on `materials_pruned` so the
        // engine's existing lenient path — declaring a DigiXros host with fewer
        // materials than the recipe `min` and paying base cost — is unchanged;
        // only a mid-resolution departure of an already-counted material can flip
        // the play to "unpayable → returns to hand".
        let recipe_now_unsatisfied = materials_pruned
            && self
                .pending_digixros_transaction
                .as_ref()
                .is_some_and(|tx| !tx.recipe_is_satisfied());
        let memory_affordable =
            (self.memory as i32 - effective_cost as i32) >= self.rules.memory_range.0 as i32;
        let payable = !recipe_now_unsatisfied && memory_affordable;
        if !payable {
            self.pending_digixros_transaction = None;
            // The played card never left the hand, and no memory was paid — the
            // unpayable play simply does not happen. This is the faithful
            // "returns to hand for 0 memory" outcome.
            return PlayFromHandCostResult::Failed;
        }

        let committed = self.commit_play_from_hand_card_no_replace(
            player_id,
            target_card,
            effective_cost,
            source == PlaySource::ByEffect,
            suppress_on_play,
        );
        self.pending_digixros_transaction = None;
        committed
            .map(PlayFromHandCostResult::Played)
            .unwrap_or(PlayFromHandCostResult::Failed)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn wrap_pending_play_cost_continuation(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: CostTargetContext,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        accumulated_reduction: i32,
        processed: Vec<CostReductionKey>,
    ) {
        let Some(mut pending) = self.pending_selection.take() else {
            return;
        };

        // Resume-aware (make-engine-cloneable Batch 2): a resume-driven select's
        // closure callback is bypassed by `run_resume`, so defer the play-cost
        // continuation onto the resume channel (runs after the resume resolution
        // for accept AND decline — both branches below run the same call).
        if self.pending_selection_resume.is_some() {
            self.pending_selection = Some(pending);
            self.after_selection_resume_hooks.0.push(
                crate::resume::AfterSelectionHook::PlayCostContinuation {
                    player_id,
                    hand_index,
                    target,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    accumulated_reduction,
                    processed,
                },
            );
            return;
        }

        let original_callback = pending.callback;
        let accept_processed = processed.clone();
        pending.callback = Box::new(move |game: &mut Game, action_id: u16| {
            original_callback(game, action_id);
            game.resume_play_cost_continuation_after_pending(
                player_id,
                hand_index,
                target,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                accumulated_reduction,
                accept_processed,
            );
        });

        let decline_processed = processed;
        let original_decline = pending.on_decline.take();
        pending.on_decline = Some(Box::new(move |game: &mut Game| {
            if let Some(original_decline) = original_decline {
                original_decline(game);
            }
            game.resume_play_cost_continuation_after_pending(
                player_id,
                hand_index,
                target,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                accumulated_reduction,
                decline_processed,
            );
        }));

        self.pending_selection = Some(pending);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn resume_play_cost_continuation_after_pending(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: CostTargetContext,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        mut accumulated_reduction: i32,
        processed: Vec<CostReductionKey>,
    ) {
        if self.pending_selection.is_some() {
            self.wrap_pending_play_cost_continuation(
                player_id,
                hand_index,
                target,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                accumulated_reduction,
                processed,
            );
            return;
        }
        // VARIABLE-amount interactive delete cost
        // (`G-ENGINE-COST-REDUCTION-INTERACTIVE-DELETE-COST`, BT13-103): a
        // `delete_for_cost_reduction` step inside the just-resolved parked
        // pay_cost recorded the deleted permanent's printed play cost here. Credit
        // it now (the parked select has resolved and the delete has happened, so
        // the amount is finally known) and clear the slot so it never leaks into a
        // later play.
        if let Some(extra) = self.pending_cost_reduction_amount_override.take() {
            accumulated_reduction += extra;
        }
        let _ = self.continue_play_from_hand_cost_reduction_chain(
            player_id,
            hand_index,
            target,
            cost_delta,
            source,
            origin,
            suppress_on_play,
            accumulated_reduction,
            processed,
        );
    }

    pub(crate) fn run_digixros_material_selection_step(
        &mut self,
        state: crate::resume::DigiXrosMaterialSelectionState,
        action_id: u16,
        is_pass: bool,
    ) {
        if is_pass {
            let _ = self.commit_play_from_hand_after_reductions(
                state.player_id,
                state.hand_index,
                state.target_card,
                state.cost_delta,
                state.source,
                state.origin,
                state.suppress_on_play,
                state.total_reduction,
            );
            return;
        }

        let Some((_, material_origin)) = state
            .candidates
            .iter()
            .find(|(candidate_action, _)| *candidate_action == action_id)
            .copied()
        else {
            return;
        };
        let Some(card_data) = self.card_data_for_handle(material_origin.card()).cloned() else {
            return;
        };
        if let Some(transaction) = self.pending_digixros_transaction.as_mut() {
            let _ = transaction.try_select_material(material_origin, &card_data);
        }
        let _ = self.finish_play_from_hand_after_reductions(
            state.player_id,
            state.hand_index,
            state.target_card,
            state.cost_delta,
            state.source,
            state.origin,
            state.suppress_on_play,
            state.total_reduction,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn install_pending_digixros_material_selection_or_finish(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: crate::card_source::CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> bool {
        let Some(transaction) = self.pending_digixros_transaction.as_ref() else {
            return false;
        };
        if transaction.played_card != target_card || transaction.controller != player_id {
            return false;
        }

        let candidates = self.pending_digixros_material_candidates(player_id);
        if candidates.is_empty() && transaction.material_count() == 0 {
            return false;
        }

        // DCGO `CanNoSelect` (BT15_102.cs): the player may STOP selecting
        // materials only while the remaining reduced cost is payable
        // (`PayingCost - 4*placed <= MaxMemoryCost`). For a printed-DigiXros
        // host the mask already gated the play on the printed cost, and
        // adding materials only lowers it, so `can_stop` stays true there —
        // this only bites cast-time-assembly plays whose printed cost
        // exceeds the payable range (BT15-102's 15). With no candidates left
        // the stop must stay legal regardless (the selection would otherwise
        // have no legal action); an unpayable commit then routes through the
        // declare-then-pay return-to-hand flow.
        let stop_cost = (transaction.final_cost() as i32 - total_reduction).max(0);
        let can_stop = candidates.is_empty()
            || (self.memory as i32 - stop_cost) >= self.rules.memory_range.0 as i32;

        let valid_action_ids = candidates
            .iter()
            .map(|(action_id, _)| *action_id)
            .collect::<Vec<_>>();
        let resume_candidates = candidates.clone();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Material,
            selecting_player: player_id,
            previous_phase,
            valid_action_ids,
            is_optional: can_stop,
            prompt: "Select DigiXros material".to_string(),
            effect_choices: None,
            source_card: target_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let Some((_, origin_ref)) = candidates
                    .iter()
                    .find(|(candidate_action, _)| *candidate_action == action_id)
                else {
                    return;
                };
                let material_origin = *origin_ref;
                let Some(card_data) = game.card_data_for_handle(material_origin.card()).cloned()
                else {
                    return;
                };
                if let Some(transaction) = game.pending_digixros_transaction.as_mut() {
                    let _ = transaction.try_select_material(material_origin, &card_data);
                }
                let _ = game.finish_play_from_hand_after_reductions(
                    player_id,
                    hand_index,
                    target_card,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    total_reduction,
                );
            }),
            on_decline: Some(Box::new(move |game: &mut Game| {
                let _ = game.commit_play_from_hand_after_reductions(
                    player_id,
                    hand_index,
                    target_card,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    total_reduction,
                );
            })),
        });
        self.pending_selection_resume = Some(crate::resume::ResumeStack {
            frames: vec![crate::resume::ResumeFrame::DigiXrosMaterialSelection(
                crate::resume::DigiXrosMaterialSelectionState {
                    player_id,
                    hand_index,
                    target_card,
                    cost_delta,
                    source,
                    origin,
                    suppress_on_play,
                    total_reduction,
                    candidates: resume_candidates,
                    outer_conts: Vec::new(),
                },
            )],
        });
        true
    }

    /// Try the Assembly alt-path before the normal play finish. Returns
    /// `Some(Pending)` when an Assembly selection flow was installed; `None`
    /// when the card is not assembly-capable (caller then finishes normally).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn assembly_or_finish_play_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> PlayFromHandCostResult {
        #[cfg(feature = "dsl-yaml-loader")]
        {
            if let Some(result) = self.try_begin_assembly_flow(
                player_id,
                hand_index,
                target_card,
                cost_delta,
                source,
                origin,
                suppress_on_play,
                total_reduction,
            ) {
                return result;
            }
        }
        self.finish_play_from_hand_after_reductions(
            player_id,
            hand_index,
            target_card,
            cost_delta,
            source,
            origin,
            suppress_on_play,
            total_reduction,
        )
    }

    /// Install the Assembly flow when the played card has an `assembly`
    /// alt-path whose materials are satisfiable from the controller's trash.
    /// Returns `Some(Pending)` if a selection was installed, else `None`.
    #[cfg(feature = "dsl-yaml-loader")]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_begin_assembly_flow(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target_card: CardHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
        origin: PendingWouldPlayOrigin,
        suppress_on_play: bool,
        total_reduction: i32,
    ) -> Option<PlayFromHandCostResult> {
        // Assembly is a hand-play mechanic; only attempt for genuine hand plays.
        if !matches!(origin, PendingWouldPlayOrigin::Hand) {
            return None;
        }

        let (assembly_reduction, elements) =
            self.resolve_eligible_assembly(player_id, hand_index, target_card)?;

        let params = AssemblyPlayParams {
            cost_delta,
            source,
            origin,
            suppress_on_play,
            total_reduction,
            assembly_reduction,
        };
        install_assembly_element(
            self,
            player_id,
            target_card,
            params,
            std::sync::Arc::new(elements),
            0,
            Vec::new(),
        );
        Some(PlayFromHandCostResult::Pending)
    }

    pub(crate) fn restore_pending_would_play_origin(&mut self, resume: PendingWouldPlayResume) {
        let Some(hand_index) = self
            .player(resume.player)
            .hand
            .iter()
            .position(|card| card.handle() == resume.card)
        else {
            return;
        };
        match resume.origin {
            PendingWouldPlayOrigin::Hand => {}
            PendingWouldPlayOrigin::Trash { index } => {
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let insert_at = index.min(self.player(resume.player).trash.len());
                self.player_mut(resume.player).trash.insert(insert_at, card);
            }
            PendingWouldPlayOrigin::SecurityTop { was_face_up } => {
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let card_index = card.card_index;
                self.player_mut(resume.player).security.push(card);
                if was_face_up {
                    self.player_mut(resume.player)
                        .face_up_security
                        .insert(card_index);
                }
            }
            PendingWouldPlayOrigin::Reveal { index } => {
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let insert_at = index.min(self.revealed_cards.len());
                self.revealed_cards.insert(insert_at, card);
            }
            PendingWouldPlayOrigin::Source {
                permanent,
                source_index,
            } => {
                if permanent.player != resume.player {
                    return;
                }
                let card = self.player_mut(resume.player).hand.remove(hand_index);
                let Some(perm) = self
                    .player_mut(resume.player)
                    .battle_area
                    .get_mut(permanent.index as usize)
                else {
                    self.player_mut(resume.player).trash.push(card);
                    return;
                };
                let insert_at = source_index.min(perm.card_sources.len());
                perm.card_sources.insert(insert_at, card);
            }
        }
    }

    pub(crate) fn commit_play_from_hand_card_no_replace(
        &mut self,
        player_id: PlayerId,
        target_card: crate::card_source::CardHandle,
        effective_cost: u16,
        effect_initiated: bool,
        suppress_on_play: bool,
    ) -> Option<usize> {
        if self.player(player_id).battle_area.len() >= self.rules.field_slots as usize {
            return None;
        }
        let hand_index = self
            .player(player_id)
            .hand
            .iter()
            .position(|card| card.handle() == target_card)?;
        if !self.pay_memory(effective_cost) {
            return None;
        }

        let turn = self.turn_count;
        let card = self.player_mut(player_id).hand.remove(hand_index);
        let mut perm = crate::permanent::Permanent::new(card, turn);
        // G-PLAY-ENTERS-SUSPENDED (Q28 / EX5-060 "plays ... suspended"):
        // a flagged play enters the battle area suspended, BEFORE the
        // play-event triggers fire (observers see the suspended state).
        if self.play_enters_suspended {
            self.play_enters_suspended = false;
            perm.is_suspended = true;
        }
        self.player_mut(player_id).battle_area.push(perm);
        let field_index = self.player(player_id).battle_area.len() - 1;
        let mut entered = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.pending_digixros_transaction.is_some() {
            entered = self.commit_digixros_material_sources(entered);
        }
        let entered_index = entered.index as usize;

        // G-ASSEMBLY-PLAY-EXECUTION: place assembled materials at the BOTTOM of
        // the new permanent's digivolution stack BEFORE its [On Play] /
        // [When Digivolving] effects fire — so play effects reading this
        // Digimon's own digivolution-card count (e.g. AD1-025 Omnimon's bounce)
        // observe the assembled materials. Consumed here regardless of which
        // commit path (direct or `commit_pending_would_play` resume) ran.
        // Assembly and DigiXros are mutually exclusive play paths
        // (`pending_assembly_materials` vs `pending_digixros_transaction`), so
        // for an assembly play the DigiXros relocation above is a no-op and
        // `entered_index == field_index`.
        if let Some((played, materials)) = self.pending_assembly_materials.take() {
            if played == target_card {
                for h in materials {
                    let Some(pos) = self
                        .player(player_id)
                        .trash
                        .iter()
                        .position(|c| c.handle() == h)
                    else {
                        continue;
                    };
                    let cs = self.player_mut(player_id).trash.remove(pos);
                    if let Some(perm) = self
                        .player_mut(player_id)
                        .battle_area
                        .get_mut(entered_index)
                    {
                        perm.push_under(cs);
                    } else {
                        self.player_mut(player_id).trash.push(cs);
                    }
                }
            } else {
                // Mismatch (would only arise under unusual interleaving):
                // restore the slot for the correct card's commit.
                self.pending_assembly_materials = Some((played, materials));
            }
        }

        let top_card = self.players[player_id as usize].battle_area[entered_index].top_card();
        let emitted_card_id = top_card.card_id(&self.card_data).to_string();
        let emitted_card_name = top_card.card_name(&self.card_data).to_string();
        let cost_printed = self.card_data[top_card.data_index].play_cost as i16;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
            card_name: emitted_card_name,
            field_index: entered.index,
            // Standard PLAY-from-hand path: `effective_cost` is the
            // post-discount memory actually paid (after tamer / other
            // generic reductions). Not an alt-path play — `via_alt_path`
            // is None even when a discount was applied.
            cost_paid: effective_cost as i16,
            cost_printed,
            via_alt_path: None,
        });

        // PUPPETS-G030 — `suppress_on_play` skips ONLY the played card's
        // own `[On Play]` broadcast; observer broadcasts
        // (`OnEnterFieldAnyone` / `OnAllyPlayed`) are unchanged. The
        // helper internally wraps all three broadcasts in
        // `enter_deferred_drain` / `exit_deferred_drain_and_flush` so
        // simultaneous triggers share a TriggerOrder bundle — see
        // `Game::fire_play_event_triggers` for the contract.
        self.fire_play_event_triggers(player_id, entered_index, effect_initiated, suppress_on_play);

        Some(entered_index)
    }

    pub(crate) fn take_digixros_material_origin(
        &mut self,
        origin: DigiXrosMaterialOrigin,
    ) -> Option<CardSource> {
        match origin {
            DigiXrosMaterialOrigin::Hand { player, card, .. } => {
                let idx = self
                    .player(player)
                    .hand
                    .iter()
                    .position(|candidate| candidate.handle() == card)?;
                Some(self.player_mut(player).hand.remove(idx))
            }
            DigiXrosMaterialOrigin::Trash { player, card, .. } => {
                let idx = self
                    .player(player)
                    .trash
                    .iter()
                    .position(|candidate| candidate.handle() == card)?;
                Some(self.player_mut(player).trash.remove(idx))
            }
            DigiXrosMaterialOrigin::UnderTamer {
                tamer,
                source_index,
                card,
            } => {
                let permanent = self
                    .player_mut(tamer.player)
                    .battle_area
                    .get_mut(tamer.index as usize)?;
                if permanent
                    .card_sources
                    .get(source_index)
                    .is_some_and(|candidate| candidate.handle() == card)
                {
                    Some(permanent.card_sources.remove(source_index))
                } else {
                    let idx = permanent
                        .card_sources
                        .iter()
                        .position(|candidate| candidate.handle() == card)?;
                    Some(permanent.card_sources.remove(idx))
                }
            }
            DigiXrosMaterialOrigin::BattleArea { permanent, card } => {
                let player = permanent.player;
                let idx = self
                    .player(player)
                    .battle_area
                    .get(permanent.index as usize)
                    .filter(|candidate| candidate.top_card().handle() == card)
                    .map(|_| permanent.index as usize)
                    .or_else(|| {
                        self.player(player)
                            .battle_area
                            .iter()
                            .position(|candidate| candidate.top_card().handle() == card)
                    })?;
                let mut removed = self.player_mut(player).battle_area.remove(idx);
                let top = removed.card_sources.pop()?;
                if !removed.card_sources.is_empty() {
                    self.player_mut(player).trash.extend(removed.card_sources);
                }
                Some(top)
            }
        }
    }

    pub(crate) fn enqueue_when_digivolving_from_arts_card(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
    ) {
        let Some(effects) = self.effects_for_card(card_id, card_handle) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = owner == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::WhenDigivolving {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Digimon,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: owner,
                timing: EffectTiming::WhenDigivolving,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    /// Phase 9 Task 3 — enqueue every `CounterEffect`-timing effect on the
    /// in-flight `pending_option` card. Mirrors
    /// `enqueue_option_main_from_pending` but filters on `CounterEffect`
    /// so a hand Counter Option's body fires BEFORE its `OptionMain` body.
    /// Called only when `in_counter_window` is set.
    pub(crate) fn enqueue_counter_effect_from_pending(
        &mut self,
        card_id: &str,
        card_handle: crate::card_source::CardHandle,
        owner: PlayerId,
    ) {
        let Some(effects) = self.effects_for_card(card_id, card_handle) else {
            return;
        };
        let tp = self.turn_player();
        let is_turn_player = owner == tp;
        for (slot, effect) in effects.iter().enumerate() {
            if effect.timing != EffectTiming::CounterEffect {
                continue;
            }
            if !effect.counter {
                continue;
            }
            self.effect_queue.push_back(QueuedEffect {
                source_card: card_handle,
                source_permanent: None,
                source_kind: EffectSourceKind::Option,
                attribution_source_card: None,
                attribution_source_kind: None,
                bypass_once_per_turn: false,
                controller: owner,
                timing: EffectTiming::CounterEffect,
                trigger_context: None,
                effect_slot: slot as u8,
                is_optional: effect.optional,
                is_turn_player,
                card_id: card_id.to_string(),
                allow_below_top_liveness: false,
                dna_origin_context: self.current_dna_origin,
                granted_effect_id: None,
            });
        }
    }

    pub(crate) fn commit_linked_card_no_replace(&mut self, resume: PendingWouldLinkResume) {
        let Some(pending) = self.pending_option.take() else {
            return;
        };

        if pending.card.handle() != resume.card {
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        let host = resume.host;
        let host_live = self
            .player(host.player)
            .battle_area
            .get(host.index as usize)
            .map(|p| {
                self.permanent_is_digimon_for_rules(host)
                    && matches!(p.option_state, crate::permanent::OptionState::Standard)
            })
            .unwrap_or(false);
        if !host_live {
            self.player_mut(pending.owner).trash.push(pending.card);
            self.check_turn_end();
            return;
        }

        // Attach.
        let linked_card = pending.card.handle();
        self.player_mut(host.player).battle_area[host.index as usize]
            .linked_cards
            .push(pending.card);

        self.enqueue_triggered(
            EffectTiming::OnOptionPlaced,
            TriggerSource::OptionPlaced {
                player: pending.owner,
                permanent: None,
                linked_host: Some(host),
                card: linked_card,
            },
        );
        self.drain_effect_queue();
        if self.pending_selection.is_some() {
            self.pending_option_placed_link_resume = Some((host, linked_card));
            return;
        }

        self.fire_on_link_after_option_placed(host, linked_card);
    }

    pub(crate) fn insert_card_into_deck(
        &mut self,
        player_id: PlayerId,
        card: CardSource,
        position: crate::enums::StackPosition,
    ) {
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.push(card);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.insert(0, card);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.insert(idx, card);
            }
        }
    }

    pub(crate) fn insert_stack_into_deck(
        &mut self,
        player_id: PlayerId,
        stack: Vec<CardSource>,
        position: crate::enums::StackPosition,
    ) {
        match position {
            crate::enums::StackPosition::Top => {
                self.player_mut(player_id).deck.extend(stack);
            }
            crate::enums::StackPosition::Bottom => {
                self.player_mut(player_id).deck.splice(0..0, stack);
            }
            crate::enums::StackPosition::Random => {
                use rand::Rng;
                let deck_len = self.player(player_id).deck.len();
                let idx = if deck_len == 0 {
                    0
                } else {
                    self.rng.gen_range(0..=deck_len)
                };
                self.player_mut(player_id).deck.splice(idx..idx, stack);
            }
        }
    }

    pub(crate) fn insert_stack_into_owners_decks(
        &mut self,
        stack: Vec<CardSource>,
        position: crate::enums::StackPosition,
    ) {
        let player_count = self.players.len();
        for player_id in 0..player_count {
            let owner = player_id as PlayerId;
            let owned_stack: Vec<CardSource> = stack
                .iter()
                .filter(|card| card.owner == owner)
                .cloned()
                .collect();
            if !owned_stack.is_empty() {
                self.insert_stack_into_deck(owner, owned_stack, position);
            }
        }
    }
}
