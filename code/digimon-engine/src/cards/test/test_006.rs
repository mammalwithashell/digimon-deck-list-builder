//! TEST-006: "End of your turn: Gain 5 memory."
//! Exercises EndOfYourTurn timing and memory swing-back (§1.5).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test006;

impl CardEffect for Test006 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("Gain 5 memory at end of turn")
            .process(|ctx| {
                ctx.gain_memory(5);
            })
            .build()]
    }
}
