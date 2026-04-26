//! Engine-side DSL registry adapters — desktop / runtime entry points
//! that wrap `digimon_dsl::CardRegistry`.
//!
//! Named `dsl_registry` to avoid collision with the engine's unrelated
//! `card_registry` module (tensor-indexing card_id↔integer mapping).

use digimon_dsl::CardRegistry;
use std::path::Path;

// Embedded pack blob, produced by build.rs.
static CARDS_PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cards.pack"));

/// Load the card registry from the bytes embedded at build time.
pub fn from_embedded() -> Result<CardRegistry, String> {
    CardRegistry::from_pack_bytes(CARDS_PACK)
}

/// Load the card registry from a cache-directory pack file — used by
/// desktop binaries to pick up runtime-downloaded updates.
pub fn from_pack_file(path: &Path) -> Result<CardRegistry, String> {
    CardRegistry::from_pack_file(path)
}
