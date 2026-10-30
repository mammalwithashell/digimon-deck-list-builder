//! Owned physical security-to-trash movement across replacement selections.
use crate::{
    card_source::CardHandle,
    enums::{EffectTiming, PlayerId, Zone},
    game::Game,
    replacement::{ReplacementCause, ReplacementOutcome, ReplacementSubject},
    selection::SecurityRemovalDestination,
};

#[derive(Debug, Clone)]
pub(crate) struct SecurityTrashMove {
    pub player: PlayerId,
    pub card: CardHandle,
    pub source_player: PlayerId,
    pub cause: ReplacementCause,
    pub recorded: bool,
}

impl Game {
    pub(crate) fn start_security_trash(
        &mut self,
        player: PlayerId,
        card: CardHandle,
        source_player: PlayerId,
        recorded: bool,
    ) -> bool {
        let state = SecurityTrashMove {
            player,
            card,
            source_player,
            cause: self.infer_effect_cause(player),
            recorded,
        };
        self.run_security_trash(state)
    }

    fn run_security_trash(&mut self, state: SecurityTrashMove) -> bool {
        if !self
            .player(state.player)
            .security
            .iter()
            .any(|c| c.handle() == state.card)
        {
            return false;
        }
        self.pending_security_trashes.push(state.clone());
        let outcome = self.try_replace(
            EffectTiming::WhenWouldBeTrashed,
            ReplacementSubject::Card(state.card, Zone::Security),
            state.cause,
            Some(Zone::Trash),
        );
        if self.pending_selection.is_some() {
            return false;
        }
        let index = self
            .pending_security_trashes
            .iter()
            .rposition(|s| s.card == state.card)
            .unwrap();
        let state = self.pending_security_trashes.remove(index);
        self.commit_security_trash(state, outcome)
    }

    pub(crate) fn resume_security_trash(
        &mut self,
        subject: ReplacementSubject,
        outcome: ReplacementOutcome,
    ) -> bool {
        let ReplacementSubject::Card(card, Zone::Security) = subject else {
            return false;
        };
        let Some(index) = self
            .pending_security_trashes
            .iter()
            .rposition(|s| s.card == card)
        else {
            return false;
        };
        let state = self.pending_security_trashes.remove(index);
        if outcome == ReplacementOutcome::None {
            // Re-enter to offer remaining immediate effects, retaining this event's ledger.
            self.run_security_trash(state);
        } else {
            self.commit_security_trash(state, outcome);
        }
        true
    }

    fn commit_security_trash(
        &mut self,
        state: SecurityTrashMove,
        outcome: ReplacementOutcome,
    ) -> bool {
        let destination = match outcome {
            ReplacementOutcome::None | ReplacementOutcome::Redirected(Zone::Trash) => SecurityRemovalDestination::Trash,
            ReplacementOutcome::Redirected(Zone::Hand) => SecurityRemovalDestination::Hand(state.player),
            ReplacementOutcome::Redirected(Zone::Deck) => SecurityRemovalDestination::Deck { owner: state.player, to_bottom: true },
            ReplacementOutcome::Cancelled | ReplacementOutcome::CustomHandled => return false,
            unsupported => panic!("Unsupported security-trash replacement outcome: {unsupported:?}; the owning movement must implement its destination"),
        };
        let Some(index) = self
            .player(state.player)
            .security
            .iter()
            .position(|c| c.handle() == state.card)
        else {
            return false;
        };
        let card = self.player_mut(state.player).security.remove(index);
        self.player_mut(state.player)
            .face_up_security
            .remove(&card.card_index);
        let paid = matches!(destination, SecurityRemovalDestination::Trash);
        if state.recorded && paid {
            if let Some((_, result)) = self
                .security_trash_receipts
                .iter_mut()
                .rev()
                .find(|(c, _)| *c == state.card.0)
            {
                *result = true;
            }
        }
        self.fire_effect_security_removal(
            state.player,
            self.next_clockwise(state.player),
            state.source_player,
            crate::trigger_context::EventCause::from(state.cause),
            card,
            destination,
        );
        self.mark_until_condition_dirty();
        self.reevaluate_until_condition_modifiers_if_dirty();
        paid
    }
}
