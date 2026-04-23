//! Pack container: manifest + compiled cards.
//!
//! Pack format (bincode-serialized):
//!   pack_version: pack-level semver
//!   min_engine_version: minimum digimon-engine semver that can load this pack
//!   max_engine_version: optional upper bound for breaking schema changes
//!   required_raw_rust_fns: fn names the pack references; desktop rejects if missing
//!   pack_id: "BT17", "core", etc. for cache-dir segregation
//!   cards: every compiled card in the pack

use serde::{Deserialize, Serialize};
use crate::compiled::CompiledCard;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardPack {
    pub manifest: PackManifest,
    pub cards: Vec<CompiledCard>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackManifest {
    pub pack_id: String,
    pub pack_version: String,
    pub min_engine_version: String,
    pub max_engine_version: Option<String>,
    pub required_raw_rust_fns: Vec<String>,
}

impl CardPack {
    pub fn new(pack_id: impl Into<String>, cards: Vec<CompiledCard>) -> Self {
        Self {
            manifest: PackManifest {
                pack_id: pack_id.into(),
                pack_version: env!("CARGO_PKG_VERSION").to_string(),
                min_engine_version: "0.1.0".into(),
                max_engine_version: None,
                required_raw_rust_fns: Vec::new(),
            },
            cards,
        }
    }

    /// Serialize to bincode bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, String> {
        bincode::serialize(self).map_err(|e| format!("bincode serialize failed: {e}"))
    }

    /// Deserialize from bincode bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        bincode::deserialize(bytes).map_err(|e| format!("bincode deserialize failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::*;

    #[test]
    fn empty_pack_round_trips() {
        let pack = CardPack::new("test", vec![]);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.manifest.pack_id, "test");
        assert_eq!(reparsed.cards.len(), 0);
    }

    #[test]
    fn pack_with_one_card_round_trips() {
        let card = CompiledCard {
            card: "X-1".into(),
            name: "Test".into(),
            kind: CompiledCardKind::Option,
            level: None,
            color: vec![CompiledColor::Red],
            cost: Some(0),
            dp: None,
            traits: vec![],
            form: None,
            attribute: None,
            ace_overflow: None,
            identity: None,
            alt_paths: vec![],
            effects: vec![],
        };
        let pack = CardPack::new("test", vec![card.clone()]);
        let bytes = pack.to_bytes().unwrap();
        let reparsed = CardPack::from_bytes(&bytes).unwrap();
        assert_eq!(reparsed.cards, vec![card]);
    }
}
