use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::enums::{CardColor, CardKind};

/// Digivolution cost entry from cards.json.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvoCost {
    pub card_color: u8,
    pub level: u8,
    pub memory_cost: u16,
}

/// Static card metadata loaded from cards.json.
/// One instance per unique card_id, shared across all game instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardData {
    pub card_id: String,
    pub card_name: String,
    pub card_kind: CardKind,
    pub level: Option<u8>,
    pub dp: Option<i32>,
    pub play_cost: u16,
    pub colors: Vec<CardColor>,
    pub traits: Vec<String>,
    pub evo_costs: Vec<EvoCost>,
    pub effect_text: String,
    pub inherited_text: String,
    pub security_text: String,
    pub effect_class_name: String,
    /// Stable integer index from cards.json (1-based; 0 = unset/legacy).
    /// This is the source of truth for tensor encoding — must match Python.
    /// See docs/RUST_ENGINE_API.md §11 and card_registry.rs.
    #[serde(default)]
    pub index: u16,
    /// Normalized ID (index / CAPACITY) from cards.json. 0.0 if unset.
    #[serde(default)]
    pub norm_id: f32,
}

/// Raw JSON shape from cards.json — matches the actual file format.
///
/// `dp`, `level`, and `play_cost` accept both `null` (production) and `-1`
/// (legacy / some Python tooling) for "not applicable".
#[derive(Debug, Deserialize)]
struct RawCard {
    card_id: String,
    card_name_eng: String,
    #[serde(default)]
    card_effect_class_name: String,
    #[serde(default)]
    play_cost: Option<i32>,
    #[serde(default)]
    dp: Option<i32>,
    #[serde(default)]
    level: Option<i32>,
    card_kind: u8,
    card_colors: Vec<u8>,
    #[serde(default)]
    type_eng: Vec<String>,
    #[serde(default)]
    form_eng: Vec<String>,
    #[serde(default)]
    attribute_eng: Vec<String>,
    #[serde(default)]
    effect_description_eng: String,
    #[serde(default)]
    inherited_effect_description_eng: String,
    #[serde(default)]
    security_effect_description_eng: String,
    #[serde(default)]
    evo_costs: Vec<RawEvoCost>,
    /// Stable tensor-encoding index. Present in the current cards.json dict
    /// format; absent in legacy array format or minimal test fixtures.
    #[serde(default)]
    index: u16,
    /// Pre-computed index / REGISTRY_CAPACITY. Absent in legacy format.
    #[serde(default)]
    norm_id: f32,
}

#[derive(Debug, Deserialize)]
struct RawEvoCost {
    card_color: u8,
    level: u8,
    memory_cost: u16,
}

fn parse_card_kind(raw: u8) -> CardKind {
    match raw {
        0 => CardKind::Digimon,
        1 => CardKind::Tamer,
        2 => CardKind::Option,
        3 => CardKind::DigiEgg,
        _ => CardKind::Digimon, // fallback
    }
}

fn parse_card_color(raw: u8) -> CardColor {
    match raw {
        0 => CardColor::Red,
        1 => CardColor::Blue,
        2 => CardColor::Yellow,
        3 => CardColor::Green,
        4 => CardColor::Black,
        5 => CardColor::Purple,
        6 => CardColor::White,
        _ => CardColor::Red, // fallback
    }
}

impl CardData {
    /// Load all card data from a cards.json file.
    pub fn load_from_file(path: &Path) -> Result<HashMap<String, CardData>, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
        Self::load_from_str(&contents)
    }

    /// Load all card data from a JSON string.
    pub fn load_from_str(json_str: &str) -> Result<HashMap<String, CardData>, String> {
        let raw: HashMap<String, RawCard> = serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse cards.json: {}", e))?;

        let mut cards = HashMap::with_capacity(raw.len());
        for (id, raw_card) in raw {
            // Combine type, form, attribute into traits
            let mut traits = Vec::new();
            traits.extend(raw_card.form_eng);
            traits.extend(raw_card.attribute_eng);
            traits.extend(raw_card.type_eng);

            let card = CardData {
                card_id: raw_card.card_id.clone(),
                card_name: raw_card.card_name_eng,
                card_kind: parse_card_kind(raw_card.card_kind),
                level: match raw_card.level {
                    Some(l) if l >= 0 => Some(l as u8),
                    _ => None,
                },
                dp: match raw_card.dp {
                    Some(d) if d >= 0 => Some(d),
                    _ => None,
                },
                play_cost: raw_card.play_cost.unwrap_or(0).max(0) as u16,
                colors: raw_card.card_colors.iter().map(|&c| parse_card_color(c)).collect(),
                traits,
                evo_costs: raw_card
                    .evo_costs
                    .into_iter()
                    .map(|e| EvoCost {
                        card_color: e.card_color,
                        level: e.level,
                        memory_cost: e.memory_cost,
                    })
                    .collect(),
                effect_text: raw_card.effect_description_eng,
                inherited_text: raw_card.inherited_effect_description_eng,
                security_text: raw_card.security_effect_description_eng,
                effect_class_name: raw_card.card_effect_class_name,
                index: raw_card.index,
                norm_id: raw_card.norm_id,
            };
            cards.insert(id, card);
        }
        Ok(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_card() {
        let json = r#"{
            "BT1-001": {
                "index": 1,
                "norm_id": 0.00005,
                "card_id": "BT1-001",
                "card_index": 1,
                "card_name_eng": "Koromon",
                "card_name_jpn": "",
                "card_effect_class_name": "BT1_001",
                "play_cost": 0,
                "dp": -1,
                "level": 2,
                "card_kind": 3,
                "rarity": 0,
                "card_colors": [0],
                "type_eng": ["Lesser"],
                "form_eng": ["In-Training"],
                "attribute_eng": [],
                "effect_description_eng": "",
                "inherited_effect_description_eng": "",
                "security_effect_description_eng": "",
                "evo_costs": []
            }
        }"#;

        let cards = CardData::load_from_str(json).unwrap();
        assert_eq!(cards.len(), 1);

        let card = &cards["BT1-001"];
        assert_eq!(card.card_name, "Koromon");
        assert_eq!(card.card_kind, CardKind::DigiEgg);
        assert_eq!(card.level, Some(2));
        assert_eq!(card.dp, None); // -1 means no DP
        assert_eq!(card.colors, vec![CardColor::Red]);
        assert!(card.traits.contains(&"In-Training".to_string()));
        assert!(card.traits.contains(&"Lesser".to_string()));
    }
}
