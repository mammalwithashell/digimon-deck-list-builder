use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};

fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/st20/ST20-05.yaml"),
    )
    .expect("ST20-05 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    digimon_dsl::compile::compile(&spec).expect("YAML compiles")
}

#[test]
fn st20_05_loads_security_play_and_two_target_security_attack_clause() {
    let card = compiled_card();
    assert_eq!(card.effects.len(), 3);

    let CompiledClause::Triggered(security) = &card.effects[0] else {
        panic!("security clause should be triggered");
    };
    assert_eq!(security.when, vec![CompiledTiming::OnSecurity]);
    assert!(matches!(
        security.process.as_slice(),
        [CompiledStep::PlayFromSecurity { .. }]
    ));

    let CompiledClause::Triggered(main) = &card.effects[1] else {
        panic!("main clause should be triggered");
    };
    assert_eq!(
        main.when,
        vec![CompiledTiming::OnPlay, CompiledTiming::WhenDigivolving]
    );
    assert!(matches!(
        main.process.first(),
        Some(CompiledStep::SelectCountCappedMulti { .. })
    ));
}
