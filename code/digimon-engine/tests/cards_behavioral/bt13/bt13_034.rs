use digimon_dsl::compiled::{CompiledClause, CompiledModifierValue, CompiledStep, CompiledTiming};

fn compiled_card() -> digimon_dsl::compiled::CompiledCard {
    let yaml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt13/BT13-034.yaml"),
    )
    .expect("BT13-034 YAML exists");
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
fn bt13_034_loads_reveal_search_and_inherited_dp_clause() {
    let card = compiled_card();
    assert_eq!(card.effects.len(), 2);

    let CompiledClause::Triggered(on_play) = &card.effects[0] else {
        panic!("BT13-034 On Play should be triggered");
    };
    assert_eq!(on_play.when, vec![CompiledTiming::OnPlay]);
    assert!(matches!(
        on_play.process.as_slice(),
        [
            CompiledStep::RevealTopDeck { count: 3, .. },
            CompiledStep::SelectRevealBuckets { .. },
            CompiledStep::AddToHandFromReveal { .. },
            CompiledStep::AddToHandFromReveal { .. },
            CompiledStep::PlaceRemainderOnDeck { .. }
        ]
    ));

    let CompiledClause::Triggered(inherited) = &card.effects[1] else {
        panic!("BT13-034 inherited should be triggered");
    };
    assert_eq!(inherited.when, vec![CompiledTiming::WhenAttacking]);
    assert!(inherited.once_per_turn);
    assert!(inherited
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::AddDpModifier { value, .. } if literal(value) == Some(-2000))));
}
