//! `add_modifier: { modifier: TreatAsDigimon, synth_identity: {...} }` —
//! the structured payload that lets the DSL express "treat this permanent as
//! a [DP] DP Digimon" (e.g. RizeGreymon BT21-044 / ShineGreymon: Burst Mode
//! BT13-020 treating a Marcus Damon Tamer as a Digimon for the turn).
//!
//! Round-trips authored YAML through `serde_yml -> CardSpec -> compile() ->
//! CompiledCard` and asserts the compiled step carries `synth_identity`.
//! (The validator-rejection cases — TreatAsDigimon without a payload, and a
//! payload on a non-TreatAsDigimon modifier — live in `validator.rs`'s
//! in-module tests, which have access to the `StubRegistry` test harness.)

use digimon_dsl::compile::compile;
use digimon_dsl::compiled::{
    CompiledCardKind, CompiledClause, CompiledColor, CompiledStep, CompiledSynthIdentity,
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

const HEADER: &str = r#"
card: X-1
name: Test
kind: digimon
level: 5
color: [yellow]
cost: 7
dp: 7000
"#;

#[test]
fn treat_as_digimon_minimal_payload_defaults_kind_digimon() {
    // "treat the Tamer as a 3000 DP Digimon" — only `dp` is required;
    // kind defaults to Digimon, level/colors/traits default empty.
    let yaml = format!(
        "{HEADER}\
effects:
  - when: when_digivolving
    process:
      - add_modifier:
          target: self
          modifier: TreatAsDigimon
          value: 0
          expiry: end_of_turn
          synth_identity:
            dp: 3000
"
    );
    let step = compile_first_step(&yaml);
    match step {
        CompiledStep::AddModifier {
            modifier,
            synth_identity,
            ..
        } => {
            assert_eq!(modifier, "TreatAsDigimon");
            assert_eq!(
                synth_identity,
                Some(CompiledSynthIdentity {
                    kind: CompiledCardKind::Digimon,
                    level: 0,
                    colors: vec![],
                    traits: vec![],
                    dp: 3000,
                })
            );
        }
        other => panic!("expected AddModifier, got {other:?}"),
    }
}

#[test]
fn treat_as_digimon_full_payload_round_trips() {
    let yaml = format!(
        "{HEADER}\
effects:
  - when: when_digivolving
    process:
      - add_modifier:
          target: self
          modifier: TreatAsDigimon
          value: 0
          expiry: end_of_turn
          synth_identity:
            dp: 12000
            kind: digimon
            level: 6
            colors: [red, yellow]
            traits: [Dragon]
"
    );
    let step = compile_first_step(&yaml);
    match step {
        CompiledStep::AddModifier { synth_identity, .. } => {
            assert_eq!(
                synth_identity,
                Some(CompiledSynthIdentity {
                    kind: CompiledCardKind::Digimon,
                    level: 6,
                    colors: vec![CompiledColor::Red, CompiledColor::Yellow],
                    traits: vec!["Dragon".to_string()],
                    dp: 12000,
                })
            );
        }
        other => panic!("expected AddModifier, got {other:?}"),
    }
}

#[test]
fn scalar_modifier_has_no_synth_identity() {
    let yaml = format!(
        "{HEADER}\
effects:
  - when: when_digivolving
    process:
      - add_modifier:
          target: self
          modifier: CannotDigivolve
          value: 0
          expiry: end_of_turn
"
    );
    let step = compile_first_step(&yaml);
    match step {
        CompiledStep::AddModifier { synth_identity, .. } => assert_eq!(synth_identity, None),
        other => panic!("expected AddModifier, got {other:?}"),
    }
}
