use std::sync::Arc;

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps_with_runtime, StepRuntime};
use digimon_engine::effect_context::EffectContext;

#[test]
fn raw_rust_step_dispatches_registered_function() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("gain_two_and_mark", |ctx, bindings| {
        ctx.gain_memory(2);
        bindings.insert_literal("called", 1);
    });
    let runtime = StepRuntime::new(Arc::new(registry));

    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-STEP", "Raw Step"))
        .hand(0, &["RAW-STEP"])
        .memory(0)
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::RawRust {
        fn_name: "gain_two_and_mark".into(),
        consumes: vec![],
        binds: vec![],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);

    assert_eq!(ctx.game.memory, 2);
    assert_eq!(bindings.get_literal("called"), Some(1));
}

#[test]
fn missing_raw_rust_step_is_a_noop() {
    let runtime = StepRuntime::new(Arc::new(EngineRawRustRegistry::new()));
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("RAW-STEP", "Raw Step"))
        .hand(0, &["RAW-STEP"])
        .memory(0)
        .build();
    let source = runner.game.players[0].hand[0].handle();
    let mut ctx = EffectContext::new(&mut runner.game, source, None, 0);
    let mut bindings = Bindings::new();
    let steps = vec![CompiledStep::RawRust {
        fn_name: "missing".into(),
        consumes: vec![],
        binds: vec![],
    }];

    run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime);

    assert_eq!(ctx.game.memory, 0);
}
