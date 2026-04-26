use digimon_engine::dsl::spec::CardSpec;
use digimon_engine::dsl::step::StepSpec;

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn triggered_clause_parses_summary_and_summary_key() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - when: on_play
    summary: "Delete 8000 DP or digivolve Gabumon free"
    summary_key: BT17-015.onplay
    process:
      - gain_memory: 0
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.summary.as_deref(), Some("Delete 8000 DP or digivolve Gabumon free"));
    assert_eq!(t.summary_key.as_deref(), Some("BT17-015.onplay"));
}

#[test]
fn declarative_clause_parses_summary() {
    let yaml = r#"
card: BT5-093
name: Tai & Matt
kind: tamer
color: [red, blue]
cost: 4
effects:
  - kind: aura
    summary: "+1 Security Attack on Omnimon"
    active_when: { your_turn: true }
    target:
      of: you
      zone: [battle_area]
      name_contains: Omnimon
    grant_keyword: { keyword: SecurityAttackPlus, value: 1 }
"#;
    let spec = parse(yaml);
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.summary.as_deref(), Some("+1 Security Attack on Omnimon"));
}

#[test]
fn select_hand_args_parse_prompt_key() {
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
      - select_hand:
          of: you
          bind_as: pick
          filter: { name_contains: Koromon }
          prompt: "Return Koromon"
          prompt_key: X-1.clause0.step0
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    match &t.process[0] {
        StepSpec::SelectHand(args) => {
            assert_eq!(args.prompt, "Return Koromon");
            assert_eq!(args.prompt_key.as_deref(), Some("X-1.clause0.step0"));
        }
        _ => panic!("expected SelectHand"),
    }
}

#[test]
fn i18n_fields_are_all_optional() {
    let yaml = r#"
card: ST2-13
name: Hammer Spark
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - gain_memory: 1
"#;
    let spec = parse(yaml);
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.summary, None);
    assert_eq!(t.summary_key, None);
}
