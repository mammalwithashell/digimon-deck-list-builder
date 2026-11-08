//! Exercise actual raw registries, not manually wrapped test callbacks.
use crate::{
    card_source::CardHandle, debug_runner::DebugRunner,
    dsl_cards::{bindings::Bindings, raw_rust::EngineRawRustRegistry,
        formula_registry::{FormulaExtensionFn, FormulaExtensionRegistry}},
    effect_context::{EffectContext, EffectReadContext},
    game::Game, permanent::PermanentHandle, trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe}, sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn game(marked: bool, nested: bool) -> Game {
    let mut game = DebugRunner::builder().memory(5).start().game;
    let mut context = TriggerContext::default();
    context.projected_reference_reads_guarded = marked;
    if nested {
        let mut parent = TriggerContext::default();
        parent.simultaneous_play_members.push(context);
        context = parent;
    }
    game.current_trigger_context = Some(context);
    game
}

#[test]
fn raw_step_registry_checks_at_invocation_before_mutation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let mut registry = EngineRawRustRegistry::new();
    registry.register_step("probe", move |ctx, _bindings| {
        seen.fetch_add(1, Ordering::SeqCst);
        ctx.game.memory += 2;
    });
    // Retain a callback before a marked context exists, then clone the registry.
    let callback = registry.step_fn("probe").unwrap();
    let cloned = registry.clone();
    for nested in [false, true] {
        let mut ordinary = game(false, nested);
        callback(&mut EffectContext::new(&mut ordinary, CardHandle(1), None, 0), &mut Bindings::default());
        assert_eq!(ordinary.memory, 7);
        for f in [callback.clone(), cloned.step_fn("probe").unwrap()] {
            let mut projected = game(true, nested);
            let before = calls.load(Ordering::SeqCst);
            let result = catch_unwind(AssertUnwindSafe(|| f(
                &mut EffectContext::new(&mut projected, CardHandle(1), None, 0), &mut Bindings::default())));
            assert!(result.is_err(), "raw step must reject before entering an opaque body");
            assert_eq!(calls.load(Ordering::SeqCst), before);
            assert_eq!(projected.memory, 5);
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(registry.step_fn("missing").is_none());
}

#[test]
fn raw_formula_registry_checks_prefetched_and_late_callbacks() {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let target = PermanentHandle { player: 1, index: 2 };
    let mut registry = EngineRawRustRegistry::new();
    registry.register_formula("probe", move |_, actual| {
        seen.fetch_add(1, Ordering::SeqCst);
        assert_eq!(actual, target); 17
    });
    let prefetched = registry.formula_fn("probe").unwrap();
    for nested in [false, true] {
        let ordinary = game(false, nested);
        assert_eq!(prefetched(&EffectReadContext::new(&ordinary, CardHandle(1), None, 0), target), 17);
        let projected = game(true, nested);
        for f in [prefetched.clone(), registry.clone().formula_fn("probe").unwrap()] {
            let before = calls.load(Ordering::SeqCst);
            assert!(catch_unwind(AssertUnwindSafe(|| f(
                &EffectReadContext::new(&projected, CardHandle(1), None, 0), target))).is_err());
            assert_eq!(calls.load(Ordering::SeqCst), before);
        }
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert!(registry.formula_fn("missing").is_none());
}

static EXTENSION_CALLS: AtomicUsize = AtomicUsize::new(0);
fn extension(_: &EffectContext<'_>, target: PermanentHandle) -> i32 {
    EXTENSION_CALLS.fetch_add(1, Ordering::SeqCst);
    assert_eq!(target, PermanentHandle { player: 1, index: 2 });
    23
}

#[test]
fn formula_extension_dispatch_rejects_before_pointer_body() {
    EXTENSION_CALLS.store(0, Ordering::SeqCst);
    let registry = FormulaExtensionRegistry::with_entries([("probe", extension as FormulaExtensionFn)]);
    let target = PermanentHandle { player: 1, index: 2 };
    for nested in [false, true] {
        let mut ordinary = game(false, nested);
        assert_eq!(registry.evaluate("probe", &EffectContext::new(&mut ordinary, CardHandle(1), None, 0), target), Some(23));
        let mut projected = game(true, nested);
        let before = EXTENSION_CALLS.load(Ordering::SeqCst);
        assert!(catch_unwind(AssertUnwindSafe(|| registry.clone().evaluate("probe",
            &EffectContext::new(&mut projected, CardHandle(1), None, 0), target))).is_err());
        assert_eq!(EXTENSION_CALLS.load(Ordering::SeqCst), before);
        assert_eq!(registry.evaluate("missing", &EffectContext::new(&mut projected, CardHandle(1), None, 0), target), None);
    }
    assert_eq!(EXTENSION_CALLS.load(Ordering::SeqCst), 2);
}
