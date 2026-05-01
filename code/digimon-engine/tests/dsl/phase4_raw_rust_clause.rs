use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledCard, CompiledCardKind, CompiledClause, CompiledColor, CompiledDeclarativeClause,
    CompiledScope,
};
use digimon_engine::card_source::CardHandle;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::{CardEffect, Effect};

#[test]
fn whole_clause_raw_rust_dispatches_registered_function() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_declarative("emit_clause", |card: CardHandle| {
        vec![Effect::on_play(card)
            .name("raw emitted clause")
            .process(|ctx| ctx.gain_memory(1))
            .build()]
    });

    let compiled = CompiledCard {
        card: "RAW-CLAUSE".into(),
        name: "Raw Clause".into(),
        kind: CompiledCardKind::Digimon,
        level: Some(3),
        color: vec![CompiledColor::Red],
        cost: Some(3),
        dp: Some(1000),
        traits: vec![],
        form: None,
        attribute: None,
        ace_overflow: None,
        identity: None,
        dual: None,
        use_requirement: None,
        alt_paths: vec![],
        effects: vec![CompiledClause::Declarative(
            CompiledDeclarativeClause::RawRust {
                fn_name: "emit_clause".into(),
                triggers: vec![],
                scope: CompiledScope::FaceUp,
                summary: None,
                summary_key: None,
            },
        )],
    };

    let effect = DslCardEffect::new_with_raw(Arc::new(compiled), Arc::new(registry));
    let emitted = effect.effects(CardHandle(0));

    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0].name, "raw emitted clause");
    assert!(emitted[0].on_play);
}
