// digimon-engine/tests/dsl/phase1c_scaffold.rs
use digimon_dsl::compiled::{CompiledCard, CompiledCardKind};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use std::sync::Arc;

#[test]
fn dsl_card_effect_with_no_clauses_emits_no_effects() {
    let compiled = CompiledCard {
        card: "TEST-EMPTY".into(),
        name: "Empty".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![],
        cost: Some(0),
        dp: Some(1000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        alt_paths: vec![],
        effects: vec![],
    };
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let card = CardHandle(0);
    assert!(dsl.effects(card).is_empty());
}
