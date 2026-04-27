use digimon_engine::dsl::raw_rust_registry::{RawRustRegistry, StubRegistry};
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).unwrap()
}

fn ctx(reg: &dyn RawRustRegistry) -> ValidationContext<'_> {
    ValidationContext { raw_rust: reg }
}

#[test]
fn validate_well_formed_card_passes() {
    let spec = parse(
        r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#,
    );
    let reg = StubRegistry::empty();
    assert!(validate(&spec, &ctx(&reg)).is_ok());
}

#[test]
fn validate_unknown_modifier_name_fails() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: tamer
color: [red]
cost: 1
effects:
  - kind: flood_gate
    active_when: { your_turn: true }
    modifier: NotAModifierEnumVariant
    target: { of: you, zone: [battle_area] }
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("modifier")));
}

#[test]
fn validate_unknown_keyword_name_fails() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: grant_keyword
    keyword: Flyers
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("keyword")));
}

#[test]
fn validate_invalid_expiry_fails() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    process:
      - add_dp_modifier:
          target: self
          value: 1000
          expiry: forever_and_ever
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("expiry")));
}

#[test]
fn validate_declarative_body_type_mismatch() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - kind: aura
    # missing required target: field
    dp_modifier: 500
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(errs.iter().any(|e| e.path.contains("effects[0]")));
}

#[test]
fn validate_unknown_has_keyword_in_predicate_fails() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_play
    condition:
      has_keyword: NotARealKeyword
    process:
      - gain_memory: 1
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(
        errs.iter().any(|e| e.message.contains("NotARealKeyword")),
        "expected keyword typo to be reported, got: {errs:?}"
    );
}

#[test]
fn validate_unknown_aura_grant_keyword_fails() {
    let spec = parse(
        r#"
card: X-1
name: Test
kind: tamer
color: [red]
cost: 4
effects:
  - kind: aura
    active_when: { your_turn: true }
    target:
      of: you
      zone: [battle_area]
    grant_keyword: { keyword: AlsoNotRealKwrd, value: 1 }
"#,
    );
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ctx(&reg)).unwrap_err();
    assert!(
        errs.iter().any(|e| e.message.contains("AlsoNotRealKwrd")),
        "expected aura grant_keyword typo to be reported, got: {errs:?}"
    );
}
