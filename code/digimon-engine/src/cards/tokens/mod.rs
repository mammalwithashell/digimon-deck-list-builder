//! Token printed abilities. Parallels `src/cards/test/` but indexed by
//! synthetic token card_ids (`TOKEN_PETRIFICATION`, `TOKEN_FAMILIAR`).
//! Tokens without printed abilities may omit their entry here entirely —
//! the registry lookup returns `None` and the engine treats them as a
//! vanilla permanent with base stats only.

use std::sync::Arc;

use crate::cards::{CardEffectRegistry, GuardedCardEffect, RegisteredCardEffect};

mod familiar;
mod petrification;

pub fn register(registry: &mut CardEffectRegistry) {
    // This exact unit constructor owns no captures and accepts only a scalar
    // source handle. Its fixed body reads the controller and delegates security
    // movement to the engine; checked event references remain checked there.
    // Bind authority to this stored factory, never to a card-ID allowlist:
    // public insert() replaces the entire record with an opaque entry.
    registry.effects.insert("TOKEN_PETRIFICATION".into(), RegisteredCardEffect {
        factory: Arc::new(GuardedCardEffect(Arc::new(petrification::PetrificationToken), true)),
        compiled_origin: None,
        read_fence_constructible: true,
    });
    registry.generation = Arc::new(());
    registry.insert("TOKEN_FAMILIAR", Arc::new(familiar::FamiliarToken));
}
