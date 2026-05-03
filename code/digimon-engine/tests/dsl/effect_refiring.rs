use digimon_dsl::compiled::{CompiledClause, CompiledStep};
use digimon_dsl::validator::{validate, ValidationContext};
use digimon_dsl::{compile::compile, spec::CardSpec};
use digimon_engine::dsl::raw_rust_registry::StubRegistry;

#[test]
fn refire_effect_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-REFIRE-EFFECT
name: Refire Effect Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - refire_effect:
          source: target
          timing: when_digivolving
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let process = &clause.process;
    assert!(matches!(process[0], CompiledStep::RefireEffect { .. }));
}

#[test]
fn refire_effect_rejects_unsupported_timing() {
    let yaml = r#"
card: TEST-REFIRE-BAD-TIMING
name: Refire Bad Timing Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_attacking
    process:
      - refire_effect:
          source: target
          timing: on_play
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compile_errs = compile(&spec).expect_err("compile rejects unsupported refire timing");
    assert!(
        compile_errs.iter().any(|err| err
            .message
            .contains("only supports timing: when_digivolving")),
        "expected compile-time refire timing error, got: {compile_errs:?}"
    );

    let registry = StubRegistry::empty();
    let ctx = ValidationContext {
        raw_rust: &registry,
    };
    let errs = validate(&spec, &ctx).expect_err("unsupported refire timing rejected");
    assert!(
        errs.iter().any(|err| err
            .message
            .contains("only supports timing: when_digivolving")),
        "expected refire timing validation error, got: {errs:?}"
    );
}
