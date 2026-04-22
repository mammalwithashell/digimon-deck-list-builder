use digimon_engine::dsl::raw_rust_registry::StubRegistry;
use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::validator::{validate, ValidationContext};

fn card_with_raw_rust(fn_name: &str) -> CardSpec {
    let yaml = format!(r#"
card: BT10-111
name: Shoutmon KV
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - kind: raw_rust
    fn: {fn_name}
"#);
    serde_yml::from_str(&yaml).unwrap()
}

#[test]
fn missing_fn_fails_validation() {
    let spec = card_with_raw_rust("missing_fn");
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("missing_fn")));
}

#[test]
fn registered_fn_passes_validation() {
    let spec = card_with_raw_rust("present_fn");
    let reg = StubRegistry::with(["present_fn"]);
    assert!(validate(&spec, &ValidationContext { raw_rust: &reg }).is_ok());
}

#[test]
fn step_level_raw_rust_checked_against_registry() {
    let yaml = r#"
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
      - raw_rust:
          fn: unregistered_step_fn
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let reg = StubRegistry::empty();
    let errs = validate(&spec, &ValidationContext { raw_rust: &reg }).unwrap_err();
    assert!(errs.iter().any(|e| e.message.contains("unregistered_step_fn")));
}
