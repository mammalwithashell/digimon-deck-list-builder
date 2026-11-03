//! A parked destination choice owns its trigger context; it must not leak
//! into the ambient game, and its permutation tail must retain that context.
use digimon_engine::{
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_steps},
    effect_context::EffectContext,
    trigger_context::TriggerContext,
};

fn exercise(clone_resume: bool, callbacks: bool, ambient: bool) {
    let spec: digimon_dsl::spec::CardSpec = serde_yml::from_str(r#"
card: TEST-ORDER-CONTEXT
name: Order Context
kind: option
color: [red]
cost: 0
effects:
  - when: main_from_hand
    process:
      - reveal_top_deck: { of: you, count: 2 }
      - order_remainder:
          of: you
          destinations: [deck_top, deck_bottom]
      - mark_security_face_up: { of: you, card: event_card }
"#).unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let digimon_dsl::compiled::CompiledClause::Triggered(clause) = &compiled.effects[0]
        else { panic!("expected triggered clause") };
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("A", "A"))
        .add_card(make_test_card("B", "B"))
        .hand(0, &["SRC"])
        .deck(0, &["A", "B"])
        .security(0, &["A", "B"])
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let event = runner.game.players[0].security[0].handle();
    let unrelated = runner.game.players[0].security[1].handle();
    let mut authored = TriggerContext::default();
    authored.event_card = Some(event).into();
    runner.game.current_trigger_context = Some(authored);
    run_steps(&clause.process,
        &mut EffectContext::new(&mut runner.game, source, None, 0),
        &mut Bindings::new());
    // Simulate the enclosing effect scope unwinding while its choice waits.
    let mut previous = TriggerContext::default();
    previous.event_card = Some(unrelated).into();
    runner.game.current_trigger_context = ambient.then_some(previous);
    let mut game = if clone_resume { runner.game.clone() } else { runner.game };
    let expected = ambient.then_some(unrelated);
    for choice in 0..3 {
        let selection = game.pending_selection.as_ref().expect("destination then two order picks");
        if choice == 0 { assert_eq!(selection.valid_action_ids.len(), 2); }
        let action = selection.valid_action_ids[0];
        let player = selection.selecting_player;
        assert!(digimon_engine::action::mask::build_action_mask(&game, player)[action as usize] > 0.0);
        if callbacks { game.pending_selection_resume = None; }
        game.resolve_selection(player, action).unwrap();
        assert_eq!(game.current_trigger_context.as_ref().and_then(|c| c.event_card.read()),
            expected, "destination/permutation must restore the caller's context");
        if choice < 2 {
            assert!(game.pending_selection.is_some());
            assert!(!game.players[0].face_up_security.contains(&event.0),
                "tail must wait for the whole permutation");
        }
    }
    assert!(game.pending_selection.is_none());
    assert!(game.pending_selection_resume.is_none());
    assert!(game.revealed_cards.is_empty());
    assert_eq!(game.players[0].deck.len(), 2);
    assert!(game.players[0].face_up_security.contains(&event.0),
        "tail must use its authored event, including after cloning");
    assert!(!game.players[0].face_up_security.contains(&unrelated.0),
        "the ambient event must not replace the parked effect's event");
}

#[test]
fn destination_choice_restores_empty_context() { exercise(false, false, false); }
#[test]
fn destination_choice_restores_nested_context() { exercise(false, false, true); }
#[test]
fn cloned_destination_choice_retains_authored_context() { exercise(true, false, false); }
#[test]
fn callback_destination_choice_restores_context() { exercise(false, true, false); }
#[test]
fn callback_destination_choice_restores_nested_context() { exercise(false, true, true); }
