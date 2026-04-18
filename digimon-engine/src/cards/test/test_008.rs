//! TEST-008: "End of your turn: Lose 3 memory."
//! Paired with TEST-006 in drainer tests to disambiguate resolution
//! order from final-state aggregates.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test008;

impl CardEffect for Test008 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("Lose 3 memory at end of turn")
            .process(|ctx| {
                ctx.lose_memory(3);
            })
            .build()]
    }
}
