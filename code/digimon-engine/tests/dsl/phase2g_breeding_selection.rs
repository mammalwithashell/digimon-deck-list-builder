//! Phase 2g: DSL breeding permanent selections park and resume process tails.

use std::sync::{Arc, Mutex};

use digimon_dsl::compiled::CompiledStep;
use digimon_engine::action::space::encode_breeding_select;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::dsl_cards::bindings::Bindings;
use digimon_engine::dsl_cards::raw_rust::EngineRawRustRegistry;
use digimon_engine::dsl_cards::step::{run_steps, run_steps_with_runtime, RunOutcome, StepRuntime};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::selection::BreedingPermanentSelectionRef;

#[test]
fn dsl_select_breeding_permanent_binds_target() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    runner.place_in_breeding(p0, "KING-DRASIL");

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose breeding".to_string(),
        then: vec![CompiledStep::GainMemory(1)],
    }];

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps(&steps, &mut ctx, &mut bindings)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");

    assert_eq!(runner.game.memory, 1);
}

#[test]
fn select_own_breeding_permanent_binds_selected_ref() {
    let mut runner = DebugRunner::builder()
        .add_card(make_test_card("SRC", "Source"))
        .add_card(make_test_card("KING-DRASIL", "King Drasil"))
        .start();
    let p0 = 0;
    let source = runner.place_on_field(p0, "SRC", Some(0));
    let source_card = runner.top_card(source);
    let breeding = runner.place_in_breeding(p0, "KING-DRASIL");
    let expected = BreedingPermanentSelectionRef {
        player: breeding.player,
        card: breeding.card,
    };

    let steps = vec![CompiledStep::SelectOwnBreedingPermanent {
        bind_as: Some("breeding_target".to_string()),
        prompt: "Choose breeding".to_string(),
        then: vec![CompiledStep::RawRust {
            fn_name: "assert_breeding_binding".to_string(),
            consumes: vec!["breeding_target".to_string()],
            binds: vec![],
        }],
    }];

    let seen = Arc::new(Mutex::new(None));
    let seen_slot = Arc::clone(&seen);
    let mut raw = EngineRawRustRegistry::new();
    raw.register_step("assert_breeding_binding", move |_ctx, bindings| {
        *seen_slot.lock().unwrap() = bindings.get_breeding_permanent_ref("breeding_target");
    });
    let runtime = StepRuntime::new(Arc::new(raw));

    let mut bindings = Bindings::new();
    let outcome = {
        let mut ctx = EffectContext::new(&mut runner.game, source_card, Some(source), p0);
        run_steps_with_runtime(&steps, &mut ctx, &mut bindings, &runtime)
    };
    assert_eq!(outcome, RunOutcome::Parked);

    runner
        .game
        .resolve_selection(p0, encode_breeding_select(p0).unwrap())
        .expect("pick breeding");

    assert_eq!(*seen.lock().unwrap(), Some(expected));
}
