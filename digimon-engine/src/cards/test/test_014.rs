//! TEST-014: Blast-digivolve pilot whose WhenDigivolving deletes the
//! current attacker. Used to test the Counter cascade where the blast
//! kills the attacker mid-attack, forcing the state machine to skip
//! BlockOpen/Battle and land in Cleanup.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test014;

impl CardEffect for Test014 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Blast: delete the attacker")
            .blast_digivolve()
            .process(|ctx| {
                if let Some(attacker) = ctx.game.pending_attack.as_ref().map(|pa| pa.attacker) {
                    ctx.delete_permanent(attacker);
                }
            })
            .build()]
    }
}
