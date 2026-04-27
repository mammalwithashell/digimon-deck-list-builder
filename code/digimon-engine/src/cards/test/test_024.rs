//! TEST-024: "OnPlay: De-Digivolve 2 (stop at Lv3) on opponent's
//! permanent at field index 0." Deterministic target selection for
//! test purposes — real De-Digivolve cards use `pending_selection`
//! for target pick, but this synthetic card hardcodes the target so
//! we isolate the pop-and-trash logic from the selection primitive.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::permanent::PermanentHandle;

pub struct Test024;

impl CardEffect for Test024 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("DeDigivolve 2 stop-at-3 on opp[0]")
            .process(|ctx| {
                let opp = ctx.opponent_id();
                if ctx.battle_area(opp).is_empty() {
                    return;
                }
                let target = PermanentHandle {
                    player: opp,
                    index: 0,
                };
                ctx.de_digivolve(target, Some(3), Some(2));
            })
            .build()]
    }
}
