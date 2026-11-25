//! Diagnostic section contract, not registration or behavior admission for a real card.
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};

fn loaded(field: &str) -> CardData {
    let mut row = serde_json::json!({
        "card_id": "SECTION_PROBE", "card_name_eng": "Section probe",
        "card_kind": 0, "card_colors": [0], "level": 3, "dp": 2000,
        "effect_description_eng": "", "inherited_effect_description_eng": "",
        "security_effect_description_eng": ""
    });
    row[field] = "＜Blocker＞".into();
    CardData::load_from_str(&serde_json::json!({"SECTION_PROBE": row}).to_string())
        .unwrap().remove("SECTION_PROBE").unwrap()
}

#[test]
fn identical_searchable_text_has_different_face_and_source_keyword_behavior() {
    for (field, face, inherited) in [
        ("effect_description_eng", true, false),
        ("inherited_effect_description_eng", false, true),
        ("security_effect_description_eng", false, false),
    ] {
        let probe = loaded(field);
        assert!(probe.in_text_contains("Blocker"));
        let mut top = make_test_card("PLAIN_TOP", "Plain top");
        top.card_kind = CardKind::Digimon;
        top.dp = Some(5000);
        let mut runner = DebugRunner::builder().add_card(probe).add_card(top).build();
        let source_stack = runner.place_stack(0, &["SECTION_PROBE", "PLAIN_TOP"]);
        let probe_face = runner.place_on_field(0, "SECTION_PROBE", None);
        assert_eq!(runner.game.has_keyword(source_stack, Keyword::Blocker), inherited, "{field}: source");
        assert_eq!(runner.game.has_keyword(probe_face, Keyword::Blocker), face, "{field}: face");
    }
}

#[test]
fn an_unrelated_inherited_printing_cannot_supply_a_stack_keyword() {
    let inherited = loaded("inherited_effect_description_eng");
    let mut plain = make_test_card("PLAIN", "Plain");
    plain.card_kind = CardKind::Digimon;
    plain.dp = Some(5000);
    let mut runner = DebugRunner::builder().add_card(inherited).add_card(plain).build();
    let unrelated = runner.place_stack(1, &["SECTION_PROBE", "PLAIN"]);
    let tested = runner.place_stack(0, &["PLAIN", "PLAIN"]);
    assert!(runner.game.has_keyword(unrelated, Keyword::Blocker));
    assert!(!runner.game.has_keyword(tested, Keyword::Blocker));
}
