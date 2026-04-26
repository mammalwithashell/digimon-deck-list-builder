//! TEST-013: Blast-digivolve pilot.
//! Carries a single `.blast_digivolve()` flag + a `WhenDigivolving` hook
//! that grants +1 memory so tests can verify the post-stack trigger
//! fires. The card's `EvoCost` (populated via `make_test_blast_card` in
//! the test harness) controls what field targets are eligible.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test013;

impl CardEffect for Test013 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::when_digivolving(card)
            .name("Blast-digivolve pilot (+1 memory)")
            .blast_digivolve()
            .process(|ctx| {
                ctx.gain_memory(1);
            })
            .build()]
    }
}
