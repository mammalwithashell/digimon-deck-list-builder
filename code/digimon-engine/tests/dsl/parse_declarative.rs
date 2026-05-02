use digimon_engine::dsl::clause::TypedDeclarativeBody;
use digimon_engine::dsl::spec::CardSpec;

fn parse(yaml: &str) -> CardSpec {
    serde_yml::from_str(yaml).unwrap()
}

fn typed_body(spec: &CardSpec, idx: usize) -> TypedDeclarativeBody {
    spec.effects[idx]
        .as_declarative()
        .unwrap()
        .typed_body()
        .unwrap()
}

#[test]
fn parse_aura_dp_grant() {
    let yaml = r#"
card: BT5-093
name: Tai & Matt
kind: tamer
color: [red, blue]
cost: 4
effects:
  - kind: aura
    active_when: { your_turn: true }
    target:
      of: you
      zone: [battle_area]
      name_contains: Omnimon
    dp_modifier: 1000
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::Aura(a) => assert_eq!(a.dp_modifier, Some(1000)),
        _ => panic!("expected Aura"),
    }
}

#[test]
fn parse_cost_reduction_static() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - kind: cost_reduction
    scope: face_up
    when_playing_this: true
    condition:
      any_permanent:
        of: you
        kind: tamer
        name_contains: Tai Kamiya
    amount: 3
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::CostReduction(c) => {
            assert_eq!(c.amount, Some(3));
            assert!(c.when_playing_this);
        }
        _ => panic!("expected CostReduction"),
    }
}

#[test]
fn parse_flood_gate() {
    let yaml = r#"
card: BT13-007
name: King Drasil
kind: digi_egg
color: [yellow]
cost: 0
effects:
  - kind: flood_gate
    scope: face_up
    active_when: { all_of: [{ in_breeding: true }, { your_turn: true }] }
    modifier: CannotDigivolve
    target: { of: you, zone: [battle_area] }
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::FloodGate(f) => {
            assert_eq!(f.modifier, "CannotDigivolve");
        }
        _ => panic!("expected FloodGate"),
    }
}

#[test]
fn parse_player_scoped_flood_gate() {
    let yaml = r#"
card: BT14-009
name: Gotsumon
kind: digimon
level: 3
color: [black]
cost: 3
dp: 3000
effects:
  - kind: flood_gate
    scope: face_up
    modifier: CannotPlayDigimonByEffect
    target_player: any
    summary: "Players can't play Digimon by effects"
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::FloodGate(f) => {
            assert_eq!(f.modifier, "CannotPlayDigimonByEffect");
            assert_eq!(f.target_player, Some(digimon_dsl::common::PlayerRef::Any));
            assert!(f.target.is_none());
        }
        _ => panic!("expected FloodGate"),
    }
}

#[test]
fn parse_grant_keyword() {
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
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::GrantKeyword(k) => {
            assert_eq!(k.keyword, "Raid");
            assert_eq!(k.value, None);
        }
        _ => panic!("expected GrantKeyword"),
    }
}

#[test]
fn parse_grant_keyword_overclock_cost_filter() {
    let yaml = r#"
card: PUPPET-OVERCLOCK
name: Puppet Overclock
kind: digimon
level: 4
color: [yellow]
cost: 4
dp: 4000
effects:
  - kind: grant_keyword
    keyword: Overclock
    overclock_cost_filter:
      all_of:
        - kind: digimon
        - trait_has: Puppet
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::GrantKeyword(k) => {
            let filter = k
                .overclock_cost_filter
                .expect("Overclock grant should accept a cost filter");
            assert_eq!(filter.all_of.len(), 2);
            assert_eq!(filter.all_of[1].trait_has.as_deref(), Some("Puppet"));
        }
        _ => panic!("expected GrantKeyword"),
    }
}

#[test]
fn parse_partition_sources() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
effects:
  - kind: partition
    sources:
      - { name_contains: WarGreymon }
      - { name_contains: MetalGarurumon }
    exclude_cause: [own_effect, battle]
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::Partition(p) => {
            assert_eq!(p.sources.len(), 2);
            assert_eq!(p.exclude_cause.len(), 2);
        }
        _ => panic!("expected Partition"),
    }
}

#[test]
fn parse_raw_rust_clause() {
    let yaml = r#"
card: BT10-111
name: Shoutmon KV
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
effects:
  - kind: raw_rust
    fn: bt10_111_replacement_wildcard
"#;
    let spec = parse(yaml);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::RawRust(r) => {
            assert_eq!(r.fn_name, "bt10_111_replacement_wildcard");
        }
        _ => panic!("expected RawRust"),
    }
}

#[test]
fn cost_reduction_reduction_timing_is_populated_independently_of_clause_scope() {
    let yaml = r#"
card: BT17-015
name: WarGreymon
kind: digimon
level: 6
color: [red]
cost: 11
dp: 12000
effects:
  - kind: cost_reduction
    scope: inherited
    reduction_timing: before_pay_cost
    when_playing_this: true
    amount: 3
"#;
    let spec = parse(yaml);
    let d = spec.effects[0].as_declarative().unwrap();
    assert_eq!(d.scope, digimon_engine::dsl::clause::ClauseScope::Inherited);
    match typed_body(&spec, 0) {
        TypedDeclarativeBody::CostReduction(c) => {
            assert_eq!(c.reduction_timing.as_deref(), Some("before_pay_cost"));
        }
        _ => panic!("expected CostReduction"),
    }
}
