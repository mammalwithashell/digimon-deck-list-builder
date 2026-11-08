//! Selection publication belongs to the resolving callback and its enclosing
//! continuations; it must not survive into an unrelated effect activation.
use digimon_engine::{
    action::{build_action_mask, space::PASS},
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{bindings::Bindings, step::run_steps},
    effect_context::EffectContext,
};
use digimon_dsl::compiled::{CompiledPlayerRef, CompiledPredicate, CompiledStep};

fn setup() -> DebugRunner {
    DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("PICK", "Picked card"))
        .hand(0, &["SRC", "PICK"])
        .hand(1, &["SRC", "PICK"])
        .deck(0, &["PICK"; 12])
        .deck(1, &["PICK"; 12])
        .memory(5)
        .start()
}

fn pick(name: &str) -> CompiledStep {
    CompiledStep::SelectHand {
        of: CompiledPlayerRef::You,
        filter: CompiledPredicate::default(),
        bind_as: Some(name.into()),
        prompt: "Choose a card".into(),
        prompt_key: None,
        optional: false,
        cost: false,
        then: vec![],
    }
}

fn exercise(callback: bool, clone_resume: bool) {
    let mut r = setup();
    let source = r.game.players[1].hand[0].handle();
    let before = r.game.players[1].hand.len();
    // The second step is a real enclosing tail. It must still receive the
    // selected binding before the publication channel is restored on exit.
    let steps = vec![pick("private_pick"), CompiledStep::If {
        condition: CompiledPredicate { binding_exists: Some("private_pick".into()), ..Default::default() },
        then: vec![CompiledStep::Draw { of: CompiledPlayerRef::You, count: 1 }],
        else_branch: vec![],
    }];
    run_steps(&steps, &mut EffectContext::new(&mut r.game, source, None, 1), &mut Bindings::new());
    if callback { r.game.pending_selection_resume = None; }
    if clone_resume && !callback { r.game = r.game.clone(); }
    let view = r.pending_selection_view().expect("selection is exposed");
    let action = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
    assert_eq!(build_action_mask(&r.game, view.selecting_player)[action as usize], 1.0);
    r.execute_action(view.selecting_player, action).unwrap();
    // Raw callbacks are intentionally not cloneable. Verify their completed
    // publication state on a clone after executing the actual callback.
    if clone_resume && callback { r.game = r.game.clone(); }
    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.game.players[1].hand.len(), before + 1, "same-resolution enclosing tail lost its selected binding");
    assert!(r.game.dsl_resolved_tail_bindings.is_none(), "completed selection leaked private publication into a later activation");
}

#[test] fn typed_publication_ends_with_resolution() { exercise(false, false); }
#[test] fn cloned_typed_publication_ends_with_resolution() { exercise(false, true); }
#[test] fn callback_publication_ends_with_resolution() { exercise(true, false); }
#[test] fn resolved_callback_publication_not_retained_by_clone() { exercise(true, true); }

#[test]
fn nested_resolution_restores_parent_publication() {
    let mut r = setup();
    let source = r.game.players[0].hand[0].handle();
    run_steps(&[pick("parent")], &mut EffectContext::new(&mut r.game, source, None, 0), &mut Bindings::new());
    r.game.pending_selection_resume = None;
    r.game.pending_selection.as_mut().unwrap().callback = Box::new(move |game, _| {
        let mut parent = Bindings::new();
        parent.insert_literal("parent_publication", 73);
        game.dsl_resolved_tail_bindings = Some(parent);
        let child_source = game.players[1].hand[0].handle();
        run_steps(&[pick("child")], &mut EffectContext::new(game, child_source, None, 1), &mut Bindings::new());
        let child = game.pending_selection.as_ref().unwrap();
        let player = child.selecting_player;
        let action = child.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
        assert_eq!(build_action_mask(game, player)[action as usize], 1.0);
        game.decode_action(action, player);
        assert_eq!(game.dsl_resolved_tail_bindings.as_ref().and_then(|b| b.get_literal("parent_publication")), Some(73), "nested resolution replaced its caller's publication");
    });
    let view = r.pending_selection_view().unwrap();
    let action = view.valid_action_ids.iter().copied().find(|a| *a != PASS).unwrap();
    assert_eq!(build_action_mask(&r.game, view.selecting_player)[action as usize], 1.0);
    r.execute_action(view.selecting_player, action).unwrap();
    assert!(r.game.dsl_resolved_tail_bindings.is_none(), "outer resolution left publication behind");
}
