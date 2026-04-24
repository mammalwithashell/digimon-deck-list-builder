//! TEST-Q10-CAPMEM: "On Play: Gain 20 memory."
//!
//! Distilled from ZXavier's Digi Rulings quiz questions 10 and 11 (the
//! Mental Training scenarios): how much memory do you end up with when
//! an effect would push you over the 10-memory ceiling? Rule 4-1-2-2
//! says memory can't exceed 10, so the gain is clamped. This card gives
//! a gross overage (20) so the test unambiguously targets the clamp
//! rather than any off-by-one in the gain math.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestQ10Capmem;

impl CardEffect for TestQ10Capmem {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Gain 20 memory (clamped to 10)")
            .process(|ctx| {
                ctx.gain_memory(20);
            })
            .build()]
    }
}
