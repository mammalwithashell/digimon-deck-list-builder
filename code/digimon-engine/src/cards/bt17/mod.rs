//! BT17 — Versus Royal Knights. Template directory for the first
//! production set. Kept empty until `/batch-implement-cards-rust` starts
//! landing hand-written `CardEffect` impls per-card.
//!
//! Layout convention mirrors `src/cards/test/`: one file per card,
//! `bt17_NNN.rs` with a public `Bt17_NNN` struct implementing `CardEffect`.
//! Add each new card's `mod bt17_NNN;` line below and a matching
//! `registry.insert(...)` call in `register()`.

#![allow(unused_imports)]

use crate::cards::CardEffectRegistry;

/// Register all BT17 card effects into the registry. Empty until the
/// first card lands.
pub fn register(_registry: &mut CardEffectRegistry) {
    // Cards will register here as they are implemented — e.g.:
    // registry.insert("BT17-015", std::sync::Arc::new(bt17_015::Bt17_015));
}
