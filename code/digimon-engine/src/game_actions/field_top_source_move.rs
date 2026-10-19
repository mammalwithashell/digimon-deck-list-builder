//! A field top becomes a source: rule 4-7-7 removes its permanent, not just a card.
use super::*;
use crate::game::PendingFieldTopSourceMove;
use crate::replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject};
use crate::trigger_context::{DeletedObjectSnapshot, EffectAttribution, EventCause};

impl Game {
    pub(crate) fn source_move_anchor(&mut self, h: PermanentHandle) -> Option<(PlayerId, u64)> {
        self.capture_source_play_host_identity(h)
            .map(|id| (h.player, id))
    }
    pub(crate) fn resolve_source_move_anchor(
        &self,
        a: (PlayerId, u64),
        breeding: bool,
    ) -> Option<PermanentHandle> {
        self.resolve_source_play_host_identity(
            PermanentHandle {
                player: a.0,
                index: if breeding {
                    crate::action::space::BREEDING_TARGET as u8
                } else {
                    0
                },
            },
            a.1,
        )
    }
    pub(crate) fn begin_field_top_source_move(
        &mut self,
        donor: PermanentHandle,
        target: PermanentHandle,
        observer: PlayerId,
        face_down: bool,
        top: bool,
        effect_source_card: Option<CardHandle>,
        source_kind: EffectSourceKind,
    ) -> bool {
        self.begin_field_top_source_move_with_receipt(
            donor,
            target,
            observer,
            face_down,
            top,
            effect_source_card,
            source_kind,
            None,
        )
    }
    pub(crate) fn begin_field_top_source_move_with_receipt(
        &mut self,
        donor: PermanentHandle,
        target: PermanentHandle,
        observer: PlayerId,
        face_down: bool,
        top: bool,
        effect_source_card: Option<CardHandle>,
        source_kind: EffectSourceKind,
        result_receipt: Option<u64>,
    ) -> bool {
        if self.is_token_source_destination(target) { return false; }
        if donor == target {
            return false;
        }
        let Some(donor_anchor) = self.source_move_anchor(donor) else {
            return false;
        };
        let Some(target_anchor) = self.source_move_anchor(target) else {
            return false;
        };
        let card = self.player(donor.player).battle_area[donor.index as usize]
            .top_card()
            .handle();
        let cause = if observer == donor.player {
            ReplacementCause::OwnEffect
        } else {
            ReplacementCause::OpponentEffect
        };
        self.pending_field_top_source_moves
            .push(PendingFieldTopSourceMove {
                result_receipt,
                donor: donor_anchor,
                target: target_anchor,
                target_breeding: target.index == crate::action::space::BREEDING_TARGET as u8,
                donor_card: card,
                observer,
                face_down,
                top,
                effect_source_card,
                source_kind,
                cause,
            });
        self.run_field_top_source_move(donor_anchor)
    }
    fn run_field_top_source_move(&mut self, key: (PlayerId, u64)) -> bool {
        let Some(pos) = self
            .pending_field_top_source_moves
            .iter()
            .rposition(|m| m.donor == key)
        else {
            return false;
        };
        let state = self.pending_field_top_source_moves[pos].clone();
        let Some(donor) = self.resolve_source_move_anchor(state.donor, false) else {
            self.pending_field_top_source_moves.remove(pos);
            return false;
        };
        if self
            .resolve_source_move_anchor(state.target, state.target_breeding)
            .is_none()
        {
            self.pending_field_top_source_moves.remove(pos);
            return false;
        }
        if self.progress_excludes(donor, Some(state.observer))
            || self.permanent_is_unaffected_by_effect(donor, state.observer, state.source_kind)
        {
            self.pending_field_top_source_moves.remove(pos);
            return false;
        }
        let outcome = self.try_replace(
            EffectTiming::WhenWouldLeaveBattleArea,
            ReplacementSubject::Permanent(donor),
            state.cause,
            None,
        );
        if self.pending_selection.is_some() {
            return false;
        }
        self.finish_field_top_source_move(key, outcome)
    }
    pub(crate) fn resume_field_top_source_move(
        &mut self,
        subject: ReplacementSubject,
        cause: ReplacementCause,
        outcome: ReplacementOutcome,
    ) -> bool {
        let Some(h) = subject.permanent() else {
            return false;
        };
        let Some(id) = self
            .player(h.player)
            .battle_area
            .get(h.index as usize)
            .and_then(|p| p.battle_identity)
        else {
            return false;
        };
        let key = (h.player, id);
        if !self
            .pending_field_top_source_moves
            .iter()
            .any(|m| m.donor == key && m.cause == cause)
        {
            return false;
        }
        if outcome == ReplacementOutcome::None {
            self.run_field_top_source_move(key);
        } else {
            self.finish_field_top_source_move(key, outcome);
        }
        true
    }
    fn finish_field_top_source_move(
        &mut self,
        key: (PlayerId, u64),
        outcome: ReplacementOutcome,
    ) -> bool {
        let Some(pos) = self
            .pending_field_top_source_moves
            .iter()
            .rposition(|m| m.donor == key)
        else {
            return false;
        };
        let state = self.pending_field_top_source_moves.remove(pos);
        let Some(donor) = self.resolve_source_move_anchor(state.donor, false) else {
            return false;
        };
        match outcome {
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => return false,
            ReplacementOutcome::Redirected(Zone::Hand) => {
                self.return_to_hand(donor);
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Deck) => {
                self.return_to_deck(donor, crate::enums::StackPosition::Bottom);
                return false;
            }
            ReplacementOutcome::Redirected(Zone::Trash) => {
                self.delete_permanent_with_cause(donor, state.cause);
                return false;
            }
            ReplacementOutcome::Substituted(ReplacementSubject::Permanent(other)) => {
                let Some(target) =
                    self.resolve_source_move_anchor(state.target, state.target_breeding)
                else {
                    return false;
                };
                return self.begin_field_top_source_move_with_receipt(
                    other,
                    target,
                    state.observer,
                    state.face_down,
                    state.top,
                    state.effect_source_card,
                    state.source_kind,
                    state.result_receipt,
                );
            }
            ReplacementOutcome::None => {}
            other => {
                debug_assert!(false, "unsupported field-to-source replacement {other:?}");
                return false;
            }
        }
        let Some(target) = self.resolve_source_move_anchor(state.target, state.target_breeding)
        else {
            return false;
        };
        if self.is_token_source_destination(target) { return false; }
        if donor == target {
            return false;
        }
        let p = &self.player(donor.player).battle_area[donor.index as usize];
        // A replacement may evolve this same Digimon; its current top is moved.
        let top_card = p.top_card().handle();
        let Some(data) = self.card_data_for_handle(top_card) else {
            return false;
        };
        let source_handles = p
            .card_sources
            .iter()
            .take(p.card_sources.len() - 1)
            .map(|s| s.handle())
            .collect::<Vec<_>>();
        let dp = self.effective_dp(donor);
        let identity = p.synth_identity(&self.card_data, &self.modifiers, donor);
        let snapshot = DeletedObjectSnapshot {
            former_controller: donor.player,
            top_card,
            card_kind: identity.kind,
            retained_kind: identity.retained_kind,
            traits: identity.traits.clone(),
            level: identity.level,
            dp,
            cause: EventCause::from(state.cause),
            dp_just_before: dp,
            level_just_before: identity.level,
            cost_just_before: Some(data.play_cost),
            names_just_before: identity.card_names.clone(),
            traits_just_before: identity.traits.clone(),
            source_count_just_before: source_handles.len(),
            digisources_just_before: source_handles.clone(),
            is_token: p.top_card().is_token,
        };
        let mut donor_perm = self
            .player_mut(donor.player)
            .battle_area
            .remove(donor.index as usize);
        self.clear_permanent_full(donor);
        self.modifiers.expire_player_on_permanent_leave(donor);
        self.modifiers
            .shift_after_battle_area_remove(donor.player, donor.index);
        self.shift_pending_attack_after_battle_area_remove(donor.player, donor.index);
        self.shift_effect_queue_after_battle_area_remove(donor.player, donor.index);
        let mut card = donor_perm
            .card_sources
            .pop()
            .expect("validated nonempty donor");
        card.face_down = state.face_down;
        let target = self
            .resolve_source_move_anchor(state.target, state.target_breeding)
            .expect("distinct receiving host survives compaction");
        if !card.is_token {
            let host = if state.target_breeding {
                self.player_mut(target.player)
                    .breeding_area
                    .as_mut()
                    .unwrap()
            } else {
                &mut self.player_mut(target.player).battle_area[target.index as usize]
            };
            if state.top {
                host.push_as_top_source(card);
            } else {
                host.push_under(card);
            }
            if let Some(id) = state.result_receipt {
                if let Some((_, result)) = self
                    .field_source_receipts
                    .iter_mut()
                    .find(|(receipt, _)| *receipt == id)
                {
                    *result = Some(top_card);
                }
            }
            self.note_effect_added_sources(
                target,
                vec![top_card],
                EffectAttribution {
                    controller: state.observer,
                    source_card: state.effect_source_card,
                    source_permanent: None,
                },
            );
        }
        // All disposition completes before any observer can drain.
        let overflow = donor_perm
            .card_sources
            .iter()
            .chain(donor_perm.linked_cards.iter())
            .cloned()
            .collect::<Vec<_>>();
        for card in donor_perm.card_sources {
            self.player_mut(card.owner).put_in_trash(card);
        }
        let linked = !donor_perm.linked_cards.is_empty();
        for card in donor_perm.linked_cards {
            self.player_mut(card.owner).put_in_trash(card);
        }
        self.apply_ace_overflow_for_sources(&overflow);
        self.tick_declarative_effects();
        // The donor's digivolution cards are trashed at the same time: one
        // event (§15-5-2).
        self.enqueue_digivolution_cards_trashed(
            source_handles
                .into_iter()
                .map(|card| super::TrashedSource {
                    player: donor.player,
                    host: donor,
                    host_card: top_card,
                    card,
                })
                .collect(),
            EventCause::Rule,
        );
        if linked {
            for player in 0..self.players.len() {
                self.enqueue_triggered(
                    EffectTiming::OnLinkedCardTrashed,
                    TriggerSource::PlayerBattleArea(player as PlayerId),
                );
            }
        }
        self.fire_on_leave_field(donor, snapshot);
        true
    }
}

#[cfg(test)]
mod terminal_move_tests {
    use crate::card_source::CardHandle;
    use crate::debug_runner::{make_test_card, DebugRunner};
    use crate::effect::{CardEffect, Effect};
    use crate::enums::CardSourceRef;
    use crate::permanent::PermanentHandle;
    use crate::TerminalOutcomeReason;
    struct Protect;
    impl CardEffect for Protect {
        fn effects(&self, c: CardHandle) -> Vec<Effect> {
            vec![Effect::when_would_leave_battle_area(c)
                .optional()
                .replacement_process(|ctx| ctx.cancel())
                .build()]
        }
    }
    fn parked() -> DebugRunner {
        let mut r = DebugRunner::builder()
            .add_card(make_test_card("HOST", "Host"))
            .add_card(make_test_card("DONOR", "Donor"))
            .deck(0, &["HOST"; 8])
            .deck(1, &["HOST"; 8])
            .start();
        r.register_effect("DONOR", std::sync::Arc::new(Protect));
        r.skip_mulligan();
        r.place_on_field(0, "HOST", Some(0));
        r.place_on_field(0, "DONOR", Some(0));
        r.game.place_as_bottom_source(
            CardSourceRef::Material(
                PermanentHandle {
                    player: 0,
                    index: 1,
                },
                0,
            ),
            PermanentHandle {
                player: 0,
                index: 0,
            },
            false,
        );
        assert!(r.pending_kind().is_some());
        assert_eq!(r.game.pending_field_top_source_moves.len(), 1);
        r
    }
    #[test]
    fn declared_winner_clears_parked_move() {
        let mut r = parked();
        r.game
            .declare_winner_with_reason(0, TerminalOutcomeReason::EngineDeclared);
        assert!(r.game.pending_field_top_source_moves.is_empty());
    }
    #[test]
    fn elimination_clears_parked_move() {
        let mut r = parked();
        r.game.eliminate_player(1);
        assert!(r.game.pending_field_top_source_moves.is_empty());
    }
    #[test]
    fn deckout_clears_parked_move() {
        let mut r = parked();
        r.game.handle_deckout(1);
        assert!(r.game.pending_field_top_source_moves.is_empty());
    }
}
