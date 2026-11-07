//! A context-local snapshot cannot authorize opaque captures after projection.
//! No projection is performed here; only the inactive world obligation is set.
use super::external::{self, CapturedReadObligation};
use crate::{
    card_source::CardHandle,
    cards::CardEffectRegistry,
    debug_runner::DebugRunner,
    dsl_cards::{bindings::Bindings, raw_rust::EngineRawRustRegistry},
    effect::{CardEffect, Effect},
    effect_context::{EffectContext, EffectReadContext},
    game::Game,
    permanent::PermanentHandle,
    trigger_context::TriggerContext,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

fn captured() -> TriggerContext {
    let mut context = TriggerContext::default();
    context.event_card = (Some(CardHandle(42))).into();
    context
}

fn caller(game: &mut Game, mode: u8) {
    game.current_trigger_context = match mode {
        0 => None,
        1 => Some(TriggerContext::default()),
        _ => {
            let mut context = TriggerContext::default();
            context
                .simultaneous_play_members
                .push(TriggerContext::default());
            Some(context)
        }
    };
}

fn verify<T: PartialEq + std::fmt::Debug>(reject: bool, run: impl FnOnce() -> T, expected: T) {
    let result = catch_unwind(AssertUnwindSafe(run));
    if reject {
        let error =
            result.expect_err("world obligation must reject an opaque capture before its body");
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            message.contains("closed-list projected context reached unproven callback"),
            "{message}"
        );
    } else {
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn world_obligation_is_clone_local_and_survives_context_replacement() {
    let original = DebugRunner::builder().start().game;
    assert!(!original.projected_world_reads_guarded());
    let mut guarded = original.clone();
    guarded.require_projected_world_read_guard_for_test();
    guarded.require_projected_world_read_guard_for_test();
    for mode in 0..3 {
        caller(&mut guarded, mode);
        assert!(guarded.projected_world_reads_guarded());
        assert!(guarded.clone().projected_world_reads_guarded());
        assert!(!original.projected_world_reads_guarded());
        assert!(!original.clone().projected_world_reads_guarded());
    }
}

#[test]
fn retained_raw_callbacks_reject_old_captures_under_unmarked_callers() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = EngineRawRustRegistry::new();
    let saved = captured();
    let seen = calls.clone();
    registry.register_formula("capture", move |_, target| {
        seen.fetch_add(1, Ordering::SeqCst);
        assert_eq!(target.index, 3);
        if saved.event_card.read() == Some(CardHandle(42)) {
            17
        } else {
            3
        }
    });
    let saved = captured();
    let seen = calls.clone();
    registry.register_step("capture", move |ctx, _| {
        seen.fetch_add(1, Ordering::SeqCst);
        // This restoration would erase a context-local marker inside the body.
        ctx.game.current_trigger_context = Some(saved.clone());
        ctx.game.memory += if saved.event_card.read().is_some() { 2 } else { 1 };
    });
    let formula = registry.formula_fn("capture").unwrap();
    let step = registry.clone().step_fn("capture").unwrap();
    for seat in 0..2 {
        for mode in 0..3 {
            for reject in [false, true] {
                let mut original = DebugRunner::builder().memory(5).start().game;
                if reject {
                    original.require_projected_world_read_guard_for_test();
                }
                for mut game in [original.clone(), original] {
                    caller(&mut game, mode);
                    let before = calls.load(Ordering::SeqCst);
                    let before_context = format!("{:?}", game.current_trigger_context);
                    verify(
                        reject,
                        || {
                            formula(
                                &EffectReadContext::new(&game, CardHandle(1), None, seat),
                                PermanentHandle {
                                    player: seat,
                                    index: 3,
                                },
                            )
                        },
                        17,
                    );
                    verify(
                        reject,
                        || {
                            step(
                                &mut EffectContext::new(&mut game, CardHandle(1), None, seat),
                                &mut Bindings::default(),
                            )
                        },
                        (),
                    );
                    assert_eq!(
                        calls.load(Ordering::SeqCst) - before,
                        if reject { 0 } else { 2 }
                    );
                    assert_eq!(game.memory, if reject { 5 } else { 7 });
                    if reject {
                        assert_eq!(
                            format!("{:?}", game.current_trigger_context),
                            before_context
                        );
                    } else {
                        assert_eq!(
                            game.current_trigger_context.unwrap().event_card.read(),
                            Some(CardHandle(42))
                        );
                    }
                }
            }
        }
    }
}

struct CapturingFactory {
    saved: TriggerContext,
    calls: Arc<AtomicUsize>,
    factories: Arc<AtomicUsize>,
}
impl CardEffect for CapturingFactory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.factories.fetch_add(1, Ordering::SeqCst);
        let mut effect = Effect::on_play(card).build();
        let saved = self.saved.clone();
        let calls = self.calls.clone();
        effect.condition = Some(Arc::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            saved.event_card.read().is_some()
        }));
        let saved = self.saved.clone();
        let calls = self.calls.clone();
        effect.process = Some(Box::new(move |ctx| {
            calls.fetch_add(1, Ordering::SeqCst);
            ctx.game.memory += if saved.event_card.read().is_some() { 2 } else { 1 };
        }));
        let saved = self.saved.clone();
        let calls = self.calls.clone();
        effect.pay_cost_fn = Some(Box::new(move |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            saved.event_card.read().is_some()
        }));
        vec![effect]
    }
}

#[test]
fn returned_effects_reject_captures_but_factory_execution_remains_unproved() {
    let calls = Arc::new(AtomicUsize::new(0));
    let factories = Arc::new(AtomicUsize::new(0));
    let mut registry = CardEffectRegistry::new();
    registry.insert(
        "captured",
        Arc::new(CapturingFactory {
            saved: captured(),
            calls: calls.clone(),
            factories: factories.clone(),
        }),
    );
    let factory = registry.clone().get("captured").unwrap();
    for seat in 0..2 {
        for mode in 0..3 {
            for reject in [false, true] {
                for cloned in [false, true] {
                    let early = factory.effects(CardHandle(1));
                    let mut original = DebugRunner::builder().memory(5).start().game;
                    if reject {
                        original.require_projected_world_read_guard_for_test();
                    }
                    // Explicit limit: no Game argument reaches this factory; it STILL RUNS.
                    let before_factory = factories.load(Ordering::SeqCst);
                    let late = factory.effects(CardHandle(1));
                    assert_eq!(factories.load(Ordering::SeqCst), before_factory + 1);
                    for mut effects in [early, late] {
                        let mut game = original.clone();
                        if cloned {
                            game = game.clone();
                        }
                        caller(&mut game, mode);
                        let mut effect = effects.pop().unwrap();
                        let before = calls.load(Ordering::SeqCst);
                        verify(
                            reject,
                            || {
                                effect.condition.as_ref().unwrap()(&EffectReadContext::new(
                                    &game,
                                    CardHandle(1),
                                    None,
                                    seat,
                                ))
                            },
                            true,
                        );
                        let payment = effect.pay_cost_fn.take().unwrap();
                        verify(
                            reject,
                            || {
                                payment(&mut EffectContext::new(
                                    &mut game,
                                    CardHandle(1),
                                    None,
                                    seat,
                                ))
                            },
                            true,
                        );
                        verify(
                            reject,
                            || {
                                effect.process.as_ref().unwrap()(&mut EffectContext::new(
                                    &mut game,
                                    CardHandle(1),
                                    None,
                                    seat,
                                ))
                            },
                            (),
                        );
                        assert_eq!(
                            calls.load(Ordering::SeqCst) - before,
                            if reject { 0 } else { 3 }
                        );
                        assert_eq!(game.memory, if reject { 5 } else { 7 });
                    }
                }
            }
        }
    }
    assert_eq!(factories.load(Ordering::SeqCst), 48);
}

#[test]
fn false_capture_snapshot_cannot_override_later_world_obligation() {
    let saved = captured();
    let snapshot = CapturedReadObligation::from_context(Some(&saved));
    let calls = Arc::new(AtomicUsize::new(0));
    for mode in 0..3 {
        for reject in [false, true] {
            for cloned in [false, true] {
                // Construct the FnOnce callbacks before originating any world obligation.
                let seen = calls.clone();
                let accepted = saved.clone();
                let accept = external::guard_selection(
                    Box::new(move |game, action| {
                        seen.fetch_add(1, Ordering::SeqCst);
                        assert_eq!(action, 7);
                        game.current_trigger_context = Some(accepted.clone());
                        game.memory += 2;
                    }),
                    snapshot,
                );
                let seen = calls.clone();
                let declined = saved.clone();
                let decline = external::guard_decline(
                    Box::new(move |game| {
                        seen.fetch_add(1, Ordering::SeqCst);
                        game.current_trigger_context = Some(declined.clone());
                        game.memory -= 1;
                    }),
                    snapshot,
                );
                let mut game = DebugRunner::builder().memory(5).start().game;
                if reject {
                    game.require_projected_world_read_guard_for_test();
                }
                if cloned {
                    game = game.clone();
                }
                caller(&mut game, mode);
                let before = calls.load(Ordering::SeqCst);
                let before_context = format!("{:?}", game.current_trigger_context);
                verify(reject, || accept(&mut game, 7), ());
                caller(&mut game, mode);
                verify(reject, || decline(&mut game), ());
                assert_eq!(
                    calls.load(Ordering::SeqCst) - before,
                    if reject { 0 } else { 2 }
                );
                assert_eq!(game.memory, if reject { 5 } else { 6 });
                if reject {
                    assert_eq!(
                        format!("{:?}", game.current_trigger_context),
                        before_context
                    );
                }
            }
        }
    }
}
