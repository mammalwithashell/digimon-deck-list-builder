//! Familiar Token — TS Olympos archetype (Yellow).
//!
//! Printed text: "[On Deletion] 1 of your opponent's Digimon gets
//! -3000 DP for the turn."
//!
use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::Expiry;

pub struct FamiliarToken;

impl CardEffect for FamiliarToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] -3000 DP to opponent Digimon")
            .process(|ctx| {
                ctx.select_opponent_permanent(
                    "Choose 1 of your opponent's Digimon",
                    false,
                    |game, h| {
                        game.players[h.player as usize]
                            .battle_area
                            .get(h.index as usize)
                            .is_some_and(|p| p.is_digimon(&game.card_data))
                    },
                    |ctx, target| {
                        ctx.add_dp_modifier(target, -3000, Expiry::EndOfTurn);
                    },
                );
            })
            .build()]
    }
}
