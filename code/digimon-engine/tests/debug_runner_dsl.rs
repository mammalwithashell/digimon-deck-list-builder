use digimon_dsl::compiled::CompiledClause;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::events::GameEvent;
use digimon_engine::selection::SelectionKind;

fn choice_yaml() -> &'static str {
    r#"
card: DSL-RUNNER-CHOICE
name: Runner Choice
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - select_effect_choice:
          labels: ["A", "B"]
          bind_as: branch
          prompt: "Pick one"
      - gain_memory: 2
"#
}

fn security_add_this_option_to_hand_yaml() -> &'static str {
    r#"
card: DSL-SEC-HAND
name: Security Hand
kind: option
color: [red]
cost: 1
effects:
  - when: on_security
    process:
      - add_this_option_to_hand: {}
"#
}

#[test]
fn builder_loads_embedded_and_inline_dsl_cards_for_structural_assertions() {
    let runner = DebugRunner::builder()
        .dsl_card("ST2-13")
        .expect("embedded ST2-13")
        .from_dsl_yaml(choice_yaml())
        .expect("inline fixture")
        .build();

    assert_eq!(runner.compiled_card("ST2-13").unwrap().name, "Hammer Spark");
    assert!(matches!(
        runner.dsl_clause("DSL-RUNNER-CHOICE", 0),
        Some(CompiledClause::Triggered(_))
    ));
}

#[test]
fn selection_helpers_drive_effect_choice_branches() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(choice_yaml())
        .unwrap()
        .hand(0, &["DSL-RUNNER-CHOICE"])
        .start();

    runner.play(0, 0);

    assert!(runner.pending_selection().is_some());
    assert!(runner.pending_selection_view().is_some());
    assert_eq!(runner.pending_kind(), Some(SelectionKind::EffectChoice));
    assert!(!runner.pending_is_optional());
    assert_eq!(runner.pending_action_count(), 2);

    let before = runner.memory();
    runner.execute_branch(1).expect("branch resolves");
    assert!(runner.pending_selection().is_none());
    assert_eq!(runner.memory(), before + 2);
}

#[test]
fn execute_action_and_auto_resolve_handle_ordinary_selections() {
    let yaml = r#"
card: DSL-RUNNER-HAND
name: Runner Hand
kind: digimon
level: 3
color: [red]
cost: 0
dp: 2000
effects:
  - when: on_play
    process:
      - select_hand:
          of: you
          bind_as: pick
          filter: { name_contains: Filler }
          prompt: "Pick a hand card"
      - trash_from_hand_by_index: { of: you, hand_index: pick }
"#;
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["DSL-RUNNER-HAND", "FILLER"])
        .start();

    runner.play(0, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Hand));

    let view = runner.pending_selection_view().unwrap();
    runner
        .execute_action(view.selecting_player, view.valid_action_ids[0])
        .expect("hand pick resolves");
    assert_eq!(runner.trash_size(0), 1);

    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(yaml)
        .unwrap()
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["DSL-RUNNER-HAND", "FILLER"])
        .start();
    runner.play(0, 0);
    runner.auto_resolve().expect("auto resolves first action");
    assert_eq!(runner.trash_size(0), 1);
}

#[test]
fn event_helpers_checkpoint_and_filter_events() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("EVENT-SRC", "EventSrc"))
        .hand(0, &["EVENT-SRC"])
        .memory(5)
        .start();

    let checkpoint = runner.event_checkpoint();
    runner.play(0, 0);

    assert!(!runner.events_since(checkpoint).is_empty());
    let memory_events = runner.events_of_kind(checkpoint, |event| {
        matches!(event, GameEvent::MemoryChange { .. })
    });
    assert!(!memory_events.is_empty());
}

#[test]
fn security_dsl_adds_currently_resolving_option_to_hand() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(security_add_this_option_to_hand_yaml())
        .expect("inline security option DSL parses")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .security(1, &["DSL-SEC-HAND"])
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0, "revealed option left security");
    assert_eq!(runner.hand_size(1), 1, "revealed option moved to hand");
    assert_eq!(
        runner.trash_size(1),
        0,
        "revealed option must not be trashed"
    );
    assert!(
        runner.pending_selection().is_none(),
        "adding the currently resolving option to hand is deterministic"
    );
}
