//! Parity test: Rust CardRegistry must produce the same card_id → index
//! mapping as Python's CardRegistry.initialize() when loaded from the real
//! cards.json.
//!
//! This test guards against tensor-encoding drift. If it fails, anything
//! that serialized indices before (trained models, recorded replays,
//! pretrained embeddings) becomes silently wrong.

use std::path::PathBuf;

use digimon_engine::card_data::CardData;
use digimon_engine::card_registry::{CardRegistry, PADDING_ID};

/// Locate the real cards.json relative to the workspace root.
fn cards_json_path() -> Option<PathBuf> {
    // CARGO_MANIFEST_DIR = digimon-engine/
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest
        .parent()?
        .join("digimon_gym")
        .join("engine")
        .join("data")
        .join("cards.json");
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

#[test]
fn production_cards_json_roundtrips_through_registry() {
    let Some(path) = cards_json_path() else {
        eprintln!("Skipping: cards.json not found at expected path");
        return;
    };

    let cards = CardData::load_from_file(&path)
        .expect("failed to load cards.json");

    assert!(
        cards.len() > 100,
        "expected a real production cards.json with hundreds of cards, got {}",
        cards.len()
    );

    // Every card should have a non-zero index in production.
    let missing_index: Vec<_> = cards
        .values()
        .filter(|c| c.index == 0)
        .map(|c| c.card_id.clone())
        .collect();
    assert!(
        missing_index.is_empty(),
        "production cards.json should have `index` for every card; \
         missing on {} cards (e.g. {:?})",
        missing_index.len(),
        &missing_index[..missing_index.len().min(5)]
    );

    let reg = CardRegistry::from_cards(&cards);
    assert_eq!(reg.count(), cards.len());

    // Spot-check: every card round-trips.
    for (id, data) in &cards {
        assert_ne!(data.index, PADDING_ID, "{} has PADDING_ID", id);
        assert_eq!(reg.get_index(id), data.index, "lookup mismatch for {}", id);
        assert_eq!(
            reg.get_id(data.index),
            Some(id.as_str()),
            "reverse lookup mismatch for {} @ {}",
            id,
            data.index
        );
    }
}

#[test]
fn production_indices_are_unique() {
    let Some(path) = cards_json_path() else {
        return;
    };
    let cards = CardData::load_from_file(&path).unwrap();

    // Build will panic if duplicate indices are present — but also assert it
    // explicitly with a clearer message.
    let mut seen: std::collections::HashMap<u16, String> =
        std::collections::HashMap::new();
    for (id, data) in &cards {
        if let Some(prev) = seen.insert(data.index, id.clone()) {
            panic!(
                "duplicate index {} on {} and {} in cards.json",
                data.index, prev, id
            );
        }
    }

    // Sanity: building the registry doesn't panic.
    let _ = CardRegistry::from_cards(&cards);
}

#[test]
fn norm_id_is_index_over_capacity() {
    // Python stores norm_id = index / REGISTRY_CAPACITY (20_000). Verify that
    // invariant holds in the Rust reader for a few sampled cards.
    let Some(path) = cards_json_path() else {
        return;
    };
    let cards = CardData::load_from_file(&path).unwrap();

    for (id, data) in cards.iter().take(10) {
        if data.norm_id == 0.0 {
            continue; // cards.json may omit norm_id for some entries
        }
        let expected = data.index as f32 / 20_000.0;
        assert!(
            (data.norm_id - expected).abs() < 1e-3,
            "{}: stored norm_id {} differs from index/capacity {}",
            id,
            data.norm_id,
            expected
        );
    }
}
