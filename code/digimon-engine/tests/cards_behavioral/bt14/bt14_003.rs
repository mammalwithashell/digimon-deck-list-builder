use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};

fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt14/BT14-003.yaml"),
    )
    .expect("BT14-003 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    digimon_dsl::compile::compile(&spec).expect("YAML compiles")
}

#[test]
fn bt14_003_loads_inherited_security_add_draw_clause() {
    let card = compiled_card();
    let CompiledClause::Triggered(triggered) = &card.effects[0] else {
        panic!("BT14-003 effect should be triggered");
    };
    assert_eq!(triggered.when, vec![CompiledTiming::OnPlaceSecurity]);
    assert!(triggered.once_per_turn);
    assert!(matches!(
        triggered.process.as_slice(),
        [CompiledStep::Draw { count: 1, .. }]
    ));
}
