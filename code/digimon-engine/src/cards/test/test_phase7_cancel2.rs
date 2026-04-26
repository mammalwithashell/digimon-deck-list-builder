//! TEST-P7-CANCEL2: A second mandatory WhenWouldBeDeleted replacement
//! that sets the outcome to `CustomHandled`. Paired with `TestP7Cancel` in
//! layering tests — the order in which candidates run determines the final
//! outcome (last non-None outcome wins per the v1 dispatcher semantics).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestP7Cancel2;

impl CardEffect for TestP7Cancel2 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Handled deletion (mandatory)")
            .replacement_process(|rctx| {
                rctx.handled();
            })
            .build()]
    }
}
