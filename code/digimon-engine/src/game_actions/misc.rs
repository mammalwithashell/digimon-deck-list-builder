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
        accumulated_reduction: i32,
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

        let valid_action_ids = candidates
            .iter()
            .map(|(action_id, _)| *action_id)
            .collect::<Vec<_>>();
        let previous_phase = self.current_phase;
        self.current_phase = GamePhase::SelectMaterial;
        self.pending_selection = Some(PendingSelection {
            kind: SelectionKind::Material,
            selecting_player: player_id,
            previous_phase,
            valid_action_ids,
            is_optional: true,
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
        true
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
        let perm = crate::permanent::Permanent::new(card, turn);
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
        let top_card = self.players[player_id as usize].battle_area[entered_index].top_card();
        let emitted_card_id = top_card.card_id(&self.card_data).to_string();
        let cost_printed = self.card_data[top_card.data_index].play_cost as i16;
        let seq = self.next_event_seq();
        self.events.push(crate::events::GameEvent::Play {
            seq,
            player: player_id,
            card_id: emitted_card_id,
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
            self.pending_option_placed_link_resume = Some(host);
            return;
        }

        self.fire_on_link_after_option_placed();
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
