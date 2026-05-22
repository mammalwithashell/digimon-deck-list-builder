use std::collections::HashMap;
use std::sync::Arc;

use digimon_dsl::compiled::{
    CompiledClause, CompiledCostDelta, CompiledFormula, CompiledScope, CompiledStep,
};
use digimon_dsl::spec::CardSpec;
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardHandle;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl_bridge::enrich_card_data_with_dsl_alt_paths;
use digimon_engine::dsl_cards::lower_cost_reduction;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::DslCardEffect;
use digimon_engine::effect::CardEffect;
use digimon_engine::effect_context::EffectReadContext;
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn cost_delta_reduce_compiles_for_play_steps() {
    let yaml = r#"
card: TEST-COST
name: Cost Reducer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - when: [on_play]
    process:
      - play_from_hand:
          of: you
          hand_index: picked
          cost_delta: { reduce: 2 }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    match &compiled.effects[0] {
        CompiledClause::Triggered(triggered) => match &triggered.process[0] {
            CompiledStep::PlayFromHand { cost_delta, .. } => {
                assert_eq!(*cost_delta, Some(CompiledCostDelta::Reduce(2)));
            }
            other => panic!("expected play_from_hand, got {other:?}"),
        },
        other => panic!("expected triggered clause, got {other:?}"),
    }
}

fn plain_digimon(card_id: &str, play_cost: u16) -> CardData {
    CardData {
        card_id: card_id.to_string(),
        card_name: card_id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(2000),
        play_cost,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: card_id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: Vec::new(),
        also_treated_as: Vec::new(),
    }
}

#[test]
fn dsl_dna_alt_path_enriches_card_data_dna_costs() {
    let yaml = r#"
card: DNA-DSL
name: DNA DSL
kind: digimon
level: 6
color: [red, blue]
cost: 12
dp: 12000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 5, color_is: red }
      - { name_contains: Garuru }
    cost: 2
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let registry =
        digimon_dsl::CardRegistry::from_specs("phase3-dna", &[spec]).expect("compile registry");

    let mut cards = HashMap::from([("DNA-DSL".to_string(), plain_digimon("DNA-DSL", 12))]);
    assert_eq!(cards["DNA-DSL"].dna_costs.len(), 0);

    let inserted = enrich_card_data_with_dsl_alt_paths(&mut cards, &registry);

    assert_eq!(inserted, 1);
    let dna_cost = &cards["DNA-DSL"].dna_costs[0];
    assert_eq!(dna_cost.memory_cost, 2);
    assert_eq!(dna_cost.requirement1.level, 5);
    assert_eq!(dna_cost.requirement1.card_colors, vec![CardColor::Red]);
    assert_eq!(dna_cost.requirement2.name_contains, "Garuru");
}

#[test]
fn cost_reduction_amount_fn_uses_formula_value() {
    let effect = lower_cost_reduction::lower_with_formula(
        CardHandle(0),
        CompiledScope::FaceUp,
        None,
        None,
        false,
        Some(CompiledFormula::Literal(4)),
        vec![],
        Arc::new(EngineRawRustRegistry::new()),
        false,
        false,
        None,
        None,
    );

    let runner = DebugRunner::builder().build();
    let ctx = EffectReadContext::new(&runner.game, CardHandle(0), None, 0);

    assert_eq!(
        (effect.cost_reduction_fn.as_ref().expect("cost fn"))(&ctx),
        4
    );
}

#[test]
fn cost_reduction_amount_fn_uses_raw_formula_registry() {
    let mut raw = EngineRawRustRegistry::new();
    raw.register_formula("cost_formula", |_ctx, _target| 5);
    let effect = lower_cost_reduction::lower_with_formula(
        CardHandle(0),
        CompiledScope::FaceUp,
        None,
        None,
        false,
        Some(CompiledFormula::RawRust("cost_formula".into())),
        vec![],
        Arc::new(raw),
        false,
        false,
        None,
        None,
    );

    let mut runner = DebugRunner::builder()
        .add_card(plain_digimon("REDUCER-RAW", 3))
        .build();
    let permanent = runner.place_on_field(0, "REDUCER-RAW", None);
    let source = runner.game.player(0).battle_area[0].top_card().handle();
    let ctx = EffectReadContext::new(&runner.game, source, Some(permanent), 0);

    assert_eq!(
        (effect.cost_reduction_fn.as_ref().expect("cost fn"))(&ctx),
        5
    );
}

#[test]
fn dsl_cost_reduction_amount_fn_without_literal_amount_emits_effect() {
    let yaml = r#"
card: TEST-AMOUNT-FN
name: Formula Reducer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: cost_reduction
    when_playing_this: true
    amount_fn: 4
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    let dsl = DslCardEffect::new(Arc::new(compiled));
    let effects = dsl.effects(CardHandle(0));
    assert_eq!(effects.len(), 1);
    assert!(effects[0].cost_reduction_fn.is_some());
}

#[test]
fn dsl_cost_reduction_pay_cost_body_runs_before_reduction_applies() {
    let reducer_yaml = r#"
card: REDUCER-DSL
name: Reducer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - kind: cost_reduction
    when_playing_this: true
    amount: 2
    pay_cost:
      - trash_from_top: { of: you, count: 2 }
"#;
    let reducer_spec: CardSpec = serde_yml::from_str(reducer_yaml).expect("parse reducer");
    let reducer_compiled = digimon_dsl::compile::compile(&reducer_spec).expect("compile reducer");

    let mut runner = DebugRunner::builder()
        .add_card(plain_digimon("REDUCER-DSL", 3))
        .add_card(plain_digimon("FILLER-DSL", 3))
        .deck(0, &["FILLER-DSL", "FILLER-DSL", "FILLER-DSL"])
        .hand(0, &["REDUCER-DSL"])
        .memory(10)
        .start();
    runner.register_effect(
        "REDUCER-DSL",
        Arc::new(DslCardEffect::new(Arc::new(reducer_compiled))),
    );
    runner.place_on_field(0, "REDUCER-DSL", Some(0));

    let deck_before = runner.game.players[0].deck.len();
    let trash_before = runner.game.players[0].trash.len();
    runner.play(0, 0);
    runner.auto_resolve().expect("accept paid cost reducer");

    assert_eq!(runner.memory(), 9, "printed cost 3 reduced by 2");
    assert_eq!(runner.game.players[0].deck.len(), deck_before - 2);
    assert_eq!(runner.game.players[0].trash.len(), trash_before + 2);
}
