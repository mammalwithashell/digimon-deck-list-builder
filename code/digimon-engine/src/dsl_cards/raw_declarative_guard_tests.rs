//! Returned callback checks are distinct from factory-body/capture safety.
use super::*;
use crate::{debug_runner::DebugRunner, dsl_cards::DslCardEffect, effect::CardEffect,
    game::Game, trigger_context::TriggerContext};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::atomic::{AtomicUsize, Ordering}};

fn mark(game: &mut Game, mode: u8) {
    let mut context = TriggerContext::default();
    context.projected_reference_reads_guarded = mode != 0;
    if mode == 2 {
        let mut parent = TriggerContext::default();
        let mut nested = TriggerContext::default();
        nested.simultaneous_play_members.push(context);
        parent.simultaneous_play_members.push(nested);
        context = parent;
    }
    game.current_trigger_context = Some(context);
}

fn register(registry: &mut EngineRawRustRegistry, answer: bool, calls: Arc<AtomicUsize>, factories: Arc<AtomicUsize>) {
    registry.register_declarative("probe", move |card| {
        // The factory has no Game parameter and still executes before wrapping.
        // This counter prevents mistaking callback rejection for factory rejection.
        factories.fetch_add(1, Ordering::SeqCst);
        let read_calls = calls.clone();
        let body_calls = calls.clone();
        let pay_calls = calls.clone();
        vec![
            Effect::declarative(card).name("generated read")
                .condition(move |ctx| {
                    assert_eq!(ctx.source_card, card);
                    read_calls.fetch_add(1, Ordering::SeqCst);
                    answer
                }).build(),
            Effect::on_play(card).name("generated body")
                .process(move |ctx| {
                    assert_eq!(ctx.source_card, card);
                    body_calls.fetch_add(1, Ordering::SeqCst);
                    ctx.game.memory += 2;
                }).build(),
            Effect::before_pay_cost(card).name("generated payment")
                .pay_cost_fn(move |ctx| {
                    assert_eq!(ctx.source_card, card);
                    pay_calls.fetch_add(1, Ordering::SeqCst);
                    answer
                }).build(),
        ]
    });
}

fn invoke(mut effects: Vec<Effect>, game: &mut Game, card: CardHandle, seat: u8, mode: u8, answer: bool, calls: &AtomicUsize) {
    assert_eq!(effects.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["generated read", "generated body", "generated payment"]);
    assert!(effects[1].on_play);
    let before = calls.load(Ordering::SeqCst);
    let memory = game.memory;
    macro_rules! verify { ($expr:expr, $expected:expr) => {{
        let result = catch_unwind(AssertUnwindSafe(|| $expr));
        if mode != 0 {
            let panic = result.expect_err("generated callback must reject before its body");
            let message = panic.downcast_ref::<String>().map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied()).unwrap_or("");
            assert!(message.contains("closed-list projected context reached unproven callback"), "{message}");
        } else { assert_eq!(result.unwrap(), $expected); }
    }}; }
    // Retain/move the actual stored callback values, independent of the factory.
    let condition = effects[0].condition.as_ref().unwrap().clone();
    let process = effects[1].process.take().unwrap();
    let payment = effects[2].pay_cost_fn.take().unwrap();
    verify!(condition(&EffectReadContext::new(game, card, None, seat)), answer);
    verify!(process(&mut EffectContext::new(game, card, None, seat)), ());
    verify!(payment(&mut EffectContext::new(game, card, None, seat)), answer);
    assert_eq!(calls.load(Ordering::SeqCst) - before, if mode == 0 { 3 } else { 0 });
    assert_eq!(game.memory, memory + if mode == 0 { 2 } else { 0 });
}

#[test]
fn prefetched_cloned_and_replaced_raw_factories_guard_all_emitted_entries() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factories = Arc::new(AtomicUsize::new(0));
    let mut game = DebugRunner::builder().memory(5).start().game;
    for answer in [false, true] {
        let mut registry = EngineRawRustRegistry::new();
        register(&mut registry, answer, calls.clone(), factories.clone());
        let prefetched = registry.declarative_fn("probe").unwrap();
        let cloned = registry.clone();
        assert!(Arc::ptr_eq(&prefetched, &cloned.declarative_fn("probe").unwrap()));
        for seat in 0..2 { for mode in 0..3 { for card in [CardHandle(73), CardHandle(74)] {
            // Build one set before the marker exists, then invoke after marking.
            mark(&mut game, 0);
            let retained = prefetched(card);
            mark(&mut game, mode);
            register(&mut registry, !answer, calls.clone(), factories.clone());
            invoke(retained, &mut game, card, seat, mode, answer, &calls);
            invoke(cloned.declarative_fn("probe").unwrap()(card), &mut game, card, seat, mode, answer, &calls);
            invoke(registry.declarative_fn("probe").unwrap()(card), &mut game, card, seat, mode, !answer, &calls);
        }}}
        assert_eq!(registry.registered_fn_count(), 1);
    }
    assert_eq!(factories.load(Ordering::SeqCst), 72, "factory execution is not guarded by current-context callback checks");
    assert_eq!(calls.load(Ordering::SeqCst), 72);
}

#[test]
fn direct_compiled_raw_clause_preserves_returned_callback_guards() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(
        "card: RAW-GUARD\nname: Raw guard\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: probe\n").unwrap();
    let compiled = digimon_dsl::compile::compile(&spec).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let factories = Arc::new(AtomicUsize::new(0));
    let mut registry = EngineRawRustRegistry::new();
    register(&mut registry, true, calls.clone(), factories.clone());
    // Deliberately bypass the outer CardEffectRegistry adapter. This exercises
    // the raw declarative boundary, not the already-tested outer wrapper.
    let factory = DslCardEffect::new_with_raw(Arc::new(compiled), Arc::new(registry));
    let mut game = DebugRunner::builder().memory(5).start().game;
    for seat in 0..2 { for mode in 0..3 {
        mark(&mut game, mode);
        invoke(factory.effects(CardHandle(81)), &mut game, CardHandle(81), seat, mode, true, &calls);
    }}
    assert_eq!(factories.load(Ordering::SeqCst), 6);
    assert_eq!(calls.load(Ordering::SeqCst), 6);
}

#[test]
fn empty_and_metadata_only_declaratives_keep_shape_and_lookup_semantics() {
    let mut registry = EngineRawRustRegistry::new();
    registry.register_declarative("empty", |_| vec![]);
    registry.register_declarative("metadata", |card| vec![Effect::on_play(card).name("metadata only").build()]);
    let cloned = registry.clone();
    assert!(cloned.declarative_fn("empty").unwrap()(CardHandle(2)).is_empty());
    let effects = cloned.declarative_fn("metadata").unwrap()(CardHandle(3));
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].name, "metadata only");
    assert!(effects[0].on_play && effects[0].process.is_none());
    assert!(cloned.declarative_fn("missing").is_none());
    assert!(cloned.contains_fn("metadata"));
    assert_eq!(cloned.registered_fn_count(), 2);
}
