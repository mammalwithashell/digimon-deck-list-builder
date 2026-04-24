//! TEST-Q16-ONDEL: "[On Deletion] Gain 3 memory."
//!
//! Distilled from ZXavier's Digi Rulings quiz question 16 (does
//! [Partition] trigger when Paildramon is deleted by Lilithmon's
//! effect, or only when deleted by battle?). Rule 15-16-4 defines
//! [On Deletion] as firing "when the card with the effect is
//! deleted" — no restriction on whether the deletion was caused by
//! battle or an effect. This card gives a minimal signal (memory
//! delta) so a test can verify the trigger fires through the
//! `Game::delete_permanent_with_effects` path used by effect-driven
//! deletion.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestQ16Ondel;

impl CardEffect for TestQ16Ondel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] Gain 3 memory")
            .process(|ctx| {
                ctx.gain_memory(3);
            })
            .build()]
    }
}
