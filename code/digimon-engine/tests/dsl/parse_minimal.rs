use digimon_engine::dsl::spec::{CardKind, CardSpec, ColorSpec};

#[test]
fn parse_vanilla_digimon() {
    let yaml = r#"
card: BT1-010
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.card, "BT1-010");
    assert_eq!(spec.name, "Agumon");
    assert_eq!(spec.kind, CardKind::Digimon);
    assert_eq!(spec.level, Some(3));
    assert_eq!(spec.color, vec![ColorSpec::Red]);
    assert_eq!(spec.cost, Some(3));
    assert_eq!(spec.dp, Some(2000));
    assert_eq!(spec.traits, vec!["Reptile".to_string()]);
    assert!(spec.effects.is_empty());
    assert!(spec.alt_paths.is_empty());
}

#[test]
fn parse_minimal_option() {
    let yaml = r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.kind, CardKind::Option);
    assert_eq!(spec.level, None);
    assert_eq!(spec.dp, None);
    assert_eq!(spec.card, "ST2-13");
    assert_eq!(spec.name, "Hammer Spark");
    assert_eq!(spec.color, vec![ColorSpec::Red]);
    assert_eq!(spec.cost, Some(0));
}

#[test]
fn rejects_unknown_top_level_field() {
    let yaml = r#"
card: X-1
name: Test
kind: option
color: [red]
cost: 0
bogus_field: true
"#;
    let result: Result<digimon_engine::dsl::spec::CardSpec, _> = serde_yml::from_str(yaml);
    assert!(
        result.is_err(),
        "CardSpec must reject unknown top-level fields"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("bogus_field") || err_msg.contains("unknown field"),
        "error should mention the unknown field, got: {err_msg}"
    );
}
