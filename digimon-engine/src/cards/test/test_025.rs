//! TEST-025: "OnPlay: De-Digivolve unbounded on opp[0] (TS Olympos
//! Ikkakumon-style pop-whole-stack)."

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::permanent::PermanentHandle;

pub struct Test025;

impl CardEffect for Test025 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("DeDigivolve unbounded on opp[0]")
            .process(|ctx| {
                let opp = ctx.opponent_id();
                if ctx.battle_area(opp).is_empty() {
                    return;
                }
                let target = PermanentHandle { player: opp, index: 0 };
                ctx.de_digivolve(target, None, None);
            })
            .build()]
    }
}
