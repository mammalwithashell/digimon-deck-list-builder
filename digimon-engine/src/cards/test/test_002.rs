//! TEST-002: "On Play: Draw 2 cards."
//! Exercises card draw via EffectContext.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test002;

impl CardEffect for Test002 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Draw 2")
            .process(|ctx| {
                let me = ctx.player;
                ctx.draw(me, 2);
            })
            .build()]
    }
}
