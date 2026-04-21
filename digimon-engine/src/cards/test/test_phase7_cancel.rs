//! TEST-P7-CANCEL: Mandatory WhenWouldBeDeleted replacement that cancels
//! deletion. Used by Phase 7 Task 2 dispatcher tests.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestP7Cancel;

impl CardEffect for TestP7Cancel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Cancel deletion (mandatory)")
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}
