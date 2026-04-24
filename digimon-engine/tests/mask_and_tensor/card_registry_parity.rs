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
fn production_cards_json_populates_dna_costs() {
    // Guards the data-ingest pipeline in tools/ingest_cards.py: every card
    // with a DNA Digivolve clause in `xros_req` must have a structured
    // `dna_costs` entry that the Rust deserializer picks up. Before this
    // pipeline landed, the field was always `[]` and the DNA-digivolve
    // mask branch (see RUST_PYTHON_PARITY §4.5b) could not fire.
    let Some(path) = cards_json_path() else {
        return;
    };
    let cards = CardData::load_from_file(&path).unwrap();

    let with_dna: Vec<_> = cards
        .values()
        .filter(|c| !c.dna_costs.is_empty())
        .collect();
    assert!(
        with_dna.len() >= 50,
        "expected >=50 cards with structured dna_costs in cards.json, got {}. \
         Run `python -m tools.ingest_cards --backfill` to regenerate.",
        with_dna.len()
    );

    // Every DNA entry must have two well-formed requirements.
    for c in &with_dna {
        for (i, dc) in c.dna_costs.iter().enumerate() {
            // `level` is u8 so it's automatically >= 0; require at least one
            // of {level, name_contains, text_contains} to be set on each
            // requirement, mirroring the Python parser's "no valid level"
            // rejection path.
            let r1_set = dc.requirement1.level > 0
                || !dc.requirement1.name_contains.is_empty()
                || !dc.requirement1.text_contains.is_empty();
            let r2_set = dc.requirement2.level > 0
                || !dc.requirement2.name_contains.is_empty()
                || !dc.requirement2.text_contains.is_empty();
            assert!(
                r1_set && r2_set,
                "{} dna_costs[{}] has an empty requirement: {:?}",
                c.card_id, i, dc,
            );
        }
    }
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
