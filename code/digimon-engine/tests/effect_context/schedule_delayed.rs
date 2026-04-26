//! Phase 2f4 Task 1 — `ScheduledEffect` queue + `schedule_delayed` primitive.
//!
//! Verifies the engine-level contract:
//!   1. A scheduled body fires when `fire_scheduled_for_timing` is called for
//!      its registered `when:` timing (memory delta proves body ran).
//!   2. The queue is one-shot — re-calling `fire_scheduled_for_timing` for the
//!      same timing does NOT fire the body again (drained).
//!   3. Calling `fire_scheduled_for_timing` for a different timing does NOT
//!      drain the queued effect; the effect remains queued and fires when its
//!      registered timing eventually arrives.

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;
use digimon_engine::scheduled_effects::fire_scheduled_for_timing;

#[test]
fn scheduled_effect_fires_on_matching_timing() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .start();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.schedule_delayed(
            EffectTiming::EndOfYourTurn,
            vec![CompiledStep::GainMemory(1)],
            Bindings::new(),
        );
    }
    assert_eq!(runner.game.memory, 0, "not fired before timing");
    fire_scheduled_for_timing(&mut runner.game, EffectTiming::EndOfYourTurn);
    assert_eq!(runner.game.memory, 1, "fired exactly once");
    fire_scheduled_for_timing(&mut runner.game, EffectTiming::EndOfYourTurn);
    assert_eq!(runner.game.memory, 1, "drained — does not fire twice");
}

#[test]
fn scheduled_effect_with_other_timing_does_not_fire() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("TST", "S"))
        .hand(0, &["TST"])
        .start();
    runner.game.memory = 0;
    let src = runner.game.players[0].hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, src, None, 0);
        ctx.schedule_delayed(
            EffectTiming::EndOfYourTurn,
            vec![CompiledStep::GainMemory(1)],
            Bindings::new(),
        );
    }
    fire_scheduled_for_timing(&mut runner.game, EffectTiming::EndOfOpponentsTurn);
    assert_eq!(runner.game.memory, 0, "wrong timing — should NOT fire");
    // Still queued for the right timing:
    fire_scheduled_for_timing(&mut runner.game, EffectTiming::EndOfYourTurn);
    assert_eq!(runner.game.memory, 1, "fires when correct timing arrives");
}
