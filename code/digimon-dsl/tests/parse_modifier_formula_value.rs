//! Phase 2f2 Task 1 — `value:` on `add_dp_modifier` / `add_modifier`
//! accepts either a literal int OR a `{ formula: ... }` block.
//!
//! These tests round-trip authored YAML through `serde_yml -> CardSpec ->
//! compile() -> CompiledCard`, then drill into the compiled step and assert
//! the new `CompiledModifierValue` enum carries the expected variant.

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledClause, CompiledFormula, CompiledModifierValue, CompiledPerSelector, CompiledPlayerRef,
    CompiledStep, CompiledZone,
};
use digimon_dsl::spec::CardSpec;

fn compile_first_step(yaml: &str) -> CompiledStep {
    let spec: CardSpec = serde_yml::from_str(yaml).expect("YAML parse");
    let compiled = compile(&spec).expect("compile");
    let clause = &compiled.effects[0];
    let process = match clause {
        CompiledClause::Triggered(t) => &t.process,
        _ => panic!("expected triggered clause"),
    };
    process[0].clone()
}

#[test]
fn add_dp_modifier_value_literal_lowers_to_literal_variant() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_dp_modifier:
          target: self
          value: 3000
          expiry: end_of_attack
"#;
    let step = compile_first_step(yaml);
    match step {
        CompiledStep::AddDpModifier { value, .. } => {
            assert_eq!(value, CompiledModifierValue::Literal(3000));
        }
        other => panic!("expected AddDpModifier, got {other:?}"),
    }
}

#[test]
fn add_dp_modifier_value_formula_lowers_to_formula_variant() {
    // +1000 DP per material — Susanoomon-style.
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_dp_modifier:
          target: self
          value:
            formula:
              base: 0
              per: stack_size
              delta: 1000
          expiry: end_of_attack
"#;
    let step = compile_first_step(yaml);
    match step {
        CompiledStep::AddDpModifier { value, .. } => match value {
            CompiledModifierValue::Formula(CompiledFormula::BasePerDelta { base, per, delta }) => {
                assert_eq!(base, 0);
                assert_eq!(per, CompiledPerSelector::StackSize);
                assert_eq!(delta, 1000);
            }
            other => panic!("expected Formula(BasePerDelta), got {other:?}"),
        },
        other => panic!("expected AddDpModifier, got {other:?}"),
    }
}

#[test]
fn add_modifier_value_literal_lowers_to_literal_variant() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_modifier:
          target: self
          modifier: CannotAttack
          value: 1
          expiry: end_of_your_turn
"#;
    let step = compile_first_step(yaml);
    match step {
        CompiledStep::AddModifier { value, .. } => {
            assert_eq!(value, CompiledModifierValue::Literal(1));
        }
        other => panic!("expected AddModifier, got {other:?}"),
    }
}

#[test]
fn add_modifier_value_formula_lowers_to_formula_variant() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_modifier:
          target: self
          modifier: ExtraDpFromMaterials
          value:
            formula:
              base: 0
              per: material_count
              delta: 2000
          expiry: end_of_attack
"#;
    let step = compile_first_step(yaml);
    match step {
        CompiledStep::AddModifier { value, .. } => match value {
            CompiledModifierValue::Formula(CompiledFormula::BasePerDelta { base, per, delta }) => {
                assert_eq!(base, 0);
                assert_eq!(per, CompiledPerSelector::MaterialCount);
                assert_eq!(delta, 2000);
            }
            other => panic!("expected Formula(BasePerDelta), got {other:?}"),
        },
        other => panic!("expected AddModifier, got {other:?}"),
    }
}

#[test]
fn card_count_in_zone_formula_lowers_player_and_zone_payload() {
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 6
color: [red]
cost: 8
dp: 9000
effects:
  - when: when_attacking
    process:
      - add_dp_modifier:
          target: self
          value:
            formula:
              base: 0
              per:
                card_count_in_zone:
                  of: opponent
                  zone: trash
              delta: 1000
          expiry: end_of_attack
"#;
    let step = compile_first_step(yaml);
    match step {
        CompiledStep::AddDpModifier { value, .. } => match value {
            CompiledModifierValue::Formula(CompiledFormula::BasePerDelta { base, per, delta }) => {
                assert_eq!(base, 0);
                assert_eq!(
                    per,
                    CompiledPerSelector::CardCountInZone {
                        of: CompiledPlayerRef::Opponent,
                        zone: CompiledZone::Trash,
                    }
                );
                assert_eq!(delta, 1000);
            }
            other => panic!("expected Formula(BasePerDelta), got {other:?}"),
        },
        other => panic!("expected AddDpModifier, got {other:?}"),
    }
}
