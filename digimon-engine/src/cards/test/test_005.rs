//! TEST-005: "On Deletion: Lose 1 memory."
//! Exercises OnDeletion timing.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test005;

impl CardEffect for Test005 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("Lose 1 memory on deletion")
            .process(|ctx| {
                ctx.lose_memory(1);
            })
            .build()]
    }
}
