//! Phase 2f4 Task 3 — DSL `ScheduleDelayed` step lowering.
//!
//! Drives `CompiledStep::ScheduleDelayed { when, body }` through `run_steps`
//! and asserts the body is queued (not fired immediately) and fires when the
//! engine reaches the matching timing boundary. This is the IR-level
//! analogue of Task 1's primitive test and Task 2's wiring test.

use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

#[test]
fn schedule_delayed_step_queues_body_for_end_of_your_turn() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .start();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].handle();

    let body = vec![CompiledStep::GainMemory(1)];
    let steps = vec![CompiledStep::ScheduleDelayed {
        when: CompiledTiming::EndOfYourTurn,
        body,
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    // Body should NOT have run yet — it's queued.
    assert_eq!(
        runner.game.memory, 0,
        "body must be queued, not fired immediately"
    );
    assert_eq!(
        runner.game.scheduled_effects.len(),
        1,
        "exactly one entry queued"
    );

    // Drive end-of-turn — Task 2's wiring fires the queue. Memory accounting
    // mirrors Task 2's wiring test: GainMemory(1) lands during
    // `fire_end_of_your_turn` (memory: 0 → +1), then turn rotation flips the
    // seesaw (memory: +1 → -1).
    runner.end_turn();

    assert_eq!(
        runner.game.memory, -1,
        "scheduled body ran during end-of-turn transition \
         (memory: 0 → +1 by GainMemory → -1 by seesaw flip)"
    );
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "queue drained after firing"
    );
}

/// `Delayed` is a virtual timing — `compiled_timing_to_engine` returns None
/// for it. The step lowering should silently no-op (no panic, no queue
/// entry) when the timing is unmappable, matching the engine convention for
/// degenerate-but-defensible inputs.
#[test]
fn schedule_delayed_step_with_unmappable_timing_silently_noops() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .start();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].handle();

    let steps = vec![CompiledStep::ScheduleDelayed {
        when: CompiledTiming::Delayed, // maps to None in compiled_timing_to_engine
        body: vec![CompiledStep::GainMemory(1)],
    }];

    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        let mut bindings = Bindings::new();
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(runner.game.memory, 0, "no immediate effect");
    assert!(
        runner.game.scheduled_effects.is_empty(),
        "no entry queued for unmappable timing"
    );
}
