use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn hand_selection_binding_carries_selected_player_into_tail_steps() {
    let yaml = r#"
card: DSL-3A-BINDING
name: Binding Owner
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: opponent
          bind_as: pick
          filter: { name_contains: Opponent Card }
          prompt: "Pick opponent-owned hand card"
      - trash_from_hand_by_index: { of: opponent, hand_index: pick }
"#;

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("P1-HAND", "Opponent Card"))
        .hand(0, &["DSL-3A-BINDING"])
        .hand(1, &["P1-HAND"])
        .start();

    runner.play(0, 0);
    let view = runner.pending_selection_view().expect("opponent hand pick");
    assert_eq!(view.selecting_player, 0);

    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("selection resolves");

    assert_eq!(runner.hand_size(1), 0);
    assert_eq!(runner.trash_size(1), 1);
    assert_eq!(runner.trash_size(0), 0);
}
