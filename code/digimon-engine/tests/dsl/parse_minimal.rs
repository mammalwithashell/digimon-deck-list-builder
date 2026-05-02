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
fn parses_dual_card_metadata() {
    let yaml = r#"
card: DUAL-DSL
name: Dual DSL
kind: dual
dual:
  digimon:
    level: 6
    dp: 12000
    colors: [red]
    traits: [DualTrait]
    effect_text: "[When Digivolving] Draw 1."
    inherited_text: ""
  option:
    use_cost: 5
    colors: [purple]
    effect_text: "[Main] Gain 2 memory."
    security_text: ""
    keywords: [ArtsDigivolve]
effects: []
"#;
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse dual yaml");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile dual yaml");
    assert_eq!(compiled.card, "DUAL-DSL");
    assert!(compiled.dual.is_some());
}

#[test]
fn parses_st23_09_docs_example() {
    let yaml = include_str!("../../../../docs/examples/ST23-09-dual-card.yaml");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).expect("parse ST23-09 example");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile ST23-09 example");
    let dual = compiled.dual.as_ref().expect("compiled dual metadata");
    assert!(dual.option.use_requirement.is_some());
    assert!(compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Triggered(triggered)
                if triggered.process.iter().any(|step| {
                    matches!(
                        step,
                        digimon_dsl::compiled::CompiledStep::GrantEffectImmunity { .. }
                    )
                })
        )
    }));
    assert!(compiled.effects.iter().any(|clause| {
        matches!(
            clause,
            digimon_dsl::compiled::CompiledClause::Triggered(triggered)
                if triggered.process.iter().any(|step| {
                    matches!(
                        step,
                        digimon_dsl::compiled::CompiledStep::SelectOpponentPermanent {
                            selector: Some(digimon_dsl::compiled::CompiledFieldSelector::HighestDp),
                            ..
                        }
                    )
                })
        )
    }));
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
