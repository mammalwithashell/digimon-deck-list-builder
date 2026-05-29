use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};

fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/ex6/EX6-030.yaml"),
    )
    .expect("EX6-030 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    digimon_dsl::compile::compile(&spec).expect("YAML compiles")
}

#[test]
fn ex6_030_loads_security_play_search_and_replacement_prevention() {
    let card = compiled_card();
    assert_eq!(card.effects.len(), 2);

    let CompiledClause::Triggered(when_digivolving) = &card.effects[0] else {
        panic!("EX6-030 first clause should be triggered");
    };
    assert_eq!(when_digivolving.when, vec![CompiledTiming::WhenDigivolving]);
    assert!(when_digivolving
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectSecurity { .. })));
    assert!(when_digivolving.process.iter().any(|step| matches!(
        step,
        CompiledStep::PlaySecurityCard { .. } | CompiledStep::If { .. }
    )));

    assert!(matches!(
        card.effects[1],
        CompiledClause::Declarative(CompiledDeclarativeClause::Replacement { .. })
    ));
}
