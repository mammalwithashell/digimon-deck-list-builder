//! CardRegistry: card_id ↔ integer mapping for tensor encoding.
//!
//! **Parity with Python is critical.** The integer index is the source of truth
//! for:
//!   - Tensor positions (observation vector feeds into `nn.Embedding` on the GPU)
//!   - Pretrained embedding lookup (card_embeddings.npy is indexed by this value)
//!   - Cross-engine replay determinism (Python HeadlessGame vs Rust Game)
//!
//! When cards.json contains an explicit `index` field (the production format),
//! we use that directly — identical to Python's `CardRegistry.initialize()`.
//! When the field is absent (legacy array format, inline test fixtures), we
//! fall back to assigning indices in sorted-by-card_id order.
//!
//! # Invariants
//! - `0` is reserved for padding / unknown.
//! - Indices are dense 1..=N only in fallback mode. In production mode they
//!   may be sparse (gaps where cards were deprecated), so `index_to_id` is a
//!   `HashMap` not a `Vec`.

use std::collections::HashMap;

use crate::card_data::CardData;

/// Reserved for "no card" / padding.
pub const PADDING_ID: u16 = 0;

/// Total index space for `norm_id` fallback calculation (matches Python).
pub const REGISTRY_CAPACITY: u32 = 20_000;

pub struct CardRegistry {
    id_to_index: HashMap<String, u16>,
    index_to_id: HashMap<u16, String>,
    id_to_norm: HashMap<String, f32>,
}

impl CardRegistry {
    /// Build a registry from card data.
    ///
    /// Uses the `index` field on each `CardData` as the canonical mapping.
    /// If every card has `index == 0` (e.g. hand-built test fixtures), falls
    /// back to alphabetical assignment starting at 1.
    pub fn from_cards(cards: &HashMap<String, CardData>) -> Self {
        let all_unset = cards.values().all(|c| c.index == 0);
        let mut id_to_index: HashMap<String, u16> = HashMap::with_capacity(cards.len());
        let mut index_to_id: HashMap<u16, String> = HashMap::with_capacity(cards.len());
        let mut id_to_norm: HashMap<String, f32> = HashMap::with_capacity(cards.len());

        if all_unset {
            // Legacy / test mode: sort ids, assign sequential 1-based indices.
            let mut ids: Vec<&String> = cards.keys().collect();
            ids.sort();
            for (i, id) in ids.into_iter().enumerate() {
                let index = (i + 1) as u16;
                id_to_index.insert(id.clone(), index);
                index_to_id.insert(index, id.clone());
                id_to_norm.insert(id.clone(), index as f32 / REGISTRY_CAPACITY as f32);
            }
        } else {
            // Production mode: honor the index recorded in cards.json exactly.
            // Reject accidental duplicates — they'd collapse card identities in
            // the tensor and silently destroy training.
            for (id, data) in cards {
                if data.index == PADDING_ID {
                    // Skip — this card has no stable tensor slot. Lookups will
                    // return PADDING_ID (same as unknown), matching Python's
                    // `get_id` behavior for missing ids.
                    continue;
                }
                if let Some(existing) = index_to_id.get(&data.index) {
                    panic!(
                        "CardRegistry: duplicate index {} shared by {} and {} — \
                         cards.json is corrupt",
                        data.index, existing, id
                    );
                }
                id_to_index.insert(id.clone(), data.index);
                index_to_id.insert(data.index, id.clone());
                let norm = if data.norm_id > 0.0 {
                    data.norm_id
                } else {
                    data.index as f32 / REGISTRY_CAPACITY as f32
                };
                id_to_norm.insert(id.clone(), norm);
            }
        }

        Self {
            id_to_index,
            index_to_id,
            id_to_norm,
        }
    }

    /// Get the integer index for a card_id. Returns `PADDING_ID` (0) if unknown.
    pub fn get_index(&self, card_id: &str) -> u16 {
        if card_id.is_empty() {
            return PADDING_ID;
        }
        self.id_to_index.get(card_id).copied().unwrap_or(PADDING_ID)
    }

    /// Get the normalized ID (float in `[0, 1)`). Returns 0.0 if unknown.
    pub fn get_norm_id(&self, card_id: &str) -> f32 {
        if card_id.is_empty() {
            return 0.0;
        }
        self.id_to_norm.get(card_id).copied().unwrap_or(0.0)
    }

    /// Get the card_id for an integer index. Returns None for 0 or unknown.
    pub fn get_id(&self, index: u16) -> Option<&str> {
        if index == PADDING_ID {
            None
        } else {
            self.index_to_id.get(&index).map(|s| s.as_str())
        }
    }

    /// Total number of registered cards.
    pub fn count(&self) -> usize {
        self.id_to_index.len()
    }

    /// Whether this registry was built from explicit cards.json indices
    /// (the production case) rather than alphabetical fallback.
    pub fn has_explicit_indices(&self) -> bool {
        // Heuristic: if any index exceeds count(), we know indices are sparse,
        // which only happens in production mode. If indices are dense 1..=N it
        // could be either mode — but in dense case, both modes produce the
        // same mapping, so the distinction doesn't matter.
        self.id_to_index
            .values()
            .any(|&i| i as usize > self.id_to_index.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card_data::CardData;

    fn sample_cards_legacy() -> HashMap<String, CardData> {
        // Minimal JSON without `index`/`norm_id` fields → fallback mode.
        let json = r#"{
            "BT1-010": {
                "card_id": "BT1-010", "card_name_eng": "Agumon",
                "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            },
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": []
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    fn sample_cards_with_indices() -> HashMap<String, CardData> {
        // Production format — each card has an explicit index. Note that the
        // indices are *intentionally non-alphabetical* to catch regressions.
        let json = r#"{
            "BT1-010": {
                "card_id": "BT1-010", "card_name_eng": "Agumon",
                "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
                "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": [],
                "index": 42, "norm_id": 0.0021
            },
            "BT1-001": {
                "card_id": "BT1-001", "card_name_eng": "Koromon",
                "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
                "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": [],
                "index": 1337, "norm_id": 0.06685
            }
        }"#;
        CardData::load_from_str(json).unwrap()
    }

    #[test]
    fn fallback_mode_assigns_sorted_indices() {
        let cards = sample_cards_legacy();
        let reg = CardRegistry::from_cards(&cards);

        assert_eq!(reg.count(), 2);
        // BT1-001 sorts before BT1-010
        assert_eq!(reg.get_index("BT1-001"), 1);
        assert_eq!(reg.get_index("BT1-010"), 2);
        assert_eq!(reg.get_index("MISSING"), PADDING_ID);
    }

    #[test]
    fn fallback_mode_reverse_lookup() {
        let cards = sample_cards_legacy();
        let reg = CardRegistry::from_cards(&cards);

        assert_eq!(reg.get_id(1), Some("BT1-001"));
        assert_eq!(reg.get_id(2), Some("BT1-010"));
        assert_eq!(reg.get_id(0), None);
        assert_eq!(reg.get_id(999), None);
    }

    #[test]
    fn production_mode_uses_explicit_indices() {
        let cards = sample_cards_with_indices();
        let reg = CardRegistry::from_cards(&cards);

        // These match what Python's CardRegistry would return.
        assert_eq!(reg.get_index("BT1-010"), 42);
        assert_eq!(reg.get_index("BT1-001"), 1337);
        assert_eq!(reg.get_id(42), Some("BT1-010"));
        assert_eq!(reg.get_id(1337), Some("BT1-001"));
        // Sparse indices — nothing at position 2.
        assert_eq!(reg.get_id(2), None);
    }

    #[test]
    fn production_mode_preserves_norm_id() {
        let cards = sample_cards_with_indices();
        let reg = CardRegistry::from_cards(&cards);

        assert!((reg.get_norm_id("BT1-010") - 0.0021).abs() < 1e-6);
        assert!((reg.get_norm_id("BT1-001") - 0.06685).abs() < 1e-5);
        assert_eq!(reg.get_norm_id("MISSING"), 0.0);
    }

    #[test]
    fn fallback_mode_derives_norm_id_from_capacity() {
        let cards = sample_cards_legacy();
        let reg = CardRegistry::from_cards(&cards);

        let expected_001 = 1.0 / REGISTRY_CAPACITY as f32;
        let expected_010 = 2.0 / REGISTRY_CAPACITY as f32;
        assert!((reg.get_norm_id("BT1-001") - expected_001).abs() < 1e-9);
        assert!((reg.get_norm_id("BT1-010") - expected_010).abs() < 1e-9);
    }

    #[test]
    fn padding_id_returns_none() {
        let cards = sample_cards_with_indices();
        let reg = CardRegistry::from_cards(&cards);
        assert_eq!(reg.get_index(""), PADDING_ID);
        assert_eq!(reg.get_norm_id(""), 0.0);
    }

    #[test]
    #[should_panic(expected = "duplicate index")]
    fn duplicate_indices_panic() {
        // Two cards with the same `index` would silently alias in the tensor.
        // Refuse to build a registry with this kind of data.
        let json = r#"{
            "BT1-A": {
                "card_id": "BT1-A", "card_name_eng": "A",
                "card_effect_class_name": "", "play_cost": 0, "dp": -1,
                "level": 1, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": [],
                "index": 100
            },
            "BT1-B": {
                "card_id": "BT1-B", "card_name_eng": "B",
                "card_effect_class_name": "", "play_cost": 0, "dp": -1,
                "level": 1, "card_kind": 0, "rarity": 0, "card_colors": [0],
                "type_eng": [], "form_eng": [], "attribute_eng": [],
                "effect_description_eng": "", "inherited_effect_description_eng": "",
                "security_effect_description_eng": "", "evo_costs": [],
                "index": 100
            }
        }"#;
        let cards = CardData::load_from_str(json).unwrap();
        let _ = CardRegistry::from_cards(&cards);
    }
}
