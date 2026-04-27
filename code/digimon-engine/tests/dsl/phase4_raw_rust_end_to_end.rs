use std::sync::Arc;

use digimon_dsl::compiled::{CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::EffectTiming;

#[test]
fn scheduled_body_preserves_raw_runtime() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("gain_three", |ctx, _bindings| {
        ctx.gain_memory(3);
    });
    let runtime = StepRuntime::new(Arc::new(registry));

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-SCHEDULE", "Raw Schedule"))
        .hand(0, &["RAW-SCHEDULE"])
        .memory(0)
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::ScheduleDelayed {
        when: CompiledTiming::EndOfYourTurn,
        body: vec![CompiledStep::RawRust {
            fn_name: "gain_three".into(),
            consumes: vec![],
            binds: vec![],
        }],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);
    digimon_engine::scheduled_effects::fire_scheduled_for_timing(
        &mut runner.game,
        EffectTiming::EndOfYourTurn,
    );

    assert_eq!(runner.game.memory, 3);
}
