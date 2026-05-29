use digimon_engine::dsl::clause::{ClauseScope, DeclarativeKind, Timing, TimingSet};
use digimon_engine::dsl::pretty::format_spec;
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_triggered_clause_single_timing() {
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
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let c = &spec.effects[0];
    let t = c.as_triggered().expect("triggered");
    assert!(matches!(t.when, TimingSet::Single(Timing::MainFromHand)));
    assert_eq!(t.scope, ClauseScope::FaceUp);
    assert!(!t.optional);
    assert!(!t.once_per_turn);
}

#[test]
fn parse_triggered_clause_multiple_timings() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - when: [on_play, when_digivolving]
    process:
      - gain_memory: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    match &t.when {
        TimingSet::Multi(v) => {
            assert_eq!(v, &vec![Timing::OnPlay, Timing::WhenDigivolving]);
        }
        _ => panic!("expected multi"),
    }
}

#[test]
fn parse_inherited_scope_clause() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - scope: inherited
    when: when_attacking
    once_per_turn: true
    process:
      - trash_top_security: { of: opponent }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    assert_eq!(t.scope, ClauseScope::Inherited);
    assert!(t.once_per_turn);
}

#[test]
fn parse_attack_observer_timings() {
    let yaml = r#"
card: OBSERVER
name: Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 1000
effects:
  - scope: inherited
    when: [on_ally_attack, on_opponent_attack]
    process:
      - end_attack: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    match &t.when {
        TimingSet::Multi(v) => {
            assert_eq!(v, &vec![Timing::OnAllyAttack, Timing::OnOpponentAttack]);
        }
        _ => panic!("expected multi"),
    }
}

#[test]
fn parse_on_block_timing_clause() {
    let yaml = r#"
card: DSL-BLOCK
name: Block Observer
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
effects:
  - when: on_block
    process:
      - gain_memory: 1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let t = spec.effects[0].as_triggered().unwrap();
    assert!(matches!(t.when, TimingSet::Single(Timing::OnBlock)));
}

#[test]
fn parse_declarative_grant_keyword_clause() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
effects:
  - kind: grant_keyword
    keyword: Raid
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.kind, DeclarativeKind::GrantKeyword);
}

#[test]
fn face_up_scope_is_omitted_from_output() {
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
    let spec: digimon_engine::dsl::spec::CardSpec = serde_yml::from_str(yaml).unwrap();
    let formatted = format_spec(&spec);
    assert!(
        !formatted.contains("scope:"),
        "face_up is the default scope and must not serialize; got:\n{formatted}"
    );
}

#[test]
fn inherited_scope_is_preserved_in_output() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - scope: inherited
    when: when_attacking
    process:
      - trash_top_security: { of: opponent }
"#;
    let spec: digimon_engine::dsl::spec::CardSpec = serde_yml::from_str(yaml).unwrap();
    let formatted = format_spec(&spec);
    assert!(
        formatted.contains("scope: inherited"),
        "non-default scope must serialize; got:\n{formatted}"
    );
}
