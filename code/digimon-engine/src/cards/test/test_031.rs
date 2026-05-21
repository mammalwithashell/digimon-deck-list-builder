//! TEST-031: "OnPlay: play 1 Hinukamuy Token."
//!
//! Exercises Royal Knights Gankoomon token registration end-to-end
//! through the play pipeline (BT23-057).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test031;

impl CardEffect for Test031 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Play a Hinukamuy Token")
            .process(|ctx| {
                let me = ctx.player;
                ctx.play_token(me, "hinukamuy");
            })
            .build()]
    }
}
