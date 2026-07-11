//! Phase 2e Task 2: SelectEffectChoice installs a parking selection,
//! its callback writes the chosen branch index into Bindings as
//! `BindingValue::Literal`, and the post-selection step runs.

use digimon_dsl::compiled::{
    CompiledBindingCompare, CompiledBindingRef, CompiledFormula, CompiledModifierValue,
    CompiledPredicate, CompiledStep,
};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

fn resolve_pending_index(runner: &mut DebugRunner, index: usize) {
    let (action_id, selecting_player) = {
        let pending = runner
            .game
            .pending_selection
            .as_ref()
            .expect("pending selection");
        (pending.valid_action_ids[index], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve selection");
}

#[test]
fn select_effect_choice_binds_picked_index() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .hand(0, &["SRC"])
        .build();

    let src_card = runner.game.players[0].hand[0].handle();

    let steps = vec![
        CompiledStep::SelectEffectChoice {
            labels: vec!["A".to_string(), "B".to_string()],
            bind_as: Some("branch".to_string()),
            prompt: "Pick A or B".to_string(),
            prompt_key: None,
        },
        // Sentinel: gain memory so the test can confirm the post-select
        // tail ran. Branch-specific behavior is exercised by the end-to-end
        // test in Task 9 once the equals predicate lands; until then we
        // just confirm the callback fires and the tail executes.
        CompiledStep::GainMemory(1),
    ];

    let memory_before = runner.game.memory;

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // SelectEffectChoice parked — the GainMemory tail should not have run yet.
    assert!(
        runner.game.pending_selection.is_some(),
        "select_effect_choice must install a pending selection"
    );
    assert_eq!(
        runner.game.memory, memory_before,
        "tail must not run before the choice is resolved"
    );

    // Resolve by picking branch 1 ("B").
    let (action_id, selecting_player) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        // labels[1] is the second action_id in the list; the engine
        // guarantees `valid_action_ids[i]` corresponds to label index i.
        (pending.valid_action_ids[1], pending.selecting_player)
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve");

    assert!(runner.game.pending_selection.is_none());
    assert_eq!(
        runner.game.memory,
        memory_before + 1,
        "tail must run after resolution"
    );
}

fn mode_is(n: i64) -> CompiledPredicate {
    CompiledPredicate {
        equals: Some(vec![
            CompiledBindingCompare::Binding("mode".to_string()),
            CompiledBindingCompare::Literal(n),
        ]),
        ..CompiledPredicate::default()
    }
}

#[test]
fn repeat_effect_choice_repeats_count_and_resumes_after_nested_selection() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "SRC"))
        .add_card(make_test_card("ALLY", "ALLY"))
        .hand(0, &["SRC"])
        .memory(5)
        .start();

    let ally = runner.place_on_field(0, "ALLY", None);
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::RepeatEffectChoice {
        count: CompiledFormula::Literal(2),
        labels: vec!["Buff ally".to_string(), "Gain memory".to_string()],
        bind_as: Some("mode".to_string()),
        prompt: "Choose an effect to activate".to_string(),
        prompt_key: None,
        body: vec![
            CompiledStep::If {
                condition: mode_is(0),
                then: vec![
                    CompiledStep::SelectOwnPermanent {
                        filter: CompiledPredicate {
                            name_is: Some("ALLY".to_string()),
                            ..CompiledPredicate::default()
                        },
                        bind_as: Some("target".to_string()),
                        selector: None,
                        prompt: "Choose ALLY".to_string(),
                        prompt_key: None,
                        optional: false,
                        continue_on_decline: false,
                        then: vec![],
                    },
                    CompiledStep::AddDpModifier {
                        target: CompiledBindingRef::Named("target".to_string()),
                        value: CompiledModifierValue::Literal(1000),
                        expiry: "end_of_turn".to_string(),
                    },
                ],
                else_branch: vec![],
            },
            CompiledStep::If {
                condition: mode_is(1),
                then: vec![CompiledStep::GainMemory(3)],
                else_branch: vec![],
            },
        ],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert!(
        runner.game.pending_selection.is_some(),
        "first repeated effect-choice prompt should park"
    );

    resolve_pending_index(&mut runner, 0);
    assert!(
        runner.game.pending_selection.is_some(),
        "first choice body should park on its nested target prompt"
    );

    resolve_pending_index(&mut runner, 0);
    assert_eq!(
        runner.game.effective_dp(ally),
        Some(3000),
        "first mode should resolve before the next repeated prompt"
    );
    assert!(
        runner.game.pending_selection.is_some(),
        "second repeated effect-choice prompt should be installed"
    );

    resolve_pending_index(&mut runner, 1);
    assert!(
        runner.game.pending_selection.is_none(),
        "repeat loop should finish after the second choice"
    );
    assert_eq!(
        runner.game.memory,
        memory_before + 3,
        "second mode should resolve after the repeated prompt"
    );
}
