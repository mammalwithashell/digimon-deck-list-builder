//! TEST-004: "When Digivolving: Gain 2 memory if opponent has any Digimon."
//! Exercises WhenDigivolving timing with a condition closure.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test004;

impl CardEffect for Test004 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Gain 2 memory if opp has Digimon")
            .condition(|ctx| {
                let opp = ctx.opponent_id();
                ctx.battle_area(opp)
                    .iter()
                    .any(|p| p.is_digimon(ctx.card_data()))
            })
            .process(|ctx| {
                ctx.gain_memory(2);
            })
            .build()]
    }
}
