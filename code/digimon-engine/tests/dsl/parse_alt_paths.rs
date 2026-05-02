use digimon_engine::dsl::alt_path::{AltPathKind, CostSpec, RepeatSpec};
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_standard_digivolve() {
    let yaml = r#"
card: BT17-007
name: Agumon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    assert_eq!(spec.alt_paths.len(), 1);
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::Digivolve));
    assert_eq!(ap.cost, Some(CostSpec::Literal(0)));
}

#[test]
fn parse_dna_digivolve() {
    let yaml = r#"
card: AD1-025
name: Omnimon
kind: digimon
level: 7
color: [red, blue]
cost: 15
dp: 13000
alt_paths:
  - kind: dna_digivolve
    materials:
      - { level_eq: 6, name_contains: Greymon }
      - { level_eq: 6, name_contains: Garurumon }
    cost: 0
    stacks_unsuspended: true
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::DnaDigivolve));
    assert_eq!(ap.materials.len(), 2);
    assert_eq!(ap.stacks_unsuspended, true);
}

#[test]
fn parse_digixros_unbounded() {
    let yaml = r#"
card: BT12-112
name: Shoutmon X7 Superior Mode
kind: digimon
level: 7
color: [red]
cost: 15
dp: 17000
alt_paths:
  - kind: digixros
    materials:
      - filter:
          any_of:
            - trait_has: Xros Heart
            - trait_has: Blue Flare
        repeat: unbounded
        distinct_by: card_number
    cost:
      formula:
        base: 15
        per: material_count
        delta: -1
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::DigiXros));
    assert_eq!(ap.materials.len(), 1);
    assert!(matches!(
        ap.materials[0].repeat,
        Some(RepeatSpec::Keyword(_))
    ));
    match &ap.cost {
        Some(CostSpec::Formula(_)) => {}
        other => panic!("expected formula cost, got {:?}", other),
    }
}

#[test]
fn parse_burst_digivolve_with_extra_cost_and_teardown() {
    let yaml = r#"
card: BT13-060
name: "Rosemon: Burst Mode"
kind: digimon
level: 7
color: [green]
cost: 15
dp: 15000
alt_paths:
  - kind: burst_digivolve
    from: { level_eq: 6, name_is: Rosemon }
    cost: 0
    extra_cost:
      - select_own_permanent:
          bind_as: yoshi
          filter: { kind: tamer, name_is: Yoshino Fujieda }
          prompt: Return Yoshino Fujieda
      - return_to_hand: { target: yoshi }
    on_burst_turn_end:
      - trash_top_source: { target: self }
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let ap = &spec.alt_paths[0];
    assert!(matches!(ap.kind, AltPathKind::BurstDigivolve));
    assert_eq!(ap.extra_cost.as_ref().map(|v| v.len()), Some(2));
    assert_eq!(ap.on_burst_turn_end.as_ref().map(|v| v.len()), Some(1));
    let from = ap.from.as_ref().expect("from should be set");
    assert_eq!(from.level_eq, Some(6));
    assert_eq!(from.name_is.as_deref(), Some("Rosemon"));
}

#[test]
fn material_inline_predicate_typo_is_silently_accepted() {
    // Documents a known serde limitation: MaterialSpec cannot use
    // `#[serde(deny_unknown_fields)]` because it combines with `#[serde(flatten)]`
    // of the inline PredicateSpec. Typos in inline predicate fields are silently
    // dropped at parse time. Semantic validator (Task 12) must re-check.
    let yaml = r#"
card: X-1
name: Test
kind: digimon
level: 3
color: [red]
cost: 3
dp: 2000
alt_paths:
  - kind: digivolve
    from: { name_is: Koromon }
    cost: 0
    materials:
      - levle_eq: 6     # typo — silently dropped
        name_contains: Greymon
"#;
    let spec: digimon_engine::dsl::spec::CardSpec =
        serde_yml::from_str(yaml).expect("must parse despite typo (documents blindspot)");
    // Material parsed with only the valid field populated; the typo is gone.
    let m = &spec.alt_paths[0].materials[0];
    assert_eq!(m.inline_filter.name_contains.as_deref(), Some("Greymon"));
    // level_eq stays None because `levle_eq` was dropped.
    assert_eq!(m.inline_filter.level_eq, None);
}
