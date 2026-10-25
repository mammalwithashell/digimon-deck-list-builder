use digimon_engine::action::{build_action_mask, space::PASS};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

const YAML: &str = r#"
card: CARD-PROPERTY-HOST
name: Card Property Host
kind: digimon
color: [yellow]
level: 5
cost: 5
dp: 6000
effects:
  - when: on_play
    process:
      - select_material:
          of_permanent: source
          filter: {}
          bind_as: chosen
          prompt: Choose a source whose printed level is captured
      - bind_permanent_property: {from: chosen, property: level, bind_as: selected_level}
      - security_place_stacked_card:
          carrier: source
          source: chosen
          of: you
          position: bottom
          face: down
      - gain_memory: {binding_value: selected_level}
"#;

#[test]
fn bound_source_card_level_is_captured_before_the_card_moves() {
    let mut lower = make_test_card("PROPERTY-LOWER", "Lower");
    lower.level = Some(4);
    let mut r = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .unwrap()
        .add_card(lower)
        .start();
    r.skip_mulligan();
    r.game.memory = 3;
    r.place_stack(0, &["PROPERTY-LOWER", "CARD-PROPERTY-HOST"]);
    let card = r.game.players[0].battle_area[0].card_sources[0].handle();
    r.fire_on_play(0, 0);
    let selection = r
        .pending_selection_view()
        .expect("explicit physical source choice");
    let action = *selection
        .valid_action_ids
        .iter()
        .find(|&&a| a != PASS)
        .unwrap();
    assert!(build_action_mask(&r.game, 0)[action as usize] > 0.5);
    r.execute_action(0, action).unwrap();
    r.auto_resolve().unwrap();
    assert_eq!(r.game.players[0].security.first().unwrap().handle(), card);
    assert_eq!(
        r.game.memory, 7,
        "selected printed level4, not carrier level5"
    );
}
