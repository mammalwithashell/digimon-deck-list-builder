//! Diagnostic controls: serialized JSON equivalence is narrower than text normalization.
//! These fixtures are not card registrations or training-admission evidence.
use digimon_engine::card_data::CardData;

fn loaded(field: &str, text: &str) -> CardData {
    let mut row = serde_json::json!({
        "card_id": "TEXT_PROBE", "card_name_eng": "Plain",
        "card_kind": 0, "card_colors": [0]
    });
    row[field] = text.into();
    CardData::load_from_str(&serde_json::json!({"TEXT_PROBE": row}).to_string())
        .unwrap().remove("TEXT_PROBE").unwrap()
}

#[test]
fn json_escape_spelling_preserves_decoded_runtime_text() {
    let escaped = r#"{"TEXT_PROBE":{"card_id":"TEXT_PROBE","card_name_eng":"Plain","card_kind":0,"card_colors":[0],"effect_description_eng":"Alpha\u0020Beta\nGamma"}}"#;
    let card = CardData::load_from_str(escaped).unwrap().remove("TEXT_PROBE").unwrap();
    let literal = loaded("effect_description_eng", "Alpha Beta\nGamma");
    assert_eq!(card.effect_text, literal.effect_text);
    for needle in ["Alpha Beta", "Beta\nGamma", "missing"] {
        assert_eq!(card.in_text_contains(needle), literal.in_text_contains(needle));
    }
    assert!(card.in_text_contains("Beta\nGamma"));
    assert!(!card.in_text_contains("missing"));
}

#[test]
fn newline_conversion_is_observable_in_every_searchable_text_zone() {
    for field in ["effect_description_eng", "inherited_effect_description_eng", "security_effect_description_eng", "xros_req"] {
        let lf = loaded(field, "Alpha\nBeta");
        let crlf = loaded(field, "Alpha\r\nBeta");
        assert!(lf.in_text_contains("Alpha\nBeta"), "{field}");
        assert!(!crlf.in_text_contains("Alpha\nBeta"), "{field}");
        assert!(crlf.in_text_contains("Alpha\r\nBeta"), "{field}");
        assert!(!lf.in_text_contains("Alpha\r\nBeta"), "{field}");
        assert!(lf.in_text_contains("Beta") && crlf.in_text_contains("Beta"));
    }
}

#[test]
fn trailing_newline_is_observable_even_when_common_keyword_queries_match() {
    let mut plain = loaded("effect_description_eng", "Alpha");
    let mut terminated = loaded("effect_description_eng", "Alpha\n");
    plain.inherited_text = "Beta".into();
    terminated.inherited_text = "Beta".into();
    // Whole-card search joins zones with newline; test that real boundary.
    assert!(plain.in_text_contains("Alpha") && terminated.in_text_contains("Alpha"));
    assert!(plain.in_text_contains("Alpha\nBeta"));
    assert!(!terminated.in_text_contains("Alpha\nBeta"));
    assert!(!plain.in_text_contains("Alpha\n\nBeta"));
    assert!(terminated.in_text_contains("Alpha\n\nBeta"));
}

#[test]
fn nbsp_and_repeated_spaces_are_not_runtime_equivalence() {
    let plain = loaded("effect_description_eng", "Alpha Beta");
    for changed in ["Alpha\u{a0}Beta", "Alpha  Beta"] {
        let card = loaded("effect_description_eng", changed);
        assert!(plain.in_text_contains("Alpha Beta"));
        assert!(!card.in_text_contains("Alpha Beta"));
        assert!(card.in_text_contains(changed));
        assert!(!plain.in_text_contains(changed));
    }
}
