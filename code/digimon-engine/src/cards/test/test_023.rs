//! TEST-023: "OnPlay: play 1 Petrification Token."
//! Exercises `ctx.play_token` through the full play pipeline.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test023;

impl CardEffect for Test023 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Petrification Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "petrification");
            })
            .build()]
    }
}
