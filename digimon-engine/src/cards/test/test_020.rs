//! TEST-020: "[Security] Draw 2 cards."
//! Pilot for `SecuritySkill` trigger-and-trash — exercises the
//! security-reveal → enqueue → drain → trash pipeline with a simple
//! draw effect. The revealed card ends up in the defender's trash.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test020;

impl CardEffect for Test020 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Draw 2")
            .process(|ctx| {
                let me = ctx.player;
                ctx.draw(me, 2);
            })
            .build()]
    }
}
