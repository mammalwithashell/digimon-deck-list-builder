use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::replacement::ReplacementCause;

#[test]
fn replacement_body_can_cancel_would_delete() {
    let yaml = r#"
card: DSL-3B-CANCEL
name: Cancel Replacement
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    process:
      - cancel_replacement: {}
"#;
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().start();
    let handle = runner.place_on_field(0, "DSL-3B-CANCEL", Some(0));

    runner
        .game
        .delete_permanent_with_cause(handle, ReplacementCause::OpponentEffect);

    assert_eq!(runner.battle_area_size(0), 1);
    assert_eq!(runner.trash_size(0), 0);
}

#[test]
fn replacement_body_can_redirect_would_delete_to_hand() {
    let yaml = r#"
card: DSL-3B-REDIRECT
name: Redirect Replacement
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    process:
      - redirect_replacement: { zone: hand }
"#;
    let mut runner = DebugRunner::builder().from_dsl_yaml(yaml).unwrap().start();
    let handle = runner.place_on_field(0, "DSL-3B-REDIRECT", Some(0));

    runner
        .game
        .delete_permanent_with_cause(handle, ReplacementCause::OpponentEffect);

    assert_eq!(runner.battle_area_size(0), 0);
    assert_eq!(runner.trash_size(0), 0);
    assert_eq!(runner.hand_size(0), 1);
}

#[test]
fn replacement_body_with_nested_selection_resumes_and_sets_outcome() {
    let yaml = r#"
card: DSL-3B-NESTED
name: Nested Replacement
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - kind: replacement
    trigger: when_would_be_deleted
    process:
      - select_opponent_permanent:
          bind_as: helper
          filter: { name_contains: Helper }
          prompt: "Pick helper"
      - gain_memory: 1
      - cancel_replacement: {}
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("HELPER", "Helper"))
        .start();
    let original = runner.place_on_field(0, "DSL-3B-NESTED", Some(0));
    runner.place_on_field(1, "HELPER", Some(0));

    runner
        .game
        .delete_permanent_with_cause(original, ReplacementCause::OpponentEffect);

    let view = runner
        .pending_selection_view()
        .expect("replacement body parked helper selection");
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("selection resolves");

    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.battle_area_size(0), 1, "original remains");
    assert_eq!(runner.battle_area_size(1), 1, "helper selection resolved");
    assert_eq!(
        runner.memory(),
        1,
        "tail resumed after the nested selection"
    );
}
