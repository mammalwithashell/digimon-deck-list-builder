//! Petrification Token — Medusamon archetype (Purple).
//!
//! Printed text: "[Your Turn] This Digimon can't suspend.
//! [On Deletion] Trash the top card of this Digimon's owner's
//! security stack."
//!
//! Task 3 wires the OnDeletion trash-top-security via
//! `ctx.trash_top_security(...)`. The CannotSuspend [Your Turn] rider
//! depends on a modifier framework piece scheduled for a later phase
//! (see parity §4.6b-residual).

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct PetrificationToken;

impl CardEffect for PetrificationToken {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        // OnDeletion effect wired in Task 3.
        Vec::new()
    }
}
