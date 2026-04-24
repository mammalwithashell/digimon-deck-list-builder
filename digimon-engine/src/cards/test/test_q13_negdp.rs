//! TEST-Q13-NEGDP: "On Play: 1 of your opponent's Digimon gets -3000 DP
//! for the turn." (Targets field index 0 for determinism.)
//!
//! Distilled from ZXavier's Digi Rulings quiz questions 13 and 14
//! (Nyabootmon layering -DP onto Rapidmon X Antibody / ShineGreymon:
//! Ruin Mode). Those questions hinge on rule 1-3-8 — multiple DP
//! modifications are totalled and then applied to the base DP, so
//! three separate -3000 instances stack to -9000 rather than each
//! being applied in isolation.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::Expiry;
use crate::permanent::PermanentHandle;

pub struct TestQ13Negdp;

impl CardEffect for TestQ13Negdp {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Opp[0] gets -3000 DP for the turn")
            .process(|ctx| {
                let opp = ctx.opponent_id();
                if ctx.battle_area(opp).is_empty() {
                    return;
                }
                let target = PermanentHandle { player: opp, index: 0 };
                ctx.add_dp_modifier(target, -3000, Expiry::EndOfTurn);
            })
            .build()]
    }
}
