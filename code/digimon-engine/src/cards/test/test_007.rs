//! TEST-007: "End of your turn (Optional): Gain 2 memory."
//! Exercises `TriggerOrder` + decline-all with an optional queued effect.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test007;

impl CardEffect for Test007 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("(Optional) Gain 2 memory at end of turn")
            .optional()
            .process(|ctx| {
                ctx.gain_memory(2);
            })
            .build()]
    }
}
