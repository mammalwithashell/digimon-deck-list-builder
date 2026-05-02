//! Token printed abilities. Parallels `src/cards/test/` but indexed by
//! synthetic token card_ids (`TOKEN_PETRIFICATION`, `TOKEN_FAMILIAR`).
//! Tokens without printed abilities may omit their entry here entirely —
//! the registry lookup returns `None` and the engine treats them as a
//! vanilla permanent with base stats only.

use std::sync::Arc;

use crate::cards::CardEffectRegistry;

mod familiar;
mod petrification;

pub fn register(registry: &mut CardEffectRegistry) {
    registry.insert(
        "TOKEN_PETRIFICATION",
        Arc::new(petrification::PetrificationToken),
    );
    registry.insert("TOKEN_FAMILIAR", Arc::new(familiar::FamiliarToken));
}
