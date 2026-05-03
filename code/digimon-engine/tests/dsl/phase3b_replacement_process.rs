use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::permanent::PermanentHandle;
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

#[test]
fn replacement_cancels_only_when_top_security_cost_is_paid() {
    let yaml = r#"
card: TEST-TRASH-SEC-PROTECT
name: Trash Security Protect
kind: digimon
color: [yellow]
level: 6
cost: 12
dp: 12000
traits: [TS]
effects:
  - kind: replacement
    timing: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: TS
        - security_count_gte: 1
    process:
      - trash_top_security_and_cancel_replacement: { of: you }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline replacement DSL compiles")
        .add_card(make_trait_card("TARGET", &["TS"]))
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .start();
    runner.place_on_field(0, "TEST-TRASH-SEC-PROTECT", Some(0));
    let target = runner.place_on_field(0, "TARGET", Some(0));

    runner.game.return_to_hand_from_effect(target, 1);

    assert!(permanent_exists(&runner, 0, "TARGET"));
    assert_eq!(runner.security_count(0), 0, "security cost was paid");
    assert!(permanent_exists(&runner, 0, "TEST-TRASH-SEC-PROTECT"));
}

#[test]
fn replacement_once_per_turn_skips_second_matching_subject() {
    let yaml = r#"
card: TEST-OPT-SEC-PROTECT
name: OPT Security Protect
kind: digimon
color: [yellow]
level: 6
cost: 12
dp: 12000
traits: [TS]
effects:
  - kind: replacement
    timing: when_would_leave_battle_area
    once_per_turn: true
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: TS
        - security_count_gte: 1
    process:
      - trash_top_security_and_cancel_replacement: { of: you }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline replacement DSL compiles")
        .add_card(make_trait_card("TARGET-A", &["TS"]))
        .add_card(make_trait_card("TARGET-B", &["TS"]))
        .add_card(make_test_card("SEC-A", "Security A"))
        .add_card(make_test_card("SEC-B", "Security B"))
        .security(0, &["SEC-A", "SEC-B"])
        .start();
    runner.place_on_field(0, "TEST-OPT-SEC-PROTECT", Some(0));
    let target_a = runner.place_on_field(0, "TARGET-A", Some(0));
    let target_b = runner.place_on_field(0, "TARGET-B", Some(0));

    runner.game.return_to_hand_from_effect(target_a, 1);
    assert!(permanent_exists(&runner, 0, "TARGET-A"));
    assert_eq!(
        runner.security_count(0),
        1,
        "first replacement activation pays one security"
    );

    runner.game.return_to_hand_from_effect(target_b, 1);

    assert!(
        !permanent_exists(&runner, 0, "TARGET-B"),
        "second matching subject in the same turn must not be protected"
    );
    assert_eq!(
        runner.security_count(0),
        1,
        "once-per-turn replacement must not pay a second security"
    );
}

#[test]
fn replacement_cancels_only_when_other_sourceless_digimon_is_placed_in_security() {
    let yaml = r#"
card: TEST-PLACE-SEC-PROTECT
name: Place Security Protect
kind: digimon
color: [yellow]
level: 6
cost: 12
dp: 12000
traits: [TS]
effects:
  - kind: replacement
    timing: when_would_leave_battle_area
    active_when:
      all_of:
        - replacement_subject_is_mine: true
        - trait_has: TS
        - replacement_cause: opponent_effect
    process:
      - select_own_permanent:
          bind_as: cost_body
          filter:
            all_of:
              - kind: digimon
              - materials_count_lte: 0
              - not_in_binding: replacement_subject
          prompt: "Place another sourceless Digimon as bottom security"
      - place_permanent_bottom_security_and_cancel_replacement:
          of: you
          target: cost_body
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .expect("inline replacement DSL compiles")
        .add_card(make_trait_card("TARGET", &["TS"]))
        .add_card(make_test_card("COST-BODY", "Cost Body"))
        .add_card(make_test_card("STACKED-SOURCE", "Stacked Source"))
        .add_card(make_test_card("STACKED-BODY", "Stacked Body"))
        .start();
    runner.place_on_field(0, "TEST-PLACE-SEC-PROTECT", Some(0));
    let target = runner.place_on_field(0, "TARGET", Some(0));
    let cost_body = runner.place_on_field(0, "COST-BODY", Some(0));
    let stacked_body = runner.place_stack(0, &["STACKED-SOURCE", "STACKED-BODY"]);

    runner.game.return_to_hand_from_effect(target, 1);
    let view = runner
        .pending_selection_view()
        .expect("choose cost body selection");
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_attack(target.player as u16, target.index as u16)),
        "replacement subject must not be selectable as its own 'other' cost"
    );
    assert!(
        !view
            .valid_action_ids
            .contains(&encode_permanent(stacked_body)),
        "Digimon with digivolution cards must not be selectable as sourceless cost"
    );
    runner
        .execute_action(view.selecting_player, encode_permanent(cost_body))
        .expect("choose cost body");

    assert!(permanent_exists(&runner, 0, "TARGET"));
    assert!(!permanent_exists(&runner, 0, "COST-BODY"));
    assert_eq!(
        runner.game.players[0]
            .security
            .first()
            .unwrap()
            .card_id(&runner.game.card_data),
        "COST-BODY",
        "cost body was placed as bottom security"
    );
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(handle.player as u16, handle.index as u16)
}

fn make_trait_card(card_id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

fn permanent_exists(runner: &DebugRunner, player: usize, card_id: &str) -> bool {
    runner.game.players[player]
        .battle_area
        .iter()
        .any(|permanent| permanent.top_card().card_id(&runner.game.card_data) == card_id)
}
