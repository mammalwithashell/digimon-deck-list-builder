use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::spec::CardSpec;

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn format_roundtrip_minimal() {
    let original = r#"
card: ST2-13
name: Hammer Spark
kind: option
color:
  - red
cost: 0
"#;
    let spec = parse(original);
    let formatted = format_spec(&spec);
    let reparsed: CardSpec = serde_yml::from_str(&formatted).unwrap();
    assert_eq!(reparsed.card, spec.card);
    assert_eq!(reparsed.name, spec.name);
    assert_eq!(reparsed.kind, spec.kind);
    assert_eq!(reparsed.color, spec.color);
}

#[test]
fn format_is_idempotent() {
    let spec = parse(r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#);
    let first = format_spec(&spec);
    let reparsed: CardSpec = serde_yml::from_str(&first).unwrap();
    let second = format_spec(&reparsed);
    assert_eq!(first, second, "pretty-print should be idempotent");
}

#[test]
fn format_preserves_top_level_key_order() {
    let spec = parse(r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
traits: [Reptile]
"#);
    let formatted = format_spec(&spec);
    let card_idx = formatted.find("card:").unwrap();
    let name_idx = formatted.find("name:").unwrap();
    let kind_idx = formatted.find("kind:").unwrap();
    let level_idx = formatted.find("level:").unwrap();
    assert!(card_idx < name_idx && name_idx < kind_idx && kind_idx < level_idx);
}
