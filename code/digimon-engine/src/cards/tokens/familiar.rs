//! Familiar Token — TS Olympos archetype (Yellow).
//!
//! Printed text: "[On Deletion] 1 of your opponent's Digimon gets
//! -3000 DP for the turn."
//!
use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};
use crate::enums::Expiry;
use crate::resume::{
    FamiliarTokenOnDeletionSelectionState, ResumeFrame, ResumeProvenance, ResumeStack,
};

pub struct FamiliarToken;

impl CardEffect for FamiliarToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] -3000 DP to opponent Digimon")
            .process(|ctx| {
                let prov = ResumeProvenance {
                    source_card: ctx.source_card,
                    source_permanent: ctx.source_permanent,
                    source_kind: ctx.source_kind,
                    controller: ctx.player,
                    override_pin: ctx.override_selecting_player(),
                };
                let target_player = ctx.game.next_clockwise(ctx.player);
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
                if ctx.game.pending_selection.is_some() {
                    ctx.game.pending_selection_resume = Some(ResumeStack {
                        frames: vec![ResumeFrame::FamiliarTokenOnDeletionSelection(
                            FamiliarTokenOnDeletionSelectionState {
                                prov,
                                target_player,
                                outer_conts: Vec::new(),
                            },
                        )],
                    });
                }
            })
            .build()]
    }
}
