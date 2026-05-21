//! TEST-030: "OnPlay: play 1 Atho, René & Por Token."
//!
//! Exercises Royal Knights Jesmon token registration end-to-end through
//! the play pipeline (BT20-017, BT23-013 family).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test030;

impl CardEffect for Test030 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play an Atho, René & Por Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "atho_rene_por");
            })
            .build()]
    }
}
