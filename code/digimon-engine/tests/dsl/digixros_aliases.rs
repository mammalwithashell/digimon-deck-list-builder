use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn digixros_material_matching_sees_scoped_alias_but_name_predicates_do_not() {
    let material = CardData {
        card_id: "MATERIAL-A".to_string(),
        card_name: "Alias Carrier".to_string(),
        card_kind: CardKind::Digimon,
        level: Some(3),
        dp: Some(3000),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: vec!["Xros Heart".to_string()],
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        dual: None,
        effect_class_name: "MATERIAL_A".to_string(),
        index: 0,
        norm_id: 0.0,
        ace_overflow: None,
        digixros_aliases: vec!["Shoutmon".to_string()],
    };

    assert!(
        digimon_engine::digixros::matches_digixros_name_requirement_for_test(&material, "Shoutmon",),
        "DigiXros recipe matching must see scoped aliases"
    );
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(&material, "Shoutmon",),
        "generic name predicates must not see DigiXros aliases"
    );
}

#[test]
fn digixros_aliases_flow_from_dsl_yaml_into_runtime_card_data() {
    let yaml = r#"
card: XROS-BRIDGE
name: Bridge Alias Carrier
kind: digimon
level: 4
color: [red]
cost: 5
dp: 4000
digixros_aliases: ["Shoutmon"]
alt_paths:
  - kind: digixros
    materials:
      - filter: { name_contains: "Shoutmon" }
        repeat: { min: 1, max: 1 }
    cost: 3
"#;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline DigiXros alias DSL should compile")
        .start();

    let material = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "XROS-BRIDGE")
        .expect("runtime CardData should include DSL card");

    assert_eq!(material.digixros_aliases, vec!["Shoutmon"]);
    assert!(
        digimon_engine::digixros::matches_digixros_name_requirement_for_test(
            material,
            "Shoutmon",
        ),
        "DigiXros recipe matching must see aliases copied by the DSL bridge"
    );
    assert!(
        !digimon_engine::digixros::matches_generic_name_requirement_for_test(
            material,
            "Shoutmon",
        ),
        "generic name predicates must still ignore scoped DigiXros aliases"
    );
}
