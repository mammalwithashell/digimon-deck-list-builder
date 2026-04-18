//! TEST-003: "On Play: All your Digimon get +1000 DP for the turn."
//! Exercises modifier registration with end-of-turn expiry.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::Expiry;

pub struct Test003;

impl CardEffect for Test003 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Buff allies +1000 DP")
            .process(|ctx| {
                let me = ctx.player;
                let count = ctx.battle_area(me).len();
                for i in 0..count {
                    let h = crate::permanent::PermanentHandle {
                        player: me,
                        index: i as u8,
                    };
                    ctx.add_dp_modifier(h, 1000, Expiry::EndOfTurn);
                }
            })
            .build()]
    }
}
