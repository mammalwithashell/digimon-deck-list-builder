//! TEST-010: "On Play: Delete 1 of your opponent's Digimon. (Optional)"
//! Pilot for the selection subsystem — exercises `select_opponent_permanent`
//! with a mandatory/optional flip and a `delete_permanent` mutation inside
//! the resolution callback.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct Test010;

impl CardEffect for Test010 {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card)
            .name("Delete 1 opp Digimon (optional)")
            .process(|ctx| {
                ctx.select_opponent_permanent(
                    "Delete 1 of your opponent's Digimon",
                    /* is_optional = */ true,
                    // Only Digimon are valid targets — Tamer slots are skipped.
                    |game, handle| {
                        game.player(handle.player)
                            .battle_area
                            .get(handle.index as usize)
                            .map(|p| p.is_digimon(&game.card_data))
                            .unwrap_or(false)
                    },
                    |ctx, chosen| {
                        ctx.delete_permanent(chosen);
                    },
                );
            })
            .build()]
    }
}
