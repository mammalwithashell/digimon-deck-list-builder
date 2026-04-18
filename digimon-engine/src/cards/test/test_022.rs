//! TEST-022: "[Security] Gain 3 memory."
//! Pilot for observer-timing parity — a second non-destructive effect
//! so the harness can distinguish "effect fired" (memory changed) from
//! "effect fired AND card is on field" (TEST-021's signature). Trashes
//! the revealed card after firing.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test022;

impl CardEffect for Test022 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::security(card)
            .name("[Security] Gain 3 memory")
            .process(|ctx| {
                ctx.gain_memory(3);
            })
            .build()]
    }
}
