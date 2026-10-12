//! Source-zone contract only, not a card behavior or training-readiness test.
use digimon_engine::card_data::{CardData, parse_printed_keywords};

#[test]
fn exact_security_relocation_preserves_sentence_and_removes_inherited_presence() {
    let sentence = "[Security] Play this card without paying the cost.";
    for (id, name) in [("AD1-019", "Matt Ishida & T.K. Takaishi"), ("AD1-022", "Izzy Izumi & Tai Kamiya")] {
        let record = |inherited: &str, security: &str| serde_json::json!({id: {
            "card_id": id, "card_name_eng": name, "card_kind": 1, "card_colors": [1,2], "play_cost": 3,
            "card_effect_class_name": "", "inherited_effect_description_eng": inherited,
            "security_effect_description_eng": security
        }}).to_string();
        let old = CardData::load_from_str(&record(sentence, "")).unwrap().remove(id).unwrap();
        let new = CardData::load_from_str(&record("", sentence)).unwrap().remove(id).unwrap();
        assert!(!old.inherited_text.trim().is_empty());
        assert!(new.inherited_text.trim().is_empty());
        assert_eq!(new.security_text, sentence);
        assert!(old.text_for_search_all_faces().contains(sentence));
        assert!(new.text_for_search_all_faces().contains(sentence));
        assert!(parse_printed_keywords("", &new.inherited_text, "").is_empty());
        assert!(new.keywords.is_empty());
    }
}
