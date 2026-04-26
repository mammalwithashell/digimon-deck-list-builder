//! TEST-P7-OPTIONAL: Optional WhenWouldBeDeleted replacement that cancels
//! on accept. Used by Phase 7 Task 2 dispatcher tests for the pending-
//! selection accept/decline paths.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestP7Optional;

impl CardEffect for TestP7Optional {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Optional cancel deletion")
            .optional()
            .replacement_process(|rctx| {
                rctx.cancel();
            })
            .build()]
    }
}
