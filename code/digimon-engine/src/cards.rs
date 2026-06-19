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
///
/// Cheaply `Clone`: entries are `Arc<dyn CardEffect>` (stateless, shared), so a
/// clone duplicates only the `HashMap` of string keys + Arc handles — not the
/// ~4000-card DSL parse/lowering that `build_registry()` performs. This is what
/// lets `build_registry_cached()` build the pack once and hand each game a cheap
/// clone (see that fn).
#[derive(Default, Clone)]
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
        let raw = Arc::new(raw_rust::build_registry());
        // Validate raw_rust references in the embedded pack. An unregistered ref
        // (step, declarative, OR formula `amount_fn: { raw_rust }`) is a silent
        // no-op at runtime (the BT13-007 class) — surface it loudly.
        //
        // WARN-MODE for now: the only remaining offender is EX11-027 Maquinamon,
        // whose link clauses need substrate that does not exist yet (filed as
        // G-DSL-LINK-* in qa/dsl-vocab-gaps.md). PROMOTE this to a hard error
        // (panic) once EX11-027 migrates off test-only raw_rust and the pack has
        // zero unregistered refs. See fix-dsl-substrate-rot-and-bugs §1.
        let pack = match crate::dsl_registry::from_embedded_with_raw_registry(raw.as_ref()) {
            Ok(p) => Some(p),
            Err(msg) => {
                eprintln!(
                    "WARNING: {msg} — loading WITHOUT raw_rust validation (promote to a hard \
                     error once EX11-027's link clauses migrate off raw_rust)"
                );
                crate::dsl_registry::from_embedded().ok()
            }
        };
        match pack {
            Some(pack) => {
                if let Err(msg) =
                    raw_rust::raw_rust_budget_status(raw.registered_fn_count(), pack.len())
                {
                    eprintln!("WARNING: {msg}");
                }
                crate::dsl_cards::register_dsl_cards_with_raw(&mut registry, &pack, raw);
            }
            None => eprintln!("DSL embedded pack failed to load"),
        }
    }

    registry
}

/// Process-cached `build_registry()`.
///
/// `build_registry()` parses and lowers the entire embedded DSL pack (~4000
/// cards) — ~190 ms. The production game path (`Game::new`) builds a registry
/// per game, and `DigimonEnv.reset()` constructs a fresh game **per episode**,
/// so rebuilding here once dominated training wall-time (and flooded the log
/// with the EX11-027 raw_rust warning once per episode).
///
/// The built registry is read-only on the production path (only `get()` is
/// called after construction; the only mutators are DebugRunner test helpers,
/// which keep using the uncached `build_registry()`). Effects are stateless
/// `Arc<dyn CardEffect>`, so handing each game a cheap *clone* of a
/// process-built template (~1 ms: clone the key/Arc HashMap) is correct and
/// ~100× cheaper than re-lowering the pack. The expensive build — and its
/// one-time warning — now runs once per process instead of once per episode.
pub fn build_registry_cached() -> CardEffectRegistry {
    static CACHE: std::sync::OnceLock<CardEffectRegistry> = std::sync::OnceLock::new();
    CACHE.get_or_init(build_registry).clone()
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    /// `build_registry_cached()` must hand back the SAME underlying effect Arcs
    /// across calls — proving it reuses the process-built template rather than
    /// re-lowering the ~4000-card pack each time (the per-episode cost that
    /// dominated training wall-time). A fresh `build_registry()` would allocate
    /// distinct Arcs, so `Arc::ptr_eq` is the discriminating assertion.
    #[test]
    fn build_registry_cached_shares_effect_arcs_across_calls() {
        let a = build_registry_cached();
        let b = build_registry_cached();
        let ids = a.registered_card_ids();
        assert!(!ids.is_empty(), "registry should have registered effects");
        let id = &ids[0];
        let ea = a.get(id).expect("effect present in registry a");
        let eb = b.get(id).expect("effect present in registry b");
        assert!(
            Arc::ptr_eq(&ea, &eb),
            "build_registry_cached must reuse the cached registry (shared Arc), \
             not re-lower the pack per call"
        );
    }
}
