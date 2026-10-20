//! Source spelling comparisons only, not behavior or old-verdict equivalence.
use digimon_engine::card_data::{CardData, parse_printed_keywords};
#[test]
fn actual_nbsp_rows_preserve_literal_tokens_and_printed_keywords() {
    let rows: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/card_data/ordinary_nbsp/consumer-cases.json")).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 4);
    for row in rows.as_array().unwrap() {
        let old = row["old_value"].as_str().unwrap();
        let new = row["new_value"].as_str().unwrap();
        assert_eq!(old.replace('\u{a0}', " "), new);
        assert_eq!(parse_printed_keywords(old, "", ""), parse_printed_keywords(new, "", ""));
        let id = row["card_id"].as_str().unwrap();
        let field = row["field_path"].as_str().unwrap();
        let load = |text: &str| {
            let mut record = serde_json::json!({"card_id": id, "card_name_eng": "Source consumer fixture", "card_kind": 0, "card_colors": [0], "card_effect_class_name": ""});
            record[field] = serde_json::Value::String(text.to_owned());
            CardData::load_from_str(&serde_json::json!({id: record}).to_string()).unwrap().remove(id).unwrap()
        };
        let a = load(old); let b = load(new);
        assert_eq!(a.keywords, b.keywords);
        assert!(a.text_for_search_all_faces().contains(old));
        assert!(b.text_for_search_all_faces().contains(new));
        assert!(!a.text_for_search_all_faces().contains(new));
        for token in old.split('[').skip(1).filter_map(|s| s.split_once(']').map(|x| x.0)) {
            assert!(!token.contains('\u{a0}'));
            assert!(new.contains(&format!("[{token}]")));
        }
    }
}
#[test]
fn whitespace_around_keywords_preserves_the_existing_parser_result() {
    for old in ["＜Blocker＞\u{a0}＜Rush＞", "[Main]\u{a0}＜Digi-Burst 2＞", "[On Deletion] ＜Save＞.\u{a0}Then, draw 1."] {
        assert_eq!(parse_printed_keywords(old, "", ""), parse_printed_keywords(&old.replace('\u{a0}', " "), "", ""));
    }
}
