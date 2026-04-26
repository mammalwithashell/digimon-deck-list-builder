use digimon_engine::dsl::predicate::Zone;
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn parse_name_alias_xantibody() {
    let yaml = r#"
card: BT9-109
name: Omnimon (X Antibody)
kind: digimon
level: 7
color: [red, blue]
cost: 13
dp: 12000
identity:
  name_aliases:
    - treat_as: Omnimon
      when:
        zone: [battle_area]
        has_inherited:
          card_number_is: BT9-109
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).unwrap();
    let id = spec.identity.as_ref().unwrap();
    assert_eq!(id.name_aliases.len(), 1);
    let alias = &id.name_aliases[0];
    assert_eq!(alias.treat_as, "Omnimon");
    assert_eq!(alias.when.zone, vec![Zone::BattleArea]);
    let inh = alias.when.has_inherited.as_ref().unwrap();
    assert_eq!(inh.card_number_is.as_deref(), Some("BT9-109"));
}
