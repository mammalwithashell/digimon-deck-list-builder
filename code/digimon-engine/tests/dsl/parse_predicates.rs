use digimon_engine::dsl::{
    predicate::DpConstraint,
    predicate::{PredicateSpec, Zone},
    PlayerRef,
};

fn parse(yaml: &str) -> PredicateSpec {
    serde_yml::from_str(yaml).unwrap()
}

#[test]
fn parse_leaf_predicates() {
    let p = parse("name_contains: Greymon");
    assert_eq!(p.name_contains.as_deref(), Some("Greymon"));

    let p = parse("level_gte: 6");
    assert_eq!(p.level_gte, Some(DpConstraint::Literal(6)));

    let p = parse("face_up_security_count_lte: 0");
    assert_eq!(p.face_up_security_count_lte, Some(DpConstraint::Literal(0)));

    let p = parse("kind: digimon");
    assert_eq!(p.kind, Some(digimon_engine::dsl::spec::CardKind::Digimon));

    let p = parse("trait_has: Royal Knight");
    assert_eq!(p.trait_has.as_deref(), Some("Royal Knight"));

    let p = parse("zone: [battle_area, trash]");
    assert_eq!(p.zone, vec![Zone::BattleArea, Zone::Trash]);

    let p = parse("owner: you");
    assert_eq!(p.owner, Some(PlayerRef::You));

    // G-UNION-HAND-TRASH-NAME-EXCLUSION (Phase 2 Track J Task S2.2):
    // name-exclusion leaf used by the Jesmon-family union play filter.
    let p = parse("name_not_shared_by_field_digimon: { of: you }");
    assert_eq!(
        p.name_not_shared_by_field_digimon.map(|s| s.player()),
        Some(PlayerRef::You)
    );

    let p = parse("name_not_shared_by_field_tamer: { of: you }");
    assert_eq!(
        p.name_not_shared_by_field_tamer.map(|s| s.player()),
        Some(PlayerRef::You)
    );
}

#[test]
fn parse_compound_predicates() {
    let yaml = r#"
any_of:
  - name_contains: Garurumon
  - name_contains: Greymon
  - name_contains: Omnimon"#;
    let p = parse(yaml);
    assert_eq!(p.any_of.len(), 3);
    assert_eq!(p.any_of[0].name_contains.as_deref(), Some("Garurumon"));
}

#[test]
fn parse_nested_all_of() {
    let yaml = r#"
all_of:
  - kind: digimon
  - dp_lte: 8000
  - any_of:
      - trait_has: Reptile
      - trait_has: Dragonkin"#;
    let p = parse(yaml);
    assert_eq!(p.all_of.len(), 3);
    assert_eq!(p.all_of[2].any_of.len(), 2);
}

#[test]
fn parse_existential_any_permanent() {
    let yaml = r#"
any_permanent:
  of: you
  zone: [battle_area]
  kind: tamer
  name_contains: "Tai Kamiya""#;
    let p = parse(yaml);
    let ex = p.any_permanent.as_ref().unwrap();
    assert_eq!(ex.of, PlayerRef::You);
    assert_eq!(ex.predicate.zone, vec![Zone::BattleArea]);
}

#[test]
fn parse_count_aggregate() {
    let yaml = r#"
count_lte:
  filter: { of: you, zone: [battle_area], kind: digimon }
  n: 1"#;
    let p = parse(yaml);
    let c = p.count_lte.as_ref().unwrap();
    assert_eq!(c.n, DpConstraint::Literal(1));
}

#[test]
fn parse_group7_predicate_leaves() {
    let p = parse("play_cost_lte: 3");
    assert_eq!(p.play_cost_lte, Some(DpConstraint::Literal(3)));

    let p = parse("not_in_binding: saved");
    assert_eq!(p.not_in_binding.as_deref(), Some("saved"));

    let p = parse(
        r#"
dp_gte:
  base: 0
  per: material_count
  delta: 1000
"#,
    );
    assert!(matches!(p.dp_gte, Some(DpConstraint::Formula(_))));
}

#[test]
fn parse_can_hatch_predicate_leaf() {
    let p = parse("can_hatch: you");
    assert_eq!(p.can_hatch, Some(PlayerRef::You));
}
