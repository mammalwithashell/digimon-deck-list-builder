//! `distinct_colors_gte` (G-DSL-DISTINCT-FILTERED-COLORS) must survive
//! authored -> compiled -> JSON round-trip and appear in the schema.
#[test]
fn distinct_colors_gte_compiles_and_roundtrips() {
    let yaml = r#"
card: DISTINCT-COLORS-SCHEMA
name: Distinct Colors Schema
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    condition:
      distinct_colors_gte:
        of: you
        filter:
          all_of:
            - kind: tamer
            - trait_has: ADVENTURE
        n: 3
    process:
      - gain_memory: 1
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let json = serde_json::to_value(&compiled).unwrap();
    let leaf = &json["effects"][0]["Triggered"]["condition"]["distinct_colors_gte"];
    assert_eq!(leaf["n"], 3);
    assert!(!leaf["filter"].is_null());
    let decoded: digimon_dsl::compiled::CompiledCard = serde_json::from_value(json).unwrap();
    assert_eq!(compiled, decoded);
    let schema = serde_json::to_string(&schemars::schema_for!(digimon_dsl::CardSpec)).unwrap();
    assert!(schema.contains("distinct_colors_gte"));
}

#[test]
fn distinct_colors_gte_rejects_unknown_fields() {
    let yaml = r#"
card: DISTINCT-COLORS-BAD
name: Bad
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    condition:
      distinct_colors_gte: { of: you, filter: { kind: tamer }, n: 3, zone: hand }
    process:
      - gain_memory: 1
"#;
    assert!(serde_yml::from_str::<digimon_dsl::CardSpec>(yaml).is_err());
}
