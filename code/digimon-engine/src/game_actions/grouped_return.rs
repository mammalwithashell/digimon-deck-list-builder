//! Commit an already processed simultaneous departure, then enqueue its events.
use crate::{
    enums::{CardKind, EffectTiming, StackPosition},
    game::Game,
    permanent::PermanentHandle,
    selection::TriggerSource,
    trigger_context::EventCause,
};
impl Game {
    pub(crate) fn commit_simultaneous_returns(
        &mut self,
        handles: Vec<PermanentHandle>,
        position: Option<StackPosition>,
        include_sources: bool,
        receipt: u64,
    ) {
        // Capture every last-known object before the first removal changes auras.
        let plans: Vec<_> = handles
            .iter()
            .filter_map(|h| {
                let p = self.player(h.player).battle_area.get(h.index as usize)?;
                Some((*h, p.battle_identity?, self.build_snapshot_for_handle(*h)?))
            })
            .collect();
        self.enter_deferred_drain();
        let mut removed = Vec::new();
        for (original, id, snapshot) in plans {
            let Some(h) = self.resolve_digivolve_host_identity(original.player, id) else {
                continue;
            };
            let p = self
                .player_mut(h.player)
                .battle_area
                .remove(h.index as usize);
            self.clear_permanent_full(h);
            self.modifiers.expire_player_on_permanent_leave(h);
            self.modifiers
                .shift_after_battle_area_remove(h.player, h.index);
            self.shift_pending_attack_after_battle_area_remove(h.player, h.index);
            self.shift_effect_queue_after_battle_area_remove(h.player, h.index);
            removed.push((original, p, snapshot));
        }
        let mut overflow = Vec::new();
        let mut source_events = Vec::new();
        let mut leave_events = Vec::new();
        let mut hand_events = Vec::new();
        let mut linked_events = 0;
        let mut completed = Vec::new();
        // Ordered input is bottommost-first/topmost-first. Insertion at either
        // end therefore consumes that order in reverse.
        if position.is_some() {
            removed.reverse();
        }
        for (original, mut p, mut snapshot) in removed {
            let Some(top) = p.card_sources.pop() else {
                continue;
            };
            let card = top.handle();
            let owner = top.owner;
            let token = top.is_token;
            overflow.extend(p.card_sources.iter().cloned());
            overflow.push(top.clone());
            snapshot.cause = if position == Some(StackPosition::Bottom) {
                EventCause::DeckBottom
            } else {
                EventCause::Return
            };
            if let Some(position) = position {
                if !token {
                    if include_sources {
                        p.card_sources.push(top);
                        self.insert_stack_into_owners_decks(
                            std::mem::take(&mut p.card_sources),
                            position,
                        );
                    } else {
                        self.insert_card_into_deck(owner, top, position);
                    }
                    completed.push((card, false));
                }
            } else if !token {
                if top.card_kind(&self.card_data) == CardKind::DigiEgg {
                    self.player_mut(owner).insert_in_digi_egg_deck(0, top);
                } else {
                    self.player_mut(owner).put_in_hand(top);
                    completed.push((card, true));
                    hand_events.push(owner);
                }
            }
            for source in p.card_sources {
                let source_card = source.handle();
                self.player_mut(source.owner).put_in_trash(source);
                source_events.push((original, card, source_card));
            }
            if !p.linked_cards.is_empty() {
                linked_events += 1;
            }
            for linked in p.linked_cards {
                self.player_mut(linked.owner).put_in_trash(linked);
            }
            leave_events.push((original, snapshot));
        }
        if let Some((_, result)) = self
            .return_receipts
            .iter_mut()
            .find(|(id, _)| *id == receipt)
        {
            result.extend(completed);
        }
        // All zone changes and departed auras precede Overflow and observers.
        self.tick_declarative_effects();
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        self.apply_ace_overflow_for_sources(&overflow);
        // Every returned stack's digivolution cards are trashed at the same
        // time: one event (§15-5-2).
        self.enqueue_digivolution_cards_trashed(
            source_events
                .into_iter()
                .map(|(h, host, card)| super::TrashedSource {
                    player: h.player,
                    host: h,
                    host_card: host,
                    card,
                })
                .collect(),
            EventCause::Rule,
        );
        for _ in 0..linked_events {
            for player in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(player as u8),
                );
            }
        }
        for (h, snapshot) in leave_events {
            self.fire_on_leave_field(h, snapshot);
        }
        for owner in hand_events {
            self.fire_on_add_to_hand_by_effect(owner);
        }
        // The originating effect still owns its result finalizer and outer tail.
        // Its dispatcher performs the drain when that effect completes.
        self.draining_deferred = self.draining_deferred.saturating_sub(1);
    }
}
