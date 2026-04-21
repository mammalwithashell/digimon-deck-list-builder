//! Petrification Token — Medusamon archetype (Purple).
//!
//! Printed text:
//!   [Your Turn] This Digimon can't suspend.
//!   [On Deletion] Trash the top card of this Digimon's owner's security stack.
//!
//! Phase 10 ships the OnDeletion clause. The CannotSuspend [Your Turn]
//! rider depends on a condition-gated modifier primitive tracked in
//! `RUST_ENGINE_GAPS.md` §"Condition-gated modifier entries"; when that
//! lands, append a second `Effect` for the CannotSuspend clause.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct PetrificationToken;

impl CardEffect for PetrificationToken {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_deletion(card)
            .name("[On Deletion] Trash top of owner's security")
            .process(|ctx| {
                // The token's owner = the player who controls the token
                // permanent = `ctx.player` (EffectContext is always
                // scoped to the source's controller).
                let owner = ctx.player;
                ctx.trash_top_security(owner);
            })
            .build()]
    }
}
