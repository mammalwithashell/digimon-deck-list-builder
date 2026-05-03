use digimon_dsl::{compile::compile, spec::CardSpec};

#[test]
fn replacement_subject_and_source_predicates_compile_together() {
    let yaml = r#"
card: TEST-CROSS-REPLACEMENT
name: Cross Replacement Test
kind: digimon
color: [yellow]
level: 6
cost: 11
dp: 11000
effects:
  - kind: replacement
    timing: when_would_be_deleted
    active_when:
      replacement_subject_is_mine: true
      replacement_source_is_opponent: false
      replacement_cause: opponent_effect
    outcome: prevent
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("yaml parses");
    let compiled = compile(&spec).expect("replacement compiles");
    assert_eq!(compiled.card, "TEST-CROSS-REPLACEMENT");
}
