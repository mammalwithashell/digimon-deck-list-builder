use digimon_dsl::compiled::{CompiledClause, CompiledModifierValue, CompiledStep, CompiledTiming};

fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt15/BT15-034.yaml"),
    )
    .expect("BT15-034 YAML exists");
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(&yaml).expect("YAML parses");
    digimon_dsl::compile::compile(&spec).expect("YAML compiles")
}

fn literal(value: &CompiledModifierValue) -> Option<i32> {
    match value {
        CompiledModifierValue::Literal(value) => Some(*value),
        CompiledModifierValue::Formula(_) => None,
    }
}

#[test]
fn bt15_034_loads_security_threshold_branches_and_inherited_removal_dp() {
    let card = compiled_card();
    assert_eq!(card.effects.len(), 3);

    let CompiledClause::Triggered(add_top) = &card.effects[0] else {
        panic!("first clause should be triggered");
    };
    assert_eq!(add_top.when, vec![CompiledTiming::StartOfYourMainPhase]);
    assert!(matches!(
        add_top.process.as_slice(),
        [CompiledStep::MayAddTopSecurityToHand { .. }]
    ));

    let CompiledClause::Triggered(place_hand) = &card.effects[1] else {
        panic!("second clause should be triggered");
    };
    assert_eq!(place_hand.when, vec![CompiledTiming::StartOfYourMainPhase]);
    assert!(place_hand
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::SelectHand { .. })));

    let CompiledClause::Triggered(inherited) = &card.effects[2] else {
        panic!("inherited clause should be triggered");
    };
    assert_eq!(inherited.when, vec![CompiledTiming::OnOwnSecurityRemoved]);
    assert!(inherited
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddDpModifier { value, .. } if literal(value) == Some(-2000))));
}
