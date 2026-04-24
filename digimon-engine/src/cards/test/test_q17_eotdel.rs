//! TEST-Q17-EOTDEL: "[End of Your Turn] Delete this Digimon."
//!
//! Distilled from ZXavier's Digi Rulings quiz question 17 (does
//! Magnamon X's "[End of Your Turn] Delete this Digimon" fire on the
//! turn you digivolve into it?). Rule 15-16-12-1 says [End of Your
//! Turn] triggers "when your turn ends" — no restriction on how long
//! the Digimon has been on the field. This card is the simplest form
//! of that effect: its own end-of-turn trigger deletes itself.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct TestQ17Eotdel;

impl CardEffect for TestQ17Eotdel {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::end_of_your_turn(card)
            .name("[End of Your Turn] Delete this Digimon")
            .process(|ctx| {
                if let Some(me) = ctx.source_permanent {
                    ctx.delete_permanent(me);
                }
            })
            .build()]
    }
}
