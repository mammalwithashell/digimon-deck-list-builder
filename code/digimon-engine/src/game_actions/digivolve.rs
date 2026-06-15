//! Digivolve / DNA-digivolve operations (Tier 2) — `impl Game`.

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
    /// Digivolve: push a card onto a permanent's stack.
    pub fn digivolve_onto(
        &mut self,
        player_id: PlayerId,
        field_index: usize,
        card: CardSource,
    ) -> bool {
        let turn = self.turn_count;
        let player = self.player_mut(player_id);
        if field_index >= player.battle_area.len() {
            return false;
        }
        player.battle_area[field_index].digivolve(card, turn);
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        true
    }

    /// Low-level source-attribution helper for tests and engine internals.
    ///
    /// Uses the standard De-Digivolve floor (`stop_at_level = Some(3)`) and
    /// returns whether at least one card was popped. Replacement windows are
    /// resolved by `EffectContext::de_digivolve` under the supplied source
    /// attribution. Production card effects should prefer an existing
    /// `EffectContext` so `can_affect_permanent` and source-kind metadata come
    /// from the real resolving card.
    #[doc(hidden)]
    pub fn de_digivolve_from_effect(
        &mut self,
        handle: PermanentHandle,
        effect_player: PlayerId,
        amount: u8,
    ) -> bool {
        let previous = self.effect_source_player;
        self.effect_source_player = Some(effect_player);
        let popped = {
            let mut ctx =
                EffectContext::new(self, crate::card_source::CardHandle(0), None, effect_player);
            ctx.de_digivolve(handle, Some(3), Some(amount))
        };
        self.effect_source_player = previous;
        popped > 0
    }

    /// Full "digivolve from hand" action — Python parity for
    /// `action_digivolve(field_idx, hand_idx)`. Validates phase, indices,
    /// `CannotDigivolve` modifier, and evo-cost fit; pays memory; removes
    /// the card from hand; stacks it onto the target permanent; draws 1;
    /// fires `WhenDigivolving` triggers and drains the effect queue;
    /// finally calls `check_turn_end`.
    ///
    /// Deferred (see RUST_PYTHON_PARITY.md):
    /// - Cost reductions (`WhenWouldDigivolve`, `CHANGE_DIGIVOLUTION_COST`)
    /// - `digivolve_observer` mechanism (no Rust equivalent yet)
    /// - Contextual modifier predicates (`{'digivolving_card': card}`)
    pub fn digivolve_from_hand(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) -> bool {
        self.digivolve_from_hand_inner(player_id, hand_index, field_index, source, false)
    }

    /// As [`Self::digivolve_from_hand`], but threads a `player_reducer_resolved`
    /// flag (`G-COST-REDUCE-ALLY-DIGIVOLVE`). When `false` (the default user
    /// call), the function first consults `Game::player_digivolve_cost_reducers`
    /// and — if a reducer qualifies and is payable — installs an interactive
    /// accept/decline prompt, returning `false` without performing the
    /// digivolution; the accept/decline callbacks re-invoke this function with
    /// the flag `true` and a pre-resolved `pending_player_digivolve_reduction`.
    /// When `true`, the player-scoped reducer prompt is skipped (it has already
    /// been resolved this attempt).
    pub(crate) fn digivolve_from_hand_inner(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
        player_reducer_resolved: bool,
    ) -> bool {
        if self.current_phase != GamePhase::Main {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: not in Main phase (phase={:?})",
                self.current_phase
            ));
            return false;
        }
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        if field_index >= player.battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: field index {} out of range (battle_area size={})",
                field_index,
                player.battle_area.len()
            ));
            return false;
        }
        let handle = PermanentHandle {
            player: player_id,
            index: field_index as u8,
        };
        if self.modifiers.has(handle, ModifierType::CannotDigivolve) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: permanent at field index {} blocked by CannotDigivolve modifier",
                field_index
            ));
            return false;
        }

        let card = player.hand[hand_index].clone();
        let perm = &player.battle_area[field_index];
        let Some(route) = self.normal_digivolve_route_for_hand_card(player_id, hand_index, handle)
        else {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: card {} cannot digivolve onto {} (evo-cost mismatch)",
                card.card_id(&self.card_data),
                perm.top_card().card_id(&self.card_data),
            ));
            return false;
        };
        let printed_cost = route.memory_cost;

        // G-COST-REDUCE-ALLY-DIGIVOLVE — player-scoped one-shot future-digivolve
        // cost reducer prompt. Runs BEFORE the synchronous field-hosted
        // BeforePayCost scan; if a reducer qualifies and is payable, this
        // installs an interactive accept/decline PendingSelection and returns
        // — the accept/decline callbacks re-enter `digivolve_from_hand_inner`
        // with `player_reducer_resolved = true`.
        if !player_reducer_resolved
            && self.try_prompt_player_digivolve_cost_reducer(
                player_id,
                handle,
                hand_index,
                field_index,
                source,
            )
        {
            return false;
        }
        // Pre-resolved player-scoped reduction (set by the accept callback,
        // 0 on decline). Consumed once here.
        let player_reduction = std::mem::take(&mut self.pending_player_digivolve_reduction);

        // Pass the hand card being digivolved into as the cost-target so
        // target-aware predicates (`cost_target: { trait_has: Free }`)
        // can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. `handle` is the
        // permanent being digivolved into; threaded as a target permanent
        // so `source_is_cost_target_permanent` gates self-targeted
        // observers.
        let target = CostTargetContext {
            card: card.handle(),
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(handle), None],
        };

        // G-COST-REDUCTION-INTERACTIVE-PAY-COST (digivolve half) — a
        // field-hosted reducer whose `pay_cost` INSTALLS A SELECTION (the
        // interactive `trash_bottom_face_down_source_under_tamer` Tamer pick,
        // ST23-03 / ST23-11) cannot be resolved by the synchronous scan below
        // (a park would leave a dangling `pending_selection` mid-digivolve).
        // Handle it through a dedicated interactive prompt FIRST: it runs the
        // parking pay_cost, credits the reduction into
        // `pending_interactive_digivolve_reduction` on resolution, and
        // re-enters with `player_reducer_resolved = true`. The synchronous scan
        // skips `pay_cost_interactive` reducers so they are never
        // double-applied. Runs only on the first attempt
        // (`!player_reducer_resolved`).
        if !player_reducer_resolved
            && self.try_prompt_interactive_digivolve_cost_reducer(
                player_id,
                target,
                hand_index,
                field_index,
                source,
            )
        {
            return false;
        }
        // Pre-resolved interactive field reduction (set by the prompt's parked
        // pay_cost success continuation, 0 otherwise). Consumed once here.
        let interactive_reduction =
            std::mem::take(&mut self.pending_interactive_digivolve_reduction);

        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(target),
        ) + player_reduction
            + interactive_reduction;
        // Fire BeforePayCost observers (e.g. gain_memory) AFTER reduction
        // is computed but BEFORE pay_memory — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(target));
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        self.pending_would_digivolve_resume = Some(PendingWouldDigivolveResume {
            player: player_id,
            permanent: handle,
            card: card.handle(),
            effective_cost,
        });
        let cause = match source {
            PlaySource::ByEffect => crate::replacement::ReplacementCause::OwnEffect,
            PlaySource::ByHand | PlaySource::ByDigivolve => {
                crate::replacement::ReplacementCause::OwnEffect
            }
        };
        let outcome = self.try_replace(
            EffectTiming::WhenPermanentWouldDigivolve,
            crate::replacement::ReplacementSubject::Permanent(handle),
            cause,
            Some(Zone::BattleArea),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        match outcome {
            crate::replacement::ReplacementOutcome::None => {
                self.pending_would_digivolve_resume = None;
            }
            crate::replacement::ReplacementOutcome::Cancelled
            | crate::replacement::ReplacementOutcome::CustomHandled => {
                self.pending_would_digivolve_resume = None;
                return false;
            }
            crate::replacement::ReplacementOutcome::Redirected(_)
            | crate::replacement::ReplacementOutcome::Substituted(_) => {
                self.pending_would_digivolve_resume = None;
                return false;
            }
        }

        self.commit_digivolve_from_hand_no_replace(PendingWouldDigivolveResume {
            player: player_id,
            permanent: handle,
            card: card.handle(),
            effective_cost,
        })
    }

    /// Install the accept/decline `PendingSelection` for a player-scoped
    /// digivolve cost reducer. On accept → install the suspend-cost
    /// selection, then re-enter the digivolve with the reduction applied.
    /// On decline → re-enter the digivolve at full cost (reducer stays
    /// armed). `G-COST-REDUCE-ALLY-DIGIVOLVE`.
    pub(crate) fn install_player_digivolve_reducer_prompt(
        &mut self,
        reducer_idx: usize,
        reducer: crate::player_cost_reducer::PlayerDigivolveCostReducer,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        use crate::action::space::HAND_EFFECT_START;
        use crate::selection::{EffectChoiceEntry, PendingSelection, SelectionKind};

        let source_card = reducer.source_card;
        let source_kind = self.effect_source_kind_for_handle(source_card);
        let amount = reducer.amount;

        // Accept branch — pay the suspend cost (if any), apply the reduction,
        // consume the reducer if single-fire, then re-enter the digivolve.
        let accept = {
            let reducer = reducer.clone();
            move |game: &mut Game, _action_id: u16| {
                game.player_digivolve_reducer_accept(
                    reducer_idx,
                    reducer,
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                );
            }
        };
        // Decline branch — leave the reducer armed, re-enter the digivolve
        // at the unreduced cost.
        let decline = move |game: &mut Game| {
            game.pending_player_digivolve_reduction = 0;
            game.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
        };

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::EffectChoice;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::EffectChoice,
            selecting_player: acting_player,
            previous_phase,
            valid_action_ids: vec![HAND_EFFECT_START],
            is_optional: true,
            prompt: format!(
                "Suspend 1 of your Digimon to reduce the digivolution cost by {}?",
                amount
            ),
            effect_choices: Some(vec![EffectChoiceEntry {
                label: format!("Suspend 1 Digimon (digivolution cost -{})", amount),
                action_id: HAND_EFFECT_START,
                source_card: Some(source_card),
                source_kind: Some(source_kind),
                timing: Some(crate::enums::EffectTiming::BeforePayCost),
                is_optional: true,
                observation_metadata: Default::default(),
            }]),
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(accept),
            on_decline: Some(Box::new(decline)),
        });
    }

    /// Accept-branch continuation for a player-scoped digivolve cost
    /// reducer: install the suspend-cost selection (`select 1 unsuspended
    /// own Digimon`). On suspend resolution → suspend it, apply the
    /// reduction, consume the reducer if single-fire, and re-enter the
    /// digivolve. `G-COST-REDUCE-ALLY-DIGIVOLVE` / `G-PAY-COST-SELECT-ARBITRARY-SUSPEND`.
    pub(crate) fn player_digivolve_reducer_accept(
        &mut self,
        reducer_idx: usize,
        reducer: crate::player_cost_reducer::PlayerDigivolveCostReducer,
        acting_player: PlayerId,
        hand_index: usize,
        field_index: usize,
        source: PlaySource,
    ) {
        use crate::action::space::{encode_attack, ATTACK_START, TARGETS_PER_ATTACKER};
        use crate::selection::{PendingSelection, SelectionKind};

        let amount = reducer.amount;
        let single_fire = reducer.single_fire;
        let source_card = reducer.source_card;
        let source_kind = self.effect_source_kind_for_handle(source_card);

        if !reducer.suspend_cost {
            // No suspend cost — apply the reduction directly.
            self.consume_player_digivolve_reducer(reducer_idx, &reducer, single_fire);
            self.pending_player_digivolve_reduction = amount;
            self.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
            return;
        }

        let suspendable = self.suspendable_own_digimon(acting_player);
        if suspendable.is_empty() {
            // Cost became unpayable between prompt-install and accept — leave
            // the reducer armed and continue at the unreduced cost.
            self.pending_player_digivolve_reduction = 0;
            self.digivolve_from_hand_inner(acting_player, hand_index, field_index, source, true);
            return;
        }

        let valid_action_ids: Vec<u16> = suspendable
            .iter()
            .map(|i| encode_attack(0, *i as u16))
            .collect();

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectTarget;
        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::OwnField,
            selecting_player: acting_player,
            previous_phase,
            valid_action_ids,
            is_optional: false,
            prompt: "Suspend 1 of your Digimon (digivolution cost reduction)".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                let offset = action_id.saturating_sub(ATTACK_START);
                let target_index = (offset % TARGETS_PER_ATTACKER) as u8;
                let suspend_target = PermanentHandle {
                    player: acting_player,
                    index: target_index,
                };
                game.suspend(suspend_target);
                game.consume_player_digivolve_reducer(reducer_idx, &reducer, single_fire);
                game.pending_player_digivolve_reduction = amount;
                game.digivolve_from_hand_inner(
                    acting_player,
                    hand_index,
                    field_index,
                    source,
                    true,
                );
            }),
            on_decline: None,
        });
    }

    /// Remove a single-fire player-scoped digivolve cost reducer after a
    /// successful application. The reducer is located by identity (player +
    /// source card + amount) rather than by stale index, since the vector
    /// may have shifted between prompt-install and resolution.
    pub(crate) fn consume_player_digivolve_reducer(
        &mut self,
        reducer_idx: usize,
        reducer: &crate::player_cost_reducer::PlayerDigivolveCostReducer,
        single_fire: bool,
    ) {
        if !single_fire {
            return;
        }
        // Prefer the recorded index when it still points at the same reducer;
        // otherwise re-locate by identity.
        if self
            .player_digivolve_cost_reducers
            .get(reducer_idx)
            .is_some_and(|r| {
                r.player == reducer.player
                    && r.source_card == reducer.source_card
                    && r.amount == reducer.amount
            })
        {
            self.player_digivolve_cost_reducers.remove(reducer_idx);
            return;
        }
        if let Some(pos) = self.player_digivolve_cost_reducers.iter().position(|r| {
            r.player == reducer.player
                && r.source_card == reducer.source_card
                && r.amount == reducer.amount
        }) {
            self.player_digivolve_cost_reducers.remove(pos);
        }
    }

    pub(crate) fn commit_digivolve_from_hand_no_replace(
        &mut self,
        resume: PendingWouldDigivolveResume,
    ) -> bool {
        let field_index = resume.permanent.index as usize;
        if resume.permanent.player != resume.player {
            return false;
        }
        if self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .is_none()
        {
            return false;
        }
        if self
            .modifiers
            .has(resume.permanent, ModifierType::CannotDigivolve)
        {
            return false;
        }

        let Some(hand_index) = self
            .player(resume.player)
            .hand
            .iter()
            .position(|card| card.handle() == resume.card)
        else {
            return false;
        };
        let Some(route) =
            self.normal_digivolve_route_for_hand_card(resume.player, hand_index, resume.permanent)
        else {
            return false;
        };
        let is_app_fusion = route.app_fusion;

        let (from_stack_top, top_card_id) = {
            let player = self.player(resume.player);
            let perm = &player.battle_area[field_index];
            let card = &player.hand[hand_index];
            (
                perm.top_card().card_id(&self.card_data).to_string(),
                card.card_id(&self.card_data).to_string(),
            )
        };

        if !self.pay_memory(resume.effective_cost) {
            self.logger.log(&format!(
                "[Rejected] digivolve_from_hand: cannot pay memory cost {} (current memory={})",
                resume.effective_cost, self.memory
            ));
            return false;
        }

        let turn = self.turn_count;
        let removed = self.player_mut(resume.player).hand.remove(hand_index);
        self.player_mut(resume.player).battle_area[field_index].digivolve(removed, turn);
        // App Fusion: after stacking the App-Fusion card on top of the host
        // (the digivolve above), the host's existing LINKED cards are moved
        // UNDER the new top as digivolution sources (consumed). Mirrors DCGO
        // `CardController.cs` `AddToSources(LinkedCard(card))`. Order is
        // preserved bottom-to-top by draining front-to-back through
        // `push_under` (which inserts at position 0): draining in reverse so
        // the first linked card ends up deepest.
        if is_app_fusion {
            let linked: Vec<crate::card_source::CardSource> = std::mem::take(
                &mut self.player_mut(resume.player).battle_area[field_index].linked_cards,
            );
            for card in linked.into_iter().rev() {
                self.player_mut(resume.player).battle_area[field_index].push_under(card);
            }
        }
        let event_card = self
            .player(resume.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("digivolve target remains in battle area after stack mutation");

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: resume.player,
            top_card_id,
            field_index: field_index as u8,
            from_stack_top,
            // Regular evo-cost path from hand — not DNA, not Blast DNA.
            was_dna: false,
            was_blast_dna: false,
            memory_paid: resume.effective_cost as i16,
        });

        self.player_mut(resume.player).draw();

        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(resume.permanent),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — fires in every player's battle area
        // after the evolving permanent's WhenDigivolving resolves. Distinct
        // from WhenDigivolving (self-timing on the evolving permanent).
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: resume.player,
                permanent: resume.permanent,
                card: event_card,
                effect_initiated: false,
                dna_origin: false,
            },
        );
        self.drain_effect_queue();

        // Reward-shaping counter — bumped here at the user-action choke
        // point that both `digivolve_from_hand_inner` and the
        // replacement-resume path (`commit_pending_would_digivolve`)
        // funnel through. Effect-initiated digivolves (`digivolve_onto`)
        // intentionally do NOT bump. See
        // docs/superpowers/specs/2026-05-23-digivolve-reward-shaping-design.md.
        self.n_digivolutions[resume.player as usize] += 1;

        self.check_turn_end();
        true
    }

    /// Commit an effect-initiated App Fuse: stack `result_card` (pulled from
    /// `zone`) on top of `host`, then fold `host`'s existing linked cards under
    /// the new top as digivolution sources (consumed) — exactly like the
    /// alt-play app-fusion commit. App Fusion is printed Cost 0 and bypasses the
    /// normal evo-cost / level / colour requirement check (it is gated only by
    /// the host having the named materials, validated by the caller). Fires
    /// WhenDigivolving + OnDigivolve with `effect_initiated: true`. Returns true
    /// on success.
    ///
    /// Mirrors DCGO `PlayCardClass(result, …, host).SetAppFusion(…)` (see
    /// `BT23_079.cs`) and reuses the linked-card-consume step from
    /// `commit_digivolve_from_hand_no_replace`'s `is_app_fusion` branch.
    pub fn commit_effect_app_fuse(
        &mut self,
        player_id: PlayerId,
        host: PermanentHandle,
        result_card: crate::card_source::CardHandle,
        zone: crate::game_actions::AppFuseSourceZone,
    ) -> bool {
        let field_index = host.index as usize;
        if host.player != player_id {
            return false;
        }
        if self.player(player_id).battle_area.get(field_index).is_none() {
            self.logger
                .log("[Rejected] app_fuse: host index out of range");
            return false;
        }
        if self.modifiers.has(host, ModifierType::CannotDigivolve) {
            return false;
        }

        // Resolve the result card to a zone-tagged CardSourceRef.
        let source_ref = match zone {
            crate::game_actions::AppFuseSourceZone::Hand => {
                let Some(i) = self
                    .player(player_id)
                    .hand
                    .iter()
                    .position(|c| c.handle() == result_card)
                else {
                    self.logger
                        .log("[Rejected] app_fuse: result card not in hand");
                    return false;
                };
                crate::enums::CardSourceRef::Hand(player_id, i)
            }
            crate::game_actions::AppFuseSourceZone::Trash => {
                let Some(i) = self
                    .player(player_id)
                    .trash
                    .iter()
                    .position(|c| c.handle() == result_card)
                else {
                    self.logger
                        .log("[Rejected] app_fuse: result card not in trash");
                    return false;
                };
                crate::enums::CardSourceRef::Trash(player_id, i)
            }
        };

        let from_stack_top = self.player(host.player).battle_area[field_index]
            .top_card()
            .card_id(&self.card_data)
            .to_string();

        // App Fusion is Cost 0 — no memory payment, no evo-requirement check.
        let Some(taken) = self.take_card_source_ref(source_ref) else {
            self.logger
                .log("[Rejected] app_fuse: result card ref changed before take");
            return false;
        };
        let top_card_id = taken.card.card_id(&self.card_data).to_string();

        let turn = self.turn_count;
        self.player_mut(host.player).battle_area[field_index].digivolve(taken.card, turn);

        // Consume the host's linked cards under the new top as digivolution
        // sources (drained in reverse so the first linked card lands deepest),
        // identical to `commit_digivolve_from_hand_no_replace`'s app-fusion step.
        let linked: Vec<crate::card_source::CardSource> =
            std::mem::take(&mut self.player_mut(host.player).battle_area[field_index].linked_cards);
        for card in linked.into_iter().rev() {
            self.player_mut(host.player).battle_area[field_index].push_under(card);
        }

        let event_card = self
            .player(host.player)
            .battle_area
            .get(field_index)
            .map(|perm| perm.top_card().handle())
            .expect("app-fuse host remains in battle area after stack mutation");

        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Digivolve {
            seq,
            player: player_id,
            top_card_id,
            field_index: field_index as u8,
            from_stack_top,
            was_dna: false,
            was_blast_dna: false,
            memory_paid: 0,
        });

        self.enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(host));
        self.drain_effect_queue();

        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: player_id,
                permanent: host,
                card: event_card,
                effect_initiated: true,
                dna_origin: false,
            },
        );
        self.drain_effect_queue();

        true
    }

    /// Install a `SelectMaterial` pending selection for DNA digivolve.
    /// Drives a two-stage resolution: stage 1 picks the first material;
    /// stage 2 (installed by the stage-1 callback) picks the second
    /// material. Stage 2 resolves into `Game::dna_digivolve_inner`,
    /// computes the matching `DnaCost` via `get_dna_stacking_order`,
    /// applies `BeforePayCost` reductions, and pays memory.
    pub fn initiate_dna_digivolve(&mut self, player_id: PlayerId, hand_index: usize) -> bool {
        let Some(route_window) = self.current_dna_route_window() else {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: not in DNA action phase (phase={:?})",
                self.current_phase
            ));
            return false;
        };
        let player = self.player(player_id);
        if hand_index >= player.hand.len() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: hand index {} out of range (hand size={})",
                hand_index,
                player.hand.len()
            ));
            return false;
        }
        let card = player.hand[hand_index].clone();

        // Collect valid first-material battle_area indices: those that
        // appear in at least one valid pair (either ordering).
        let mut first_targets: Vec<u16> = self
            .valid_dna_first_targets_for_hand_card(player_id, hand_index, route_window)
            .collect();
        first_targets.sort();
        first_targets.dedup();
        if first_targets.is_empty() {
            self.logger.log(&format!(
                "[Rejected] initiate_dna_digivolve: no valid DNA material pair for {}",
                card.card_id(&self.card_data)
            ));
            return false;
        }

        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;

        let selecting_player = player_id;
        let source_card = card.handle();

        self.pending_selection = Some(PendingSelection {
            zone_owner: None,
            kind: SelectionKind::Material,
            selecting_player,
            previous_phase,
            valid_action_ids: first_targets,
            is_optional: false,
            prompt: "Select first DNA material".to_string(),
            effect_choices: None,
            source_card,
            source_permanent: None,
            source_kind: EffectSourceKind::Digimon,
            callback: Box::new(move |game: &mut Game, action_id: u16| {
                // Stage 1 resolution: action_id is the chosen first-material
                // battle_area index for `selecting_player`.
                let first_idx = action_id as usize;
                let first_player = selecting_player;
                let evo_hand_index = hand_index;

                // Defensive: validate the first index against the
                // controller's current battle_area (it could have shifted
                // since selection was installed if a triggered effect
                // removed a permanent during the install drain).
                if first_idx >= game.player(first_player).battle_area.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: first index {} out of range (battle_area size={})",
                        first_idx,
                        game.player(first_player).battle_area.len()
                    ));
                    return;
                }
                if evo_hand_index >= game.player(first_player).hand.len() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: evo hand index {} out of range (hand size={})",
                        evo_hand_index,
                        game.player(first_player).hand.len()
                    ));
                    return;
                }

                // Build valid second-material list for the chosen first.
                let second_targets: Vec<u16> = game
                    .valid_dna_second_targets_for_hand_card(
                        first_player,
                        evo_hand_index,
                        first_idx,
                        route_window,
                    )
                    .collect();
                if second_targets.is_empty() {
                    game.logger.log(&format!(
                        "[Rejected] dna_digivolve stage 1: no valid second-material targets for first index {}",
                        first_idx
                    ));
                    return;
                }

                // Install stage-2 selection. previous_phase was Main when
                // stage-1 was installed; we preserve it through the chain
                // so the final resolution returns to Main.
                game.pending_selection = Some(PendingSelection {
                    zone_owner: None,
                    kind: SelectionKind::Material,
                    selecting_player: first_player,
                    previous_phase,
                    valid_action_ids: second_targets,
                    is_optional: false,
                    prompt: "Select second DNA material".to_string(),
                    effect_choices: None,
                    source_card,
                    source_permanent: None,
                    source_kind: EffectSourceKind::Digimon,
                    callback: Box::new(move |game: &mut Game, action_id: u16| {
                        let second_idx = action_id as usize;
                        game.resolve_dna_digivolve_stage2_with_window(
                            first_player,
                            first_idx,
                            second_idx,
                            evo_hand_index,
                            route_window,
                        );
                    }),
                    on_decline: None,
                });
                // resolve_generic_selection restored current_phase to
                // previous_phase (Main) before invoking this callback. We
                // re-flip it back to SelectMaterial so the action mask
                // reflects the live stage-2 prompt.
                game.current_phase = GamePhase::SelectMaterial;
            }),
            on_decline: None,
        });
        true
    }

    /// Stage-2 resolution of `Game::initiate_dna_digivolve`'s two-stage
    /// selection chain. Receives the chosen second-material `battle_area`
    /// index and the captured stage-1 state. Re-resolves the matching
    /// DNA route orientation, applies `BeforePayCost` reductions, calls
    /// `Game::dna_digivolve_inner`, and triggers the auto-end-of-turn
    /// check (mirroring `digivolve_from_hand`).
    ///
    /// Defensively re-validates indices because triggered effects fired
    /// during stage-1 install can mutate the battle area between selection
    /// install and resolution.
    ///
    /// Failure paths log `[Rejected] ...` via `self.logger` and return
    /// without mutating game state. The `pending_selection` was already
    /// consumed by `resolve_generic_selection` before this method ran.
    pub fn resolve_dna_digivolve_stage2_with_window(
        &mut self,
        first_player: PlayerId,
        first_idx: usize,
        second_idx: usize,
        evo_hand_index: usize,
        route_window: crate::dna_digivolve::DnaRouteWindow,
    ) {
        if second_idx >= self.player(first_player).battle_area.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: second index {} out of range (battle_area size={})",
                second_idx,
                self.player(first_player).battle_area.len()
            ));
            return;
        }
        if first_idx == second_idx {
            self.logger
                .log("[Rejected] resolve_dna_digivolve_stage2: first and second indices are equal");
            return;
        }
        if evo_hand_index >= self.player(first_player).hand.len() {
            self.logger.log(&format!(
                "[Rejected] resolve_dna_digivolve_stage2: evo hand index {} out of range (hand size={})",
                evo_hand_index,
                self.player(first_player).hand.len()
            ));
            return;
        }

        let Some(route_match) = self.dna_route_for_hand_card(
            first_player,
            evo_hand_index,
            first_idx,
            second_idx,
            route_window,
        ) else {
            self.logger.log(
                "[Rejected] resolve_dna_digivolve_stage2: no matching DNA route for chosen pair",
            );
            return;
        };
        let printed_cost = route_match.memory_cost;

        let (target_a, target_b) = if route_match.first_is_top {
            (
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
            )
        } else {
            (
                PermanentHandle {
                    player: first_player,
                    index: second_idx as u8,
                },
                PermanentHandle {
                    player: first_player,
                    index: first_idx as u8,
                },
            )
        };

        // Pass the DNA-result hand card as the cost-target so target-aware
        // predicates (`cost_target: { color_is: green }`) can fire for the
        // DNA digivolve path — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. Both
        // DNA materials get threaded as target permanents so per-material
        // self-scoped observers ("when THIS Digimon would DNA digivolve")
        // can fire on either side.
        let evo_card_target = self.player(first_player).hand[evo_hand_index].handle();
        let target = CostTargetContext {
            card: evo_card_target,
            from_hand: true,
            is_digivolve: true,
            target_permanents: [Some(target_a), Some(target_b)],
        };
        // Set the DNA-origin context so cost-calc-time predicates like
        // `dna_origin: true` evaluate true for both the reducer scan and
        // the observer scan. Restored after both scans complete so the
        // marker doesn't leak into downstream effect-queue drains (those
        // re-set it themselves per-effect).
        let prev_dna_origin = self.current_dna_origin;
        self.current_dna_origin = Some(true);
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            first_player,
            CostReductionKind::Digivolve,
            Some(target),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(first_player, Some(target));
        self.current_dna_origin = prev_dna_origin;
        let effective_cost = (printed_cost as i32 - total_reduction).max(0) as u16;

        let _ = self.dna_digivolve_inner(
            target_a,
            target_b,
            first_player,
            evo_hand_index,
            effective_cost,
            true,
            false,
        );

        if route_window == crate::dna_digivolve::DnaRouteWindow::EndOfTurnAction {
            if self.memory < 0 && !self.game_over {
                self.pass_end_of_turn_action();
            }
        } else {
            self.check_turn_end();
        }
    }

    /// Script-initiated digivolve: place the card at `hand_index` from
    /// `player_id`'s hand onto `target`, bypassing the phase check and
    /// optionally the color check. Memory is paid according to `cost_delta`.
    ///
    /// Unlike `digivolve_from_hand`, this does **not** check `GamePhase::Main`
    /// or fire `check_turn_end` — it is designed for use inside effect
    /// callbacks where those invariants don't apply. It also does **not**
    /// draw a card (that's a player-action benefit, not an effect mechanic).
    ///
    /// Returns `true` on success, `false` if validation fails (bad index,
    /// no matching evo cost, or insufficient memory).
    pub fn effect_initiated_digivolve(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            ignore_color,
            false,
            source,
        )
    }

    pub fn effect_initiated_digivolve_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        hand_index: usize,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            crate::enums::CardSourceRef::Hand(player_id, hand_index),
            target,
            cost_delta,
            true,
            true,
            source,
        )
    }

    /// Source-general script-initiated digivolve. The result card is taken
    /// from any `CardSourceRef`, placed on top of `target`, and restored to
    /// its source zone if a post-take failure occurs.
    pub fn effect_initiated_digivolve_from_source(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id,
            source_ref,
            target,
            cost_delta,
            ignore_color,
            false,
            source,
        )
    }

    pub fn effect_initiated_digivolve_from_source_ignore_requirements(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        source: PlaySource,
    ) -> bool {
        self.effect_initiated_digivolve_from_source_inner(
            player_id, source_ref, target, cost_delta, true, true, source,
        )
    }

    pub(crate) fn effect_initiated_digivolve_from_source_inner(
        &mut self,
        player_id: PlayerId,
        source_ref: crate::enums::CardSourceRef,
        target: PermanentHandle,
        cost_delta: crate::enums::CostDelta,
        ignore_color: bool,
        ignore_requirements: bool,
        source: PlaySource,
    ) -> bool {
        if source == PlaySource::ByEffect
            && self
                .modifiers
                .player_has(player_id, ModifierType::CannotDigivolveDigimonByEffect)
        {
            self.logger.log(
                "[Rejected] effect_initiated_digivolve: blocked by CannotDigivolveDigimonByEffect",
            );
            return false;
        }

        let Some((evo_card_handle, evo_card_data_index, _)) =
            self.card_source_ref_snapshot(source_ref)
        else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref out of range");
            return false;
        };

        {
            let target_player = self.player(target.player);
            if (target.index as usize) >= target_player.battle_area.len() {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: target index {} out of range (battle_area size={})",
                    target.index,
                    target_player.battle_area.len()
                ));
                return false;
            }
        }

        // 2. Find a matching evo cost.
        let (base_level, base_colors) = {
            let target_player = self.player(target.player);
            let perm = &target_player.battle_area[target.index as usize];
            let identity = perm.synth_identity(&self.card_data, &self.modifiers, target);
            let Some(base_level) = identity.level else {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: target top card has no level");
                return false;
            };
            (base_level, identity.colors)
        };

        let evo_costs = &self.card_data[evo_card_data_index].evo_costs;
        let matching_memory_cost = if ignore_requirements {
            Some(0)
        } else {
            evo_costs
                .iter()
                .find(|ec| {
                    ec.level == base_level
                        && (ignore_color
                            || crate::action::mask::evo_color(ec.card_color)
                                .map(|c| base_colors.contains(&c))
                                .unwrap_or(false))
                })
                .map(|ec| ec.memory_cost)
        };
        let Some(matching_memory_cost) = matching_memory_cost else {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: no matching evo cost (base_level={}, ignore_color={}, ignore_requirements={})",
                base_level, ignore_color, ignore_requirements
            ));
            return false;
        };
        let base_cost = cost_delta.resolve(matching_memory_cost);
        // Pass the evolving card as the cost-target so target-aware
        // predicates can fire — G-BEFORE-PAY-COST-DIGIVOLVE-TARGET. The
        // `target` permanent here is the one being digivolved into
        // (effect-initiated digivolves stack a hand/trash/security card
        // onto an existing battle-area permanent).
        let from_hand = matches!(source_ref, crate::enums::CardSourceRef::Hand(_, _));
        let cost_target_ctx = CostTargetContext {
            card: evo_card_handle,
            from_hand,
            is_digivolve: true,
            target_permanents: [Some(target), None],
        };
        let total_reduction = self.scan_before_pay_cost_reduction_with_target(
            player_id,
            CostReductionKind::Digivolve,
            Some(cost_target_ctx),
        );
        // Observer dispatch — G-BEFORE-PAY-COST-GAIN-MEMORY.
        self.scan_before_pay_cost_observers(player_id, Some(cost_target_ctx));
        let effective_cost = (base_cost as i32 - total_reduction).max(0) as u16;

        // 3. Remove the card from its source and pay memory. If payment fails,
        // restore the source exactly where it came from.
        if let crate::enums::CardSourceRef::Security(defender, index) = source_ref {
            if !self.pay_memory(effective_cost) {
                self.logger.log(&format!(
                    "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                    effective_cost, self.memory
                ));
                return false;
            }

            let player = self.player_mut(defender);
            if index >= player.security.len() {
                self.logger
                    .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
                return false;
            }
            let card = player.security.remove(index);
            player.face_up_security.remove(&card.card_index);
            let cause = crate::trigger_context::EventCause::from(self.infer_effect_cause(defender));
            self.fire_effect_security_removal(
                defender,
                player_id,
                player_id,
                cause,
                card,
                crate::selection::SecurityRemovalDestination::Digivolve {
                    player: player_id,
                    target,
                    turn: self.turn_count,
                },
            );
            return true;
        }

        let Some(taken) = self.take_card_source_ref(source_ref) else {
            self.logger
                .log("[Rejected] effect_initiated_digivolve: source ref changed before take");
            return false;
        };
        if !self.pay_memory(effective_cost) {
            self.logger.log(&format!(
                "[Rejected] effect_initiated_digivolve: cannot pay memory cost {} (current memory={})",
                effective_cost, self.memory
            ));
            let _ = self.restore_card_source_ref(source_ref, taken);
            return false;
        }

        // 4. Move the card onto the target permanent's stack.
        let turn = self.turn_count;
        self.player_mut(target.player).battle_area[target.index as usize]
            .digivolve(taken.card, turn);

        // 4a. Soft-remove an emptied Material source. If `source_ref` was
        // `Material(src, _)`, `take_card_source_ref` may have removed the
        // source permanent's only card. Left in `battle_area`, that "zombie"
        // permanent panics any trigger fan-out that iterates all permanents
        // and calls `top_card()`. NOT a deletion: no OnDeletion fires, no
        // replacement window, no trash for the body card (already moved to
        // target). Matches DCGO's caller-side `RemoveField(permanent)`
        // pattern (e.g. Jogress at
        // `DCGO/Assets/Scripts/Script/CardController.cs:1509`). See
        // `Game::soft_remove_if_emptied` doc + the `G-PERMANENT-EMPTY-…`
        // entry in `qa/archetype-qa/engine-gaps.md`.
        let mut target = target;
        if let crate::enums::CardSourceRef::Material(src_handle, _) = source_ref {
            if self.soft_remove_if_emptied(src_handle) {
                target = Self::shift_handle_after_soft_remove(src_handle, target);
            }
        }

        let event_card = self
            .player(target.player)
            .battle_area
            .get(target.index as usize)
            .map(|perm| perm.top_card().handle())
            .expect("effect digivolve target remains in battle area after stack mutation");

        // 5. Fire WhenDigivolving triggers.
        self.enqueue_triggered(
            EffectTiming::WhenDigivolving,
            TriggerSource::Permanent(target),
        );
        self.drain_effect_queue();

        // OnDigivolve: global observer — carries the evolved permanent/card
        // plus effect-origin provenance for "digivolved by an effect" gates.
        self.enqueue_triggered(
            EffectTiming::OnDigivolve,
            TriggerSource::Digivolved {
                player: player_id,
                permanent: target,
                card: event_card,
                effect_initiated: true,
                dna_origin: false,
            },
        );
        self.drain_effect_queue();

        true
    }
}
