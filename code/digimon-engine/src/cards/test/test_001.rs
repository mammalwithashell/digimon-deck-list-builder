//! TEST-001: "On Play: Gain 1 memory."
//! Exercises basic OnPlay effect with a memory mutation.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test001;

impl CardEffect for Test001 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 1 memory")
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}
