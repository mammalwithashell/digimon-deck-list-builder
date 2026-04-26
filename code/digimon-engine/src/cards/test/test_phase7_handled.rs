//! TEST-P7-HANDLED: Mandatory WhenWouldBeDeleted replacement that trashes
//! the top of the owner's deck and then marks the outcome `CustomHandled`.
//! Used by Phase 7 Task 2 dispatcher tests to verify that side effects
//! *do* apply and the outcome still propagates correctly.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestP7Handled;

impl CardEffect for TestP7Handled {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Trash top of deck then handle")
            .replacement_process(|rctx| {
                // Trash the top card of the subject-controller's deck
                // directly via the game state, then mark handled.
                let owner = rctx.effect.player;
                let game = &mut *rctx.effect.game;
                if let Some(card) = game.players[owner as usize].deck.pop() {
                    game.players[owner as usize].trash.push(card);
                }
                rctx.handled();
            })
            .build()]
    }
}
