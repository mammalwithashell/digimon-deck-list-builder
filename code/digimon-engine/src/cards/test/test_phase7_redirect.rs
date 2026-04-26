//! TEST-P7-REDIRECT: Mandatory WhenWouldBeDeleted replacement that
//! redirects the target zone to Deck. Used by Phase 7 Task 2 dispatcher tests.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::Zone;

pub struct TestP7Redirect;

impl CardEffect for TestP7Redirect {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Redirect deletion to Deck")
            .replacement_process(|rctx| {
                rctx.redirect_to(Zone::Deck);
            })
            .build()]
    }
}
