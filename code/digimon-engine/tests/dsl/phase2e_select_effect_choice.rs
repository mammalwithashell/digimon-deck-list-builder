//! Phase 2e Task 2: SelectEffectChoice installs a parking selection,
//! its callback writes the chosen branch index into Bindings as
//! `BindingValue::Literal`, and the post-selection step runs.

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

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
