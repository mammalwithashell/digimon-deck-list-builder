use digimon_dsl::compiled::{
    CompiledAttackTargetSpec, CompiledBindingRef, CompiledClause, CompiledStep,
};
use digimon_dsl::{compile::compile, spec::CardSpec};

#[test]
fn may_attack_now_yaml_lowers_to_compiled_step() {
    let yaml = r#"
card: TEST-MAY-ATTACK-NOW
name: May Attack Now Test
kind: digimon
color: [red]
level: 4
cost: 4
dp: 5000
effects:
  - when: when_digivolving
    process:
      - may_attack_now:
          attacker: this
          targets: player
          without_suspending: true
          optional: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("yaml compiles");
    let CompiledClause::Triggered(clause) = &compiled.effects[0] else {
        panic!("expected triggered clause");
    };
    let process = &clause.process;
    let CompiledStep::MayAttackNow {
        attacker,
        targets,
        without_suspending,
        optional,
        prompt,
    } = &process[0]
    else {
        panic!("expected may_attack_now step");
    };
    assert_eq!(*attacker, CompiledBindingRef::Source);
    assert_eq!(*targets, CompiledAttackTargetSpec::Player);
    assert!(*without_suspending);
    assert!(*optional);
    assert_eq!(prompt, &None);
}
