//! `CardRegistry` — the runtime lookup surface. Phase 1b holds
//! `CompiledCard` values; Phase 1c wraps each with lowered `Effect`
//! closures for engine consumption.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::compile::compile;
use crate::compiled::CompiledCard;
use crate::errors::ValidationError;
use crate::pack::{
    card_has_raw_declarative, missing_required_raw_rust_fns, CardPack, CardPackEnrichment,
    CardPackIndex, PackManifest,
};
use crate::raw_rust_registry::RawRustRegistry;
use crate::spec::CardSpec;

/// Storage for a single compiled card inside [`CardRegistry`].
///
/// Embedded static packs use `Lazy` so process startup only parses the compact
/// sidecar index (~150 KiB) instead of eagerly materializing all ~4,000
/// `CompiledCard` trees (~340 MiB); each card is deserialized on first access
/// and cached lock-free in its `OnceLock`.
pub enum CardSlot {
    Eager {
        card: Arc<CompiledCard>,
        has_raw_declarative: bool,
        max_copies_override: Option<u8>,
    },
    Lazy {
        bytes: &'static [u8],
        has_raw_declarative: bool,
        max_copies_override: Option<u8>,
        cell: OnceLock<Arc<CompiledCard>>,
    },
}

impl CardSlot {
    pub fn eager(card: Arc<CompiledCard>) -> Self {
        let has_raw_declarative = card_has_raw_declarative(&card);
        let max_copies_override = card.deck_rule.as_ref().map(|r| r.max_copies);
        Self::Eager {
            card,
            has_raw_declarative,
            max_copies_override,
        }
    }

    pub fn lazy(
        bytes: &'static [u8],
        has_raw_declarative: bool,
        max_copies_override: Option<u8>,
    ) -> Self {
        Self::Lazy {
            bytes,
            has_raw_declarative,
            max_copies_override,
            cell: OnceLock::new(),
        }
    }

    pub fn has_raw_declarative(&self) -> bool {
        match self {
            CardSlot::Eager {
                has_raw_declarative,
                ..
            }
            | CardSlot::Lazy {
                has_raw_declarative,
                ..
            } => *has_raw_declarative,
        }
    }

    pub fn max_copies_override(&self) -> Option<u8> {
        match self {
            CardSlot::Eager {
                max_copies_override,
                ..
            }
            | CardSlot::Lazy {
                max_copies_override,
                ..
            } => *max_copies_override,
        }
    }

    pub fn get(&self) -> &CompiledCard {
        self.get_arc_ref().as_ref()
    }

    pub fn get_arc(&self) -> Arc<CompiledCard> {
        Arc::clone(self.get_arc_ref())
    }

    fn get_arc_ref(&self) -> &Arc<CompiledCard> {
        match self {
            CardSlot::Eager { card, .. } => card,
            CardSlot::Lazy { bytes, cell, .. } => cell.get_or_init(|| {
                let card: CompiledCard = bincode::deserialize(bytes)
                    .expect("embedded cards.pack card slice must deserialize");
                Arc::new(card)
            }),
        }
    }
}

pub struct CardRegistry {
    pub manifest: PackManifest,
    cards: HashMap<String, Arc<CardSlot>>,
    enrichments: Option<Vec<CardPackEnrichment>>,
}

impl CardRegistry {
    /// Dev / test path — compile in-memory without serializing.
    pub fn from_specs(
        pack_id: impl Into<String>,
        specs: &[CardSpec],
    ) -> Result<Self, Vec<ValidationError>> {
        let mut cards = HashMap::new();
        let mut errors = Vec::new();
        for spec in specs {
            match compile(spec) {
                Ok(c) => {
                    cards.insert(c.card.clone(), Arc::new(CardSlot::eager(Arc::new(c))));
                }
                Err(mut errs) => errors.append(&mut errs),
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let pack = CardPack::from_compiled_cards(
            pack_id,
            cards.values().map(|c| c.get().clone()).collect(),
        );
        Ok(Self {
            manifest: pack.manifest,
            cards,
            enrichments: None,
        })
    }

    /// Lazy embedded-pack path: parse only the compact sidecar index and point
    /// every card slot at its slice inside `pack_bytes` (`cards.pack`).
    pub fn from_static_indexed_pack(
        pack_bytes: &'static [u8],
        index_bytes: &[u8],
    ) -> Result<Self, String> {
        let index = CardPackIndex::from_bytes(index_bytes)?;
        let mut cards = HashMap::with_capacity(index.entries.len());
        for entry in index.entries {
            let start = entry.offset as usize;
            let end = start
                .checked_add(entry.len as usize)
                .ok_or_else(|| format!("pack index overflow for '{}'", entry.card_id))?;
            let slice = pack_bytes.get(start..end).ok_or_else(|| {
                format!(
                    "pack index slice {}..{} out of bounds for '{}' (pack len {})",
                    start,
                    end,
                    entry.card_id,
                    pack_bytes.len()
                )
            })?;
            cards.insert(
                entry.card_id,
                Arc::new(CardSlot::lazy(
                    slice,
                    entry.has_raw_declarative,
                    entry.max_copies_override,
                )),
            );
        }
        Ok(Self {
            manifest: index.manifest,
            cards,
            enrichments: Some(index.enrichments),
        })
    }

    /// Desktop / runtime path — deserialize from embedded bytes.
    pub fn from_pack_bytes(bytes: &[u8]) -> Result<Self, String> {
        let pack = CardPack::from_bytes(bytes)?;
        Ok(Self::from_pack(pack))
    }

    /// Desktop / runtime path with pack-manifest raw_rust enforcement.
    pub fn from_pack_bytes_with_raw_registry(
        bytes: &[u8],
        raw_rust: &dyn RawRustRegistry,
    ) -> Result<Self, String> {
        let pack = CardPack::from_bytes(bytes)?;
        Self::from_pack_with_raw_registry(pack, raw_rust)
    }

    /// Cache-directory path — read a .pack file.
    pub fn from_pack_file(path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read pack file: {e}"))?;
        Self::from_pack_bytes(&bytes)
    }

    /// Cache-directory path with pack-manifest raw_rust enforcement.
    pub fn from_pack_file_with_raw_registry(
        path: &Path,
        raw_rust: &dyn RawRustRegistry,
    ) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("read pack file: {e}"))?;
        Self::from_pack_bytes_with_raw_registry(&bytes, raw_rust)
    }

    fn from_pack(pack: CardPack) -> Self {
        let cards = pack
            .cards
            .into_iter()
            .map(|c| (c.card.clone(), Arc::new(CardSlot::eager(Arc::new(c)))))
            .collect();
        Self {
            manifest: pack.manifest,
            cards,
            enrichments: None,
        }
    }

    fn from_pack_with_raw_registry(
        pack: CardPack,
        raw_rust: &dyn RawRustRegistry,
    ) -> Result<Self, String> {
        let missing = missing_required_raw_rust_fns(&pack.manifest, raw_rust);
        if !missing.is_empty() {
            return Err(format!(
                "pack '{}' requires missing raw_rust fn(s): {}",
                pack.manifest.pack_id,
                missing.join(", ")
            ));
        }
        Ok(Self::from_pack(pack))
    }

    /// Validate that every `raw_rust` fn required by this pack's manifest is
    /// registered in `raw_rust`. Allows sharing one cached `CardRegistry`
    /// without re-deserializing the embedded pack bytes.
    pub fn validate_raw_registry(&self, raw_rust: &dyn RawRustRegistry) -> Result<(), String> {
        let missing = missing_required_raw_rust_fns(&self.manifest, raw_rust);
        if !missing.is_empty() {
            return Err(format!(
                "pack '{}' requires missing raw_rust fn(s): {}",
                self.manifest.pack_id,
                missing.join(", ")
            ));
        }
        Ok(())
    }

    pub fn contains(&self, card_id: &str) -> bool {
        self.cards.contains_key(card_id)
    }

    /// Return the card's printed `deck_rule.max_copies` override without
    /// forcing lazy deserialization of the full `CompiledCard`.
    pub fn max_copies_override(&self, card_id: &str) -> Option<u8> {
        self.cards
            .get(card_id)
            .and_then(|slot| slot.max_copies_override())
    }

    pub fn lookup(&self, card_id: &str) -> Option<&CompiledCard> {
        self.cards.get(card_id).map(|c| c.get())
    }

    pub fn lookup_arc(&self, card_id: &str) -> Option<Arc<CompiledCard>> {
        self.cards.get(card_id).map(|c| c.get_arc())
    }

    pub fn len(&self) -> usize {
        self.cards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn enrichments(&self) -> Option<&[CardPackEnrichment]> {
        self.enrichments.as_deref()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &CompiledCard)> {
        self.cards.iter().map(|(k, v)| (k, v.get()))
    }

    pub fn iter_arcs(&self) -> impl Iterator<Item = (&String, Arc<CompiledCard>)> {
        self.cards.iter().map(|(k, v)| (k, v.get_arc()))
    }

    /// Iterate lazy card slots without forcing deserialization.
    pub fn iter_slots(&self) -> impl Iterator<Item = (&String, &Arc<CardSlot>)> {
        self.cards.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::load_dir_ok;
    use crate::raw_rust_registry::StubRegistry;
    use std::path::PathBuf;

    fn examples_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("digimon-engine/cards/_examples")
    }

    fn run_example_suite_test(test: impl FnOnce() + Send + 'static) {
        let handle = std::thread::Builder::new()
            .name("digimon-dsl-example-suite".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(test)
            .expect("spawn example-suite test");
        if let Err(payload) = handle.join() {
            std::panic::resume_unwind(payload);
        }
    }

    #[test]
    fn registry_from_specs_compiles_all_examples() {
        run_example_suite_test(|| {
            let (specs, errs) = load_dir_ok(&examples_dir());
            assert!(errs.is_empty());
            assert!(!specs.is_empty(), "example fixtures must not be empty");
            let registry = CardRegistry::from_specs("phase1b-test", &specs).expect("compile");
            assert_eq!(registry.len(), specs.len());
            assert!(registry.lookup("BT17-015").is_some());
        });
    }

    #[test]
    fn registry_round_trips_through_bytes() {
        run_example_suite_test(|| {
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
        });
    }

    #[test]
    fn registry_indexed_pack_matches_eager_pack_bytes_and_cards() {
        run_example_suite_test(|| {
            let (specs, _) = load_dir_ok(&examples_dir());
            let registry = CardRegistry::from_specs("indexed-test", &specs).unwrap();

            let pack = CardPack {
                manifest: registry.manifest.clone(),
                cards: registry.iter().map(|(_, c)| c.clone()).collect(),
            };
            let expected_bytes = pack.to_bytes().unwrap();
            let (pack_bytes, index_bytes) = pack.to_bytes_with_index().unwrap();
            assert_eq!(
                pack_bytes, expected_bytes,
                "to_bytes_with_index must emit byte-identical cards.pack output"
            );

            let leaked: &'static [u8] = Box::leak(pack_bytes.into_boxed_slice());
            let lazy = CardRegistry::from_static_indexed_pack(leaked, &index_bytes).unwrap();
            assert_eq!(lazy.len(), registry.len());
            assert!(lazy.enrichments().is_some());
            for (card_id, card) in registry.iter() {
                assert!(lazy.contains(card_id));
                assert_eq!(lazy.lookup(card_id), Some(card));
            }
        });
    }

    #[test]
    fn registry_from_pack_file_round_trip() {
        run_example_suite_test(|| {
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
        });
    }

    #[test]
    fn registry_rejects_pack_with_missing_manifest_raw_rust_fn() {
        let pack = CardPack::new_with_required_raw_rust_fns(
            "raw-pack",
            vec![],
            ["known_fn", "missing_fn"],
        );
        let bytes = pack.to_bytes().unwrap();
        let registry = StubRegistry::with(["known_fn"]);

        let err = match CardRegistry::from_pack_bytes_with_raw_registry(&bytes, &registry) {
            Ok(_) => panic!("pack with missing raw_rust requirement should be rejected"),
            Err(err) => err,
        };
        assert!(err.contains("raw-pack"));
        assert!(err.contains("missing_fn"));
        assert!(!err.contains("known_fn"));
    }

    #[test]
    fn registry_accepts_pack_when_manifest_raw_rust_fns_are_registered() {
        let pack = CardPack::new_with_required_raw_rust_fns("raw-pack", vec![], ["known_fn"]);
        let bytes = pack.to_bytes().unwrap();
        let registry = StubRegistry::with(["known_fn"]);

        let loaded = CardRegistry::from_pack_bytes_with_raw_registry(&bytes, &registry).unwrap();
        assert_eq!(loaded.manifest.pack_id, "raw-pack");
    }
}
