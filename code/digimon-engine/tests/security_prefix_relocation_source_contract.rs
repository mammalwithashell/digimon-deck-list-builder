//! Section-label removal plus source-zone repair, not card resolution approval.
use digimon_engine::card_data::{CardData, parse_printed_keywords};

#[test]
fn literal_security_label_is_removed_once_and_body_moves_to_security() {
    for fixture in [
        include_str!("../../tests/fixtures/card_data/security_relocation/AD1-021.json"),
        include_str!("../../tests/fixtures/card_data/security_relocation/BT1-110.json"),
    ] {
        let metadata: serde_json::Value = serde_json::from_str(fixture).unwrap();
        let extracted = &metadata["original_observation"]["extracted"];
        let id = extracted["card_id"].as_str().unwrap();
        let section = extracted["text_sections"].as_array().unwrap().iter()
            .find(|row| row["label"] == "Security Effect").unwrap();
        let body = section["text"].as_str().unwrap();
        let old_text = format!("Security Effect {body}");
        let kind = match extracted["card_type"].as_str().unwrap() {
            "Tamer" => 1, "Option" => 2, _ => panic!("not a fixture in this family"),
        };
        let colors: Vec<usize> = extracted["colors"].as_array().unwrap().iter()
            .map(|color| ["Red", "Blue", "Yellow", "Green", "White", "Black", "Purple"]
                .iter().position(|candidate| *candidate == color.as_str().unwrap()).unwrap()).collect();
        let record = |inherited: &str, security: &str| serde_json::json!({id: {
            "card_id": id, "card_name_eng": extracted["name"], "card_kind": kind,
            "card_colors": colors, "play_cost": extracted["play_cost"],
            "card_effect_class_name": "", "inherited_effect_description_eng": inherited,
            "security_effect_description_eng": security
        }}).to_string();
        let old = CardData::load_from_str(&record(&old_text, "")).unwrap().remove(id).unwrap();
        let new = CardData::load_from_str(&record("", body)).unwrap().remove(id).unwrap();
        assert_eq!(old.inherited_text, old_text);
        assert!(!old.inherited_text.trim().is_empty());
        assert!(new.inherited_text.is_empty());
        assert_eq!(new.security_text, body);
        assert!(old.text_for_search_all_faces().contains(&old_text));
        assert!(new.text_for_search_all_faces().contains(body));
        assert!(!new.security_text.starts_with("Security Effect "));
        assert!(parse_printed_keywords("", &new.inherited_text, "").is_empty());
        assert!(new.keywords.is_empty());
    }
}