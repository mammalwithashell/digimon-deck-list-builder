//! TEST-P7-CANCEL2: A second mandatory WhenWouldBeDeleted replacement
//! that sets the outcome to `CustomHandled`. Paired with `TestP7Cancel` in
//! layering tests. Its draw witnesses activation even when a prior
//! cancellation remains the resulting window outcome.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestP7Cancel2;

impl CardEffect for TestP7Cancel2 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Handled deletion (mandatory)")
            .replacement_process(|rctx| {
                rctx.effect.draw(1, 1);
                rctx.handled();
            })
            .build()]
    }
}
