//! TEST-P7-SUBSTITUTE: Mandatory WhenWouldBeDeleted replacement that
//! substitutes the target to another permanent at (0, 1). Used by Phase 7
//! Task 2 dispatcher tests — place two permanents on P0's field, target
//! index 0, and the substitute redirects to index 1.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::permanent::PermanentHandle;
use crate::replacement::ReplacementSubject;

pub struct TestP7Substitute;

impl CardEffect for TestP7Substitute {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_would_be_deleted(card)
            .name("Substitute to (0, 1)")
            .replacement_process(|rctx| {
                rctx.substitute(ReplacementSubject::Permanent(PermanentHandle {
                    player: 0,
                    index: 1,
                }));
            })
            .build()]
    }
}
