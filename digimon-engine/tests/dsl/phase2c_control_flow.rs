//! Phase 2c — control-flow step lowering (Optional; If follows).

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::step::run_steps;
use digimon_engine::effect_context::EffectContext;

/// `Optional(body)` always runs the inner body in Phase 2c. The opt-out UX
/// lands in Phase 2d alongside `ScheduleDelayed`.
#[test]
fn optional_runs_inner_body() {
    let card = make_test_card("T-OPT", "T-OPT");
    let mut runner = DebugRunner::builder()
        .add_card(card.clone())
        .hand(0, &["T-OPT"])
        .build();

    // A hand-card handle is sufficient — the body (GainMemory) doesn't
    // interact with any permanent.
    let src_card = runner.game.players[0].hand[0].handle();
    let memory_before = runner.game.memory;

    let steps = vec![CompiledStep::Optional(vec![
        CompiledStep::GainMemory(2),
    ])];
    let mut bindings = Bindings::new();

    {
        let mut ctx = EffectContext::new(&mut runner.game, src_card, None, 0);
        run_steps(&steps, &mut ctx, &mut bindings);
    }

    assert_eq!(
        runner.game.memory,
        memory_before + 2,
        "Optional body should always run in Phase 2c: memory should have increased by 2"
    );
}
