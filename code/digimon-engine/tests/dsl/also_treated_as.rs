//! Card-level `also_treated_as` — the "this card is also treated as
//! [Other Name]" identity mechanic (DCGO `ChangeCardNamesClass`).
//!
//! Unlike `digixros_aliases` (visible only to DigiXros recipe matching),
//! `also_treated_as` is an always-on identity alias honored by generic
//! name-matching predicates (`name_is` / `name_contains` / `name_in`) in
//! every zone.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::dsl::spec::CardSpec;

#[test]
fn also_treated_as_parses_list_form() {
    let yaml = r#"
card: BT23-077
name: Sistermon Ciel
kind: digimon
level: 4
color: [white]
cost: 4
dp: 4000
also_treated_as: [Sistermon Noir]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    assert_eq!(spec.also_treated_as, vec!["Sistermon Noir".to_string()]);
}

#[test]
fn also_treated_as_parses_multiple_names() {
    let yaml = r#"
card: TEST-MULTI
name: Multi Alias
kind: digimon
level: 4
color: [white]
cost: 4
dp: 4000
also_treated_as: [Sistermon Noir, Sistermon Blanc]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    assert_eq!(
        spec.also_treated_as,
        vec!["Sistermon Noir".to_string(), "Sistermon Blanc".to_string()]
    );
}

#[test]
fn also_treated_as_lowers_into_compiled_card() {
    let yaml = r#"
card: BT23-077
name: Sistermon Ciel
kind: digimon
level: 4
color: [white]
cost: 4
dp: 4000
also_treated_as: [Sistermon Noir]
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert_eq!(compiled.also_treated_as, vec!["Sistermon Noir".to_string()]);
}

#[test]
fn also_treated_as_defaults_empty_when_absent() {
    let yaml = r#"
card: TEST-PLAIN
name: Plain Digimon
kind: digimon
level: 3
color: [red]
cost: 3
dp: 3000
"#;
    let spec: CardSpec = serde_yml::from_str(yaml).expect("parse");
    assert!(spec.also_treated_as.is_empty());
    let compiled = digimon_dsl::compile::compile(&spec).expect("compile");
    assert!(compiled.also_treated_as.is_empty());
}

#[test]
fn also_treated_as_flows_from_dsl_yaml_into_runtime_card_data() {
    let yaml = r#"
card: ALIAS-BRIDGE
name: Alias Carrier
kind: digimon
level: 4
color: [white]
cost: 4
dp: 4000
also_treated_as: [Sistermon Noir]
"#;

    let runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline also_treated_as DSL should compile")
        .start();

    let card: &CardData = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "ALIAS-BRIDGE")
        .expect("runtime CardData should include DSL card");

    assert_eq!(card.also_treated_as, vec!["Sistermon Noir".to_string()]);
}

/// A card with `also_treated_as: [X]` must be matched by a generic
/// `name_is` predicate that names `X` — the alias is honored in addition
/// to the printed name.
#[test]
fn name_is_predicate_matches_via_also_treated_as_alias() {
    let alias_card = r#"
card: ALIAS-CIEL
name: Sistermon Ciel
kind: digimon
level: 4
color: [white]
cost: 4
dp: 4000
also_treated_as: [Sistermon Noir]
"#;
    // A seeker whose [On Play] effect selects an own Digimon named
    // "Sistermon Noir". Only the aliased card should be a legal target.
    let seeker = r#"
card: NAME-SEEKER
name: Name Seeker
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
effects:
  - when: on_play
    summary: "Select 1 of your Digimon named [Sistermon Noir]"
    condition:
      any_permanent:
        of: you
        zone: [battle_area]
        name_is: Sistermon Noir
    process:
      - select_own_permanent:
          bind_as: target
          filter: { name_is: Sistermon Noir }
          prompt: "Select a Digimon named Sistermon Noir"
"#;
    let plain = r#"
card: PLAIN-DIGI
name: Plain Digimon
kind: digimon
level: 3
color: [white]
cost: 3
dp: 3000
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(alias_card)
        .expect("alias card compiles")
        .from_dsl_yaml(seeker)
        .expect("seeker compiles")
        .from_dsl_yaml(plain)
        .expect("plain card compiles")
        .hand(0, &["NAME-SEEKER"])
        .memory(10)
        .start();

    let ciel = runner.place_on_field(0, "ALIAS-CIEL", Some(0));
    let plain_perm = runner.place_on_field(0, "PLAIN-DIGI", Some(0));

    runner.play(0, 0).expect("play Name Seeker");

    let view = runner
        .pending_selection_view()
        .expect("name_is selection should be installed");
    assert!(
        view.valid_action_ids.contains(&encode_permanent(ciel)),
        "Sistermon Ciel must be selectable via its [Sistermon Noir] alias"
    );
    assert!(
        !view.valid_action_ids.contains(&encode_permanent(plain_perm)),
        "a card with no matching name/alias must not be selectable"
    );
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, handle.index as u16)
}
