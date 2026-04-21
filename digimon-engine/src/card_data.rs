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

/// One half of a DNA digivolve cost — a constraint that one of the two
/// sacrificed materials must satisfy. Mirror of Python's `DnaRequirement`
/// in `digimon_gym/engine/data/evo_cost.py`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnaRequirement {
    /// Required level. 0 means "any level" (name/text-only requirement).
    #[serde(default)]
    pub level: u8,
    /// Optional color constraint. `None` means any color.
    #[serde(default)]
    pub card_color: Option<CardColor>,
    /// Case-insensitive substring against `card_name`. Empty = no constraint.
    #[serde(default)]
    pub name_contains: String,
    /// Case-insensitive substring against `effect_text`. Empty = no constraint.
    #[serde(default)]
    pub text_contains: String,
}

/// A DNA digivolve cost entry. Two requirements plus a memory cost.
/// Mirror of Python's `DnaCost` in `digimon_gym/engine/data/evo_cost.py`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DnaCost {
    pub requirement1: DnaRequirement,
    pub requirement2: DnaRequirement,
    #[serde(default)]
    pub memory_cost: i16,
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
    /// DNA digivolve costs. Empty for cards without a DNA variant.
    /// Currently defaults to empty for every card loaded from cards.json —
    /// the JSON export pipeline doesn't yet emit this field (§4.5b).
    #[serde(default)]
    pub dna_costs: Vec<DnaCost>,
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
    /// Keywords printed on the card's face (Rush, Jamming, Blocker, etc.),
    /// parsed from effect_text / inherited_text / security_text at load
    /// time via parse_printed_keywords. Distinct from modifier-granted
    /// keywords managed by ModifierRegistry. The unified Game::has_keyword
    /// query considers both sources.
    #[serde(default)]
    pub keywords: Vec<crate::enums::Keyword>,
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
    /// DNA digivolve costs — absent in cards.json today; deserializes to
    /// empty. When the Python export pipeline starts emitting this field
    /// (§4.5b), the structure here is already the deserialization target.
    #[serde(default)]
    dna_costs: Vec<DnaCost>,
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

            // Parse keywords before moving the text fields into CardData.
            let keywords = parse_printed_keywords(
                &raw_card.effect_description_eng,
                &raw_card.inherited_effect_description_eng,
                &raw_card.security_effect_description_eng,
            );

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
                dna_costs: raw_card.dna_costs,
                effect_text: raw_card.effect_description_eng,
                inherited_text: raw_card.inherited_effect_description_eng,
                security_text: raw_card.security_effect_description_eng,
                keywords,
                effect_class_name: raw_card.card_effect_class_name,
                index: raw_card.index,
                norm_id: raw_card.norm_id,
            };
            cards.insert(id, card);
        }
        Ok(cards)
    }
}

/// Extract printed keywords from a card's text fields.
///
/// Keywords appear in card text as `＜Keyword＞` (full-width angle
/// brackets) optionally followed by a parenthetical English
/// description. Parametric keywords (`Security A. +N`,
/// `De-Digivolve N`, `Draw N`) are parsed into their typed variants.
///
/// Returns a deduplicated Vec in document order across the three
/// input fields.
pub fn parse_printed_keywords(
    effect_text: &str,
    inherited_text: &str,
    security_text: &str,
) -> Vec<crate::enums::Keyword> {
    use crate::enums::Keyword;

    let mut found: Vec<Keyword> = Vec::new();

    let push_unique = |k: Keyword, found: &mut Vec<Keyword>| {
        if !found.contains(&k) {
            found.push(k);
        }
    };

    for text in [effect_text, inherited_text, security_text] {
        let mut remaining = text;
        while let Some(start) = remaining.find('＜') {
            let after_open = &remaining[start + '＜'.len_utf8()..];
            let Some(end) = after_open.find('＞') else { break };
            let inside = &after_open[..end];
            remaining = &after_open[end + '＞'.len_utf8()..];

            let trimmed = inside.trim();

            // Try non-parametric keywords first (longest-prefix wins).
            // Order matters: "Blast Digivolve" before "Blast", "Armor Purge"
            // before "Armor", "Decode" before "Decoy" — we check each prefix
            // in order and break on the first match.
            let mut matched = false;
            for (prefix, kw) in [
                ("Blast Digivolve", Keyword::Blast),
                ("Blocker", Keyword::Blocker),
                ("Rush", Keyword::Rush),
                ("Jamming", Keyword::Jamming),
                ("Piercing", Keyword::Piercing),
                ("Reboot", Keyword::Reboot),
                ("Blitz", Keyword::Blitz),
                ("Armor Purge", Keyword::ArmorPurge),
                ("Armor", Keyword::Armor),
                ("Raid", Keyword::Raid),
                ("Alliance", Keyword::Alliance),
                ("Save", Keyword::Save),
                ("Fortitude", Keyword::Fortitude),
                ("Overclock", Keyword::Overclock),
                ("Barrier", Keyword::Barrier),
                ("Evade", Keyword::Evade),
                ("Decode", Keyword::Decode),
                ("Decoy", Keyword::Decoy),
                ("Material", Keyword::Material),
                ("Partition", Keyword::Partition),
                ("Vortex", Keyword::Vortex),
                ("Collision", Keyword::Collision),
            ] {
                if trimmed.starts_with(prefix) {
                    push_unique(kw, &mut found);
                    matched = true;
                    break;
                }
            }

            if matched {
                continue;
            }

            // Parametric: Security A. +N / -N
            if let Some(rest) = trimmed.strip_prefix("Security A.") {
                let rest = rest.trim();
                let (sign, digits) = if let Some(d) = rest.strip_prefix('+') {
                    (1i8, d)
                } else if let Some(d) = rest.strip_prefix('-') {
                    (-1i8, d)
                } else {
                    (1i8, rest)
                };
                let n_str = digits.trim().split_whitespace().next().unwrap_or("");
                if let Ok(n) = n_str.parse::<u8>() {
                    if sign < 0 {
                        push_unique(Keyword::SecurityAttackMinus(n as i8), &mut found);
                    } else {
                        push_unique(Keyword::SecurityAttackPlus(n as i8), &mut found);
                    }
                }
                continue;
            }

            // Parametric: De-Digivolve N
            if let Some(rest) = trimmed.strip_prefix("De-Digivolve") {
                let n_str = rest.trim().split_whitespace().next().unwrap_or("");
                if let Ok(n) = n_str.parse::<u8>() {
                    push_unique(Keyword::DeDigivolve(n), &mut found);
                }
                continue;
            }

            // Parametric: Draw N
            if let Some(rest) = trimmed.strip_prefix("Draw") {
                let n_str = rest.trim().split_whitespace().next().unwrap_or("");
                if let Ok(n) = n_str.parse::<u8>() {
                    push_unique(Keyword::DrawX(n), &mut found);
                }
                continue;
            }

            // Parametric: Fragment (N) — printed form is ＜Fragment (3)＞.
            // Also accept `Fragment N` as a conservative fallback in case
            // some printings omit the parens.
            if let Some(rest) = trimmed.strip_prefix("Fragment") {
                let rest = rest.trim();
                // Strip optional surrounding parens/brackets around the number.
                let inner = rest
                    .trim_start_matches('(')
                    .trim_start_matches('[')
                    .trim_end_matches(')')
                    .trim_end_matches(']')
                    .trim();
                let n_str = inner.split_whitespace().next().unwrap_or("");
                // Trailing `)` could remain if the token is `3)` — strip it.
                let n_str = n_str.trim_end_matches(')').trim_end_matches(']');
                if let Ok(n) = n_str.parse::<u8>() {
                    push_unique(Keyword::Fragment(n), &mut found);
                }
                continue;
            }
        }
    }

    found
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
