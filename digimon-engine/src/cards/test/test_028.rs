//! TEST-028: Card with TWO `[Security]` effects. Used by the §2.5i
//! `TriggerOrder`-suppression pilot — both effects belong to the same
//! revealed card, so the drainer must fire them in collection order
//! without installing a prompt.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test028;

impl CardEffect for Test028 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![
            Effect::security(card)
                .name("[Security] Gain 2 memory")
                .process(|ctx| ctx.gain_memory(2))
                .build(),
            Effect::security(card)
                .name("[Security] Draw 1")
                .process(|ctx| {
                    let me = ctx.player;
                    ctx.draw(me, 1);
                })
                .build(),
        ]
    }
}
