//! The search CLI has the same two admission layers as the Python trainer.
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use digimon_engine::card_data::CardData;
use serde_json::Value;

pub fn validate_cards(
    mechanics: &Value,
    admission: &Value,
    decks: &[Vec<String>],
    registered: &HashSet<String>,
    available: &HashSet<String>,
) -> Result<(), String> {
    if mechanics.get("ready").and_then(Value::as_bool) != Some(true) {
        return Err(format!(
            "mechanics training gate is closed: {}",
            mechanics["blocking"]
        ));
    }
    let cards = admission
        .get("cards")
        .and_then(Value::as_object)
        .ok_or("card_admissibility.json has no cards object")?;
    for (index, deck) in decks.iter().enumerate() {
        for cid in deck.iter().collect::<BTreeSet<_>>() {
            if !registered.contains(cid) || !available.contains(cid) {
                return Err(format!(
                    "deck {index}: {cid} is not playable in this compiled engine/card database"
                ));
            }
            if cards
                .get(cid)
                .and_then(|r| r.get("admissible"))
                .and_then(Value::as_bool)
                != Some(true)
            {
                return Err(format!(
                    "deck {index}: {cid} is not admissible ({})",
                    cards
                        .get(cid)
                        .and_then(|r| r.get("reason"))
                        .unwrap_or(&Value::Null)
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_pool(
    decks: &[Vec<String>],
    card_data: &HashMap<String, CardData>,
    data_dir: &Path,
) -> Result<Value, String> {
    let read = |name: &str| -> Result<Value, String> {
        let path = data_dir.join(name);
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("admission artifact {}: {e}", path.display()))?;
        serde_json::from_str(&raw)
            .map_err(|e| format!("admission artifact {}: {e}", path.display()))
    };
    let mechanics = read("mechanics_readiness.json")?;
    let admission = read("card_admissibility.json")?;
    let registered: HashSet<_> = digimon_engine::cards::build_registry_cached()
        .registered_card_ids()
        .into_iter()
        .collect();
    let available: HashSet<_> = card_data.keys().cloned().collect();
    validate_cards(&mechanics, &admission, decks, &registered, &available)?;
    for (index, deck) in decks.iter().enumerate() {
        let result = digimon_engine::deck_tools::validate_deck(deck);
        if !result.is_valid {
            return Err(format!(
                "deck {index} is illegal for standard English rules: {}",
                result.errors.join("; ")
            ));
        }
    }
    Ok(
        serde_json::json!({"mechanics_ready": true, "card_admissibility_enforced": true,
        "compiled_registry_checked": true, "production_data_checked": true,
        "standard_deck_legality_checked": true, "deck_count": decks.len()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gate_cannot_admit_blocked_missing_unregistered_or_data_missing_cards() {
        let ready = json!({"ready":true});
        let cards = json!({"cards":{"A":{"admissible":true},"B":{"admissible":false,"reason":"failed_verdict"}}});
        let ids: HashSet<_> = ["A".into(), "B".into()].into_iter().collect();
        let deck = vec![vec!["A".into()]];
        assert!(validate_cards(&ready, &cards, &deck, &ids, &ids).is_ok());
        assert!(
            validate_cards(&json!({"ready":false}), &cards, &deck, &ids, &ids)
                .unwrap_err()
                .contains("mechanics")
        );
        assert!(validate_cards(&json!({}), &cards, &deck, &ids, &ids).is_err());
        assert!(
            validate_cards(&ready, &cards, &[vec!["B".into()]], &ids, &ids)
                .unwrap_err()
                .contains("failed_verdict")
        );
        assert!(validate_cards(&ready, &json!({"cards":{}}), &deck, &ids, &ids).is_err());
        assert!(validate_cards(&ready, &cards, &deck, &HashSet::new(), &ids).is_err());
        assert!(validate_cards(&ready, &cards, &deck, &ids, &HashSet::new()).is_err());
    }
}
