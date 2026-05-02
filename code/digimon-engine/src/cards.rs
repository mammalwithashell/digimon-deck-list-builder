//! Card effect registry — maps card_id strings to their CardEffect implementation.
//!
//! Production card effects are DSL-authored and loaded from the embedded pack.
//!
//! This module remains as a thin shell for test effects, token printed
//! abilities, keyword auto-effects, and raw-rust escape-hatch functions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::effect::CardEffect;

pub mod keyword_effects;
pub mod raw_rust;
pub mod test;
pub mod tokens;

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

    /// Return all card IDs that have a registered effect implementation.
    /// Mirrors the Python `load_implemented_card_ids` helper that read
    /// `_frozen_manifest.json`; the Rust source of truth is the registry
    /// itself rather than an external manifest.
    pub fn registered_card_ids(&self) -> Vec<String> {
        self.effects.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.effects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }
}

/// Build the default registry with all built-in card effects.
/// Test cards and token printed abilities are always included. Production cards
/// are registered from the embedded DSL pack below.
pub fn build_registry() -> CardEffectRegistry {
    let mut registry = CardEffectRegistry::new();
    test::register(&mut registry);
    tokens::register(&mut registry);

    // DSL-authored cards (embedded at build time via build.rs → cards.pack).
    // Registered AFTER hand-written sets so DSL overrides on collision.
    #[cfg(feature = "dsl-yaml-loader")]
    {
        match crate::dsl_registry::from_embedded() {
            Ok(pack) => {
                let raw = Arc::new(raw_rust::build_registry());
                if let Err(msg) =
                    raw_rust::raw_rust_budget_status(raw.registered_fn_count(), pack.len())
                {
                    eprintln!("WARNING: {msg}");
                }
                crate::dsl_cards::register_dsl_cards_with_raw(&mut registry, &pack, raw);
            }
            Err(e) => eprintln!("DSL embedded pack failed to load: {e}"),
        }
    }

    registry
}
