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
        // dsl-substrate-integrity: validate every raw_rust reference in the
        // embedded pack (step, declarative, OR formula `amount_fn: { raw_rust }`)
        // against `EngineRawRustRegistry`. An unregistered ref is a silent
        // runtime no-op (the BT13-007 class), so this is a HARD ERROR — register
        // the fn under `src/cards/raw_rust/` or author the card in pure DSL.
        //
        // Promoted from warn-mode in collapse §4.5.6: the last offender,
        // EX11-027 Maquinamon, migrated off test-only raw_rust (§4.5.5), so the
        // pack now has zero unregistered refs (see fix-dsl-substrate-rot-and-bugs
        // §1). The matching guard test is `unregistered_raw_rust_ref_fails_load`.
        let pack = crate::dsl_registry::from_embedded_with_raw_registry(raw.as_ref())
            .unwrap_or_else(|msg| {
                panic!(
                    "dsl-substrate-integrity: the embedded DSL pack contains unregistered \
                     raw_rust reference(s): {msg}. A raw_rust ref absent from \
                     EngineRawRustRegistry is a silent runtime no-op — either register the \
                     fn under src/cards/raw_rust/ or author the card in pure DSL."
                )
            });
        if let Err(msg) = raw_rust::raw_rust_budget_status(raw.registered_fn_count(), pack.len()) {
            eprintln!("WARNING: {msg}");
        }
        crate::dsl_cards::register_dsl_cards_with_raw(&mut registry, &pack, raw);
    }

    registry
}

/// Process-cached `build_registry()`.
///
/// `build_registry()` parses and lowers the entire embedded DSL pack (~4000
/// cards) — ~190 ms. The production game path (`Game::new`) builds a registry
/// per game, and `DigimonEnv.reset()` constructs a fresh game **per episode**,
/// so rebuilding here once dominated training wall-time.
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

#[cfg(all(test, feature = "dsl-yaml-loader"))]
mod substrate_integrity_tests {
    use super::*;

    /// §4.5.6 — the promoted `dsl-substrate-integrity` guard. A pack whose
    /// manifest names a raw_rust fn absent from the registry must FAIL the
    /// integrity check (that unregistered ref is the silent runtime no-op the
    /// promoted `build_registry()` now panics on). A synthetic pack makes this
    /// independent of the embedded pack, which declares ZERO raw_rust fns now
    /// that EX11-027 migrated off (see `production_registry_loads_pack_cleanly`).
    #[test]
    fn unregistered_raw_rust_ref_fails_load() {
        let pack = digimon_dsl::CardPack::new_with_required_raw_rust_fns(
            "guard-test",
            vec![],
            ["never_registered_fn"],
        );
        let bytes = pack.to_bytes().expect("serialize synthetic pack");
        let empty = crate::dsl_cards::raw_rust::EngineRawRustRegistry::new();
        let err = digimon_dsl::CardRegistry::from_pack_bytes_with_raw_registry(&bytes, &empty)
            .err()
            .expect("a pack declaring an unregistered raw_rust fn must be rejected");
        assert!(
            err.contains("never_registered_fn"),
            "the integrity error must name the missing raw_rust fn: {err}"
        );
    }

    /// The production raw_rust registry satisfies every pack raw_rust ref: after
    /// EX11-027 Maquinamon's pure-DSL migration the pack has zero unregistered
    /// refs, so the promoted guard loads cleanly (no panic).
    #[test]
    fn production_registry_loads_pack_cleanly() {
        let raw = raw_rust::build_registry();
        assert!(
            crate::dsl_registry::from_embedded_with_raw_registry(&raw).is_ok(),
            "the production raw-rust registry must satisfy every pack raw_rust ref \
             (zero unregistered refs after EX11-027 migrated off raw_rust)"
        );
    }
}
