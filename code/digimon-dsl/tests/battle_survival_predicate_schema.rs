//! Survival-qualified battle identity must survive authored/compiled serialization.
#[test]
fn surviving_deleter_predicate_compiles_and_roundtrips() {
    let yaml = r#"
card: BATTLE-SURVIVAL-SCHEMA
name: Battle Survival Schema
kind: digimon
level: 6
color: [green]
cost: 12
dp: 12000
effects:
  - when: on_ally_won_battle
    condition:
      event_battle_surviving_deleter:
        owner: you
        name_contains: Dramon
    process:
      - trash_top_security: {of: opponent}
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let json = serde_json::to_value(&compiled).unwrap();
    assert_eq!(json["effects"][0]["Triggered"]["condition"]["event_battle_surviving_deleter"]["name_contains"], "Dramon");
    let decoded: digimon_dsl::compiled::CompiledCard = serde_json::from_value(json).unwrap();
    assert_eq!(compiled, decoded);
    let schema = serde_json::to_string(&schemars::schema_for!(digimon_dsl::CardSpec)).unwrap();
    assert!(schema.contains("event_battle_surviving_deleter"));
}
