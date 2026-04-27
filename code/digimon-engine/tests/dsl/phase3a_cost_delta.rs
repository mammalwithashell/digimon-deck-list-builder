use digimon_dsl::compiled::{CompiledClause, CompiledCostDelta, CompiledStep};

#[test]
fn effect_initiated_digivolve_cost_accepts_reduce_delta() {
    let yaml = r#"
card: DSL-3A-COST
name: Cost Delta
kind: digimon
level: 4
color: [red]
cost: 4
dp: 4000
effects:
  - when: on_play
    process:
      - effect_initiated_digivolve:
          target: base
          from_hand: evo
          cost: { reduce: 2 }
          ignore_requirements: true
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::EffectInitiatedDigivolve { cost, .. } = &triggered.process[0] else {
        panic!("expected effect-initiated digivolve step");
    };
    assert_eq!(*cost, CompiledCostDelta::Reduce(2));
}

#[test]
fn effect_initiated_dna_digivolve_cost_accepts_reduce_delta() {
    let yaml = r#"
card: DSL-3A-DNA-COST
name: DNA Cost Delta
kind: digimon
level: 6
color: [red]
cost: 8
dp: 8000
effects:
  - when: on_play
    process:
      - effect_initiated_dna_digivolve:
          target_a: a
          target_b: b
          from_hand: result
          cost: { reduce: 3 }
          ignore_requirements: true
"#;

    let spec: digimon_dsl::CardSpec = serde_yml::from_str(yaml).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();

    let CompiledClause::Triggered(triggered) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let CompiledStep::EffectInitiatedDnaDigivolve { cost, .. } = &triggered.process[0] else {
        panic!("expected effect-initiated DNA digivolve step");
    };
    assert_eq!(*cost, CompiledCostDelta::Reduce(3));
}
