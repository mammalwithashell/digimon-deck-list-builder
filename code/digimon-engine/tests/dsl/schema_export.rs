use digimon_engine::dsl::schema::export_json_schema;

#[test]
fn schema_export_is_valid_json() {
    let s = export_json_schema();
    let v: serde_json::Value = serde_json::from_str(&s).expect("schema should be JSON");
    assert!(v.is_object());
    assert!(v.get("$schema").is_some(), "schema should declare $schema");
    assert!(v.get("title").is_some());
}

#[test]
fn schema_export_is_deterministic() {
    let a = export_json_schema();
    let b = export_json_schema();
    assert_eq!(a, b, "schema export must be deterministic");
}

#[test]
fn schema_export_mentions_top_level_card_spec_fields() {
    let s = export_json_schema();
    assert!(s.contains("\"card\""));
    assert!(s.contains("\"name\""));
    assert!(s.contains("\"kind\""));
    assert!(s.contains("\"effects\""));
}
