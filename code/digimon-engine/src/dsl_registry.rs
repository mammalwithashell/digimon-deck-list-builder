//! Engine-side DSL registry adapters — desktop / runtime entry points
//! that wrap `digimon_dsl::CardRegistry`.
//!
//! Named `dsl_registry` to avoid collision with the engine's unrelated
//! `card_registry` module (tensor-indexing card_id↔integer mapping).

use digimon_dsl::raw_rust_registry::RawRustRegistry;
use digimon_dsl::CardRegistry;
use std::path::Path;

// Embedded pack blob, produced by build.rs.
static CARDS_PACK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cards.pack"));

/// Load the card registry from the bytes embedded at build time.
pub fn from_embedded() -> Result<CardRegistry, String> {
    CardRegistry::from_pack_bytes(CARDS_PACK)
}

/// Load the card registry from embedded bytes and reject packs whose manifest
/// names raw_rust functions that are absent from the provided registry.
pub fn from_embedded_with_raw_registry(
    raw_rust: &dyn RawRustRegistry,
) -> Result<CardRegistry, String> {
    CardRegistry::from_pack_bytes_with_raw_registry(CARDS_PACK, raw_rust)
}

/// Load the card registry from a cache-directory pack file — used by
/// desktop binaries to pick up runtime-downloaded updates.
pub fn from_pack_file(path: &Path) -> Result<CardRegistry, String> {
    CardRegistry::from_pack_file(path)
}

/// Load a runtime pack file and reject missing manifest-declared raw_rust
/// requirements before any cards are accepted.
pub fn from_pack_file_with_raw_registry(
    path: &Path,
    raw_rust: &dyn RawRustRegistry,
) -> Result<CardRegistry, String> {
    CardRegistry::from_pack_file_with_raw_registry(path, raw_rust)
}
