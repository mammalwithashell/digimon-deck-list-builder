//! `CardRegistry` — the runtime lookup surface. Phase 1b holds
//! `CompiledCard` values; Phase 1c wraps each with lowered `Effect`
//! closures for engine consumption.

use std::collections::HashMap;
use std::path::Path;

use crate::compile::compile;
use crate::compiled::CompiledCard;
use crate::errors::ValidationError;
use crate::pack::{CardPack, PackManifest};
use crate::spec::CardSpec;

pub struct CardRegistry {
    pub manifest: PackManifest,
    cards: HashMap<String, CompiledCard>,
}

impl CardRegistry {
    /// Dev / test path — compile in-memory without serializing.
    pub fn from_specs(pack_id: impl Into<String>, specs: &[CardSpec]) -> Result<Self, Vec<ValidationError>> {
        let mut cards = HashMap::new();
        let mut errors = Vec::new();
        for spec in specs {
            match compile(spec) {
                Ok(c) => { cards.insert(c.card.clone(), c); }
                Err(mut errs) => errors.append(&mut errs),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let pack = CardPack::new(pack_id, cards.values().cloned().collect());
        Ok(Self { manifest: pack.manifest, cards })
    }

    /// Desktop / runtime path — deserialize from embedded bytes.
    pub fn from_pack_bytes(bytes: &[u8]) -> Result<Self, String> {
        let pack = CardPack::from_bytes(bytes)?;
        Ok(Self::from_pack(pack))
    }

    /// Cache-directory path — read a .pack file.
    pub fn from_pack_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read pack file: {e}"))?;
        Self::from_pack_bytes(&bytes)
    }

    fn from_pack(pack: CardPack) -> Self {
        let cards = pack.cards.into_iter().map(|c| (c.card.clone(), c)).collect();
        Self { manifest: pack.manifest, cards }
    }

    pub fn lookup(&self, card_id: &str) -> Option<&CompiledCard> {
        self.cards.get(card_id)
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &CompiledCard)> {
        self.cards.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_dir_ok;
    use std::path::PathBuf;

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine/cards/_examples")
    }

    #[test]
    fn registry_from_specs_compiles_all_examples() {
        let (specs, errs) = load_dir_ok(&examples_dir());
        assert!(errs.is_empty());
        let registry = CardRegistry::from_specs("phase1b-test", &specs).expect("compile");
        assert_eq!(registry.len(), 15);
        assert!(registry.lookup("ST2-13").is_some());
    }

    #[test]
    fn registry_round_trips_through_bytes() {
        let (specs, _) = load_dir_ok(&examples_dir());
        let registry = CardRegistry::from_specs("phase1b-test", &specs).unwrap();

        let pack = CardPack {
            manifest: registry.manifest.clone(),
            cards: registry.iter().map(|(_, c)| c.clone()).collect(),
        };
        let bytes = pack.to_bytes().unwrap();

        let reparsed = CardRegistry::from_pack_bytes(&bytes).unwrap();
        assert_eq!(reparsed.len(), registry.len());
        for (card_id, card) in registry.iter() {
            assert_eq!(reparsed.lookup(card_id), Some(card));
        }
    }

    #[test]
    fn registry_from_pack_file_round_trip() {
        let (specs, _) = load_dir_ok(&examples_dir());
        let registry = CardRegistry::from_specs("file-test", &specs).unwrap();
        let pack = CardPack {
            manifest: registry.manifest.clone(),
            cards: registry.iter().map(|(_, c)| c.clone()).collect(),
        };
        let bytes = pack.to_bytes().unwrap();

        let mut temp = std::env::temp_dir();
        temp.push("digimon-dsl-registry-test.pack");
        std::fs::write(&temp, &bytes).unwrap();

        let loaded = CardRegistry::from_pack_file(&temp).unwrap();
        assert_eq!(loaded.len(), registry.len());
        assert_eq!(loaded.manifest.pack_id, "file-test");

        let _ = std::fs::remove_file(&temp);
    }
}
