//! Card effect registry — maps card_id strings to their CardEffect implementation.
//!
//! Each set has its own submodule that registers all its cards.

use std::collections::HashMap;
use std::sync::Arc;

use crate::effect::CardEffect;

pub mod test_cards;

/// Registry of card_id -> CardEffect implementation.
#[derive(Default)]
pub struct CardEffectRegistry {
    effects: HashMap<String, Arc<dyn CardEffect>>,
}

impl std::fmt::Debug for CardEffectRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CardEffectRegistry")
            .field("len", &self.effects.len())
            .finish()
    }
}

impl CardEffectRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a card's effect implementation.
    pub fn insert(&mut self, card_id: &str, effect: Arc<dyn CardEffect>) {
        self.effects.insert(card_id.to_string(), effect);
    }

    /// Look up a card's effect implementation.
    pub fn get(&self, card_id: &str) -> Option<Arc<dyn CardEffect>> {
        self.effects.get(card_id).cloned()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

/// Build the default registry with all built-in card effects.
/// Test cards are always included; production cards register from set modules.
pub fn build_registry() -> CardEffectRegistry {
    let mut registry = CardEffectRegistry::new();
    test_cards::register(&mut registry);
    registry
}
