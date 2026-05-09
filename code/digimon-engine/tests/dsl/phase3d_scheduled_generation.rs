use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{EffectTiming, GamePhase};

fn runner() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .hand(0, &["SRC"])
        .build();
    runner.game.turn_order = vec![0, 1];
    runner.game.turn_player_idx = 0;
    runner.game.turn_count = 1;
    runner.game.current_phase = GamePhase::Main;
    runner.game.memory = 0;
    runner
}

fn set_turn(runner: &mut DebugRunner, turn_count: u16, turn_player: u8) {
    runner.game.turn_count = turn_count;
    runner.game.turn_player_idx = runner
        .game
        .turn_order
        .iter()
        .position(|p| *p == turn_player)
        .expect("turn player in order");
}

fn schedule(runner: &mut DebugRunner, when: CompiledTiming, body: Vec<CompiledStep>) {
    let src = runner.game.players[0].hand[0].handle();
    let steps = vec![CompiledStep::ScheduleDelayed { when, body }];
    let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
    let mut bindings = Bindings::new();
    run_steps(&steps, &mut ctx, &mut bindings);
}

#[test]
fn end_of_your_next_turn_skips_current_end_turn() {
    let mut runner = runner();
    schedule(
        &mut runner,
        CompiledTiming::EndOfYourNextTurn,
        vec![CompiledStep::GainMemory(3)],
    );

    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourNextTurn,
    );
    assert_eq!(runner.game.memory, 0);
    assert_eq!(runner.game.scheduled_effects.len(), 1);

    set_turn(&mut runner, 2, 1);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourNextTurn,
    );
    assert_eq!(runner.game.memory, 0, "opponent turn is not your next turn");

    set_turn(&mut runner, 3, 0);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourNextTurn,
    );
    assert_eq!(runner.game.memory, 3);
    assert!(runner.game.scheduled_effects.is_empty());
}

#[test]
fn end_of_opponents_next_turn_skips_current_opponent_end_turn() {
    let mut runner = runner();
    set_turn(&mut runner, 2, 1);
    schedule(
        &mut runner,
        CompiledTiming::EndOfOpponentsNextTurn,
        vec![CompiledStep::GainMemory(4)],
    );

    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfOpponentsNextTurn,
    );
    assert_eq!(runner.game.memory, 0);

    set_turn(&mut runner, 3, 0);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfOpponentsNextTurn,
    );
    assert_eq!(runner.game.memory, 0, "own turn is not an opponent turn");

    set_turn(&mut runner, 4, 1);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfOpponentsNextTurn,
    );
    assert_eq!(
        runner.game.memory, -4,
        "scheduled body gains memory for its controller during the opponent's turn"
    );
    assert!(runner.game.scheduled_effects.is_empty());
}

#[test]
fn until_next_unsuspend_skips_current_unsuspend_window() {
    let mut runner = runner();
    schedule(
        &mut runner,
        CompiledTiming::UntilNextUnsuspend,
        vec![CompiledStep::GainMemory(5)],
    );

    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::UntilNextUnsuspend,
    );
    assert_eq!(runner.game.memory, 0);

    set_turn(&mut runner, 2, 1);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::UntilNextUnsuspend,
    );
    assert_eq!(runner.game.memory, 0, "opponent unsuspend is not yours");

    set_turn(&mut runner, 3, 0);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::UntilNextUnsuspend,
    );
    assert_eq!(runner.game.memory, 5);
    assert!(runner.game.scheduled_effects.is_empty());
}
