use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{EffectTiming, GamePhase};

#[test]
fn scheduled_drain_resumes_after_selection_and_continues_remaining_effects() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("TARGET", "Target"))
        .hand(0, &["SRC"])
        .build();
    runner.game.turn_order = vec![0, 1];
    runner.game.turn_player_idx = 0;
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.game.memory = 0;
    runner.place_on_field(0, "TARGET", None);

    let src = runner.game.players[0].hand[0].handle();
    let steps = vec![
        CompiledStep::ScheduleDelayed {
            when: CompiledTiming::EndOfYourTurn,
            body: vec![
                CompiledStep::SelectEffectChoice {
                    labels: vec!["first".to_string()],
                    bind_as: None,
                    prompt: "Choose one".to_string(),
                    prompt_key: None,
                },
                CompiledStep::GainMemory(1),
            ],
        },
        CompiledStep::ScheduleDelayed {
            when: CompiledTiming::EndOfYourTurn,
            body: vec![CompiledStep::GainMemory(2)],
        },
    ];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.scheduled_effects.len(), 2);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );

    assert!(runner.game.pending_selection.is_some());
    assert!(runner.game.scheduled_drain_tail.is_some());
    assert_eq!(runner.game.memory, 0);

    let (selecting_player, action_id) = {
        let pending = runner.game.pending_selection.as_ref().unwrap();
        (pending.selecting_player, pending.valid_action_ids[0])
    };
    runner
        .game
        .resolve_selection(selecting_player, action_id)
        .expect("resolve scheduled selection");

    assert!(runner.game.pending_selection.is_none());
    assert!(runner.game.scheduled_drain_tail.is_none());
    assert!(runner.game.scheduled_effects.is_empty());
    assert_eq!(runner.game.memory, 3);
}
