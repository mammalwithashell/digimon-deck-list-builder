//! Familiar Token — TS Olympos archetype (Yellow).
//!
//! Printed text: "[On Deletion] 1 of your opponent's Digimon gets
//! -3000 DP for the turn."
//!
//! The selection primitive this depends on (opponent-permanent pick
//! with a callback) is a Phase-6+ gap. Phase 10 ships the stat line
//! only; the [On Deletion] effect lands when the selection primitive
//! does.

use crate::card_source::CardHandle;
use crate::effect::{CardEffect, Effect};

pub struct FamiliarToken;

impl CardEffect for FamiliarToken {
    fn effects(&self, _card: CardHandle) -> Vec<Effect> {
        Vec::new()
    }
}
