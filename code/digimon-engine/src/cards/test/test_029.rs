//! TEST-029: "OnPlay: play 1 Familiar Token."

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test029;

impl CardEffect for Test029 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Familiar Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "familiar");
            })
            .build()]
    }
}
