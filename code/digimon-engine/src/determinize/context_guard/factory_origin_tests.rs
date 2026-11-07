//! No factory origin is trusted: block Game factory/cache routes before metadata
//! or bodies are exposed in a world with the inactive projection obligation.
use crate::{
    card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner},
    dsl_cards::{raw_rust::EngineRawRustRegistry, DslCardEffect},
    effect::{CardEffect, Effect},
    game::Game,
    trigger_context::TriggerContext,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

#[derive(Clone, Copy, Debug)]
enum Route {
    Full,
    Registry,
    Owned,
}

fn lookup(game: &Game, route: Route, id: &str, card: CardHandle) -> Option<Vec<String>> {
    let names = |effects: &[Effect]| effects.iter().map(|e| e.name.clone()).collect();
    match route {
        Route::Full => game.effects_for_card(id, card).map(|v| names(&v)),
        Route::Registry => game.registry_effects(id, card).map(|v| names(&v)),
        Route::Owned => game.effects_for_card_owned(id, card).map(|v| names(&v)),
    }
}

fn verify(reject: bool, run: impl FnOnce() -> Option<Vec<String>>, expected: Option<Vec<String>>) {
    let result = catch_unwind(AssertUnwindSafe(run));
    if reject {
        let error = result
            .expect_err("unproved factory/cache lookup must reject before returning metadata");
        let message = error
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| error.downcast_ref::<&str>().copied())
            .unwrap_or("");
        assert!(
            message.contains("closed-list projected world reached unproven factory"),
            "{message}"
        );
    } else {
        assert_eq!(result.unwrap(), expected);
    }
}

struct CapturedMetadataFactory {
    captured: TriggerContext,
    calls: Arc<AtomicUsize>,
    expected_card: CardHandle,
    empty: bool,
}
impl CardEffect for CapturedMetadataFactory {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(card, self.expected_card);
        // Returned-callback guards cannot protect a read used to build metadata.
        let name = if self.captured.event_card.read() == Some(CardHandle(42)) {
            "captured-42"
        } else {
            "erased"
        };
        if self.empty {
            vec![]
        } else {
            vec![
                Effect::on_play(card).name(name).build(),
                Effect::on_play(card).name("second").build(),
            ]
        }
    }
}

fn exercise(route: Route) {
    for seat in 0..2 {
        for known in [false, true] {
            for warm in [false, true] {
                for cloned in [false, true] {
                    for reject in [false, true] {
                        for empty in [false, true] {
                            let mut game = DebugRunner::builder()
                                .add_card(make_test_card("FACTORY-KNOWN", "Factory known"))
                                .hand(seat, &["FACTORY-KNOWN"])
                                .memory(5)
                                .start()
                                .game;
                            let card = game.players[seat as usize].hand[0].handle();
                            let id = if known {
                                "FACTORY-KNOWN"
                            } else {
                                "FACTORY-UNKNOWN"
                            };
                            let calls = Arc::new(AtomicUsize::new(0));
                            let mut captured = TriggerContext::default();
                            captured.event_card = (Some(CardHandle(42))).into();
                            game.effect_registry.insert(
                                id,
                                Arc::new(CapturedMetadataFactory {
                                    captured,
                                    calls: calls.clone(),
                                    expected_card: card,
                                    empty,
                                }),
                            );
                            let expected = Some(if empty {
                                vec![]
                            } else {
                                vec!["captured-42".into(), "second".into()]
                            });
                            if warm {
                                assert_eq!(lookup(&game, route, id, card), expected);
                            }
                            if reject {
                                game.require_projected_world_read_guard_for_test();
                            }
                            if cloned {
                                game = game.clone();
                            }
                            game.current_trigger_context = None;
                            let before = calls.load(Ordering::SeqCst);
                            let phase = game.current_phase;
                            verify(reject, || lookup(&game, route, id, card), expected);
                            let cached = known && warm && !matches!(route, Route::Owned);
                            // Debug builds intentionally rebuild Full cache entries as an oracle.
                            let debug_rebuild =
                                cfg!(debug_assertions) && matches!(route, Route::Full);
                            let wanted = usize::from(!reject && (!cached || debug_rebuild));
                            assert_eq!(
                                calls.load(Ordering::SeqCst) - before,
                                wanted,
                                "{route:?} known={known} warm={warm}"
                            );
                            assert_eq!(game.memory, 5);
                            assert_eq!(game.current_phase, phase);
                            assert!(game.current_trigger_context.is_none());
                        }
                    }
                }
            }
        }
    }
    // Ordinary absent entries remain absent. The world prototype is stricter:
    // even absent lookup requests reject until factory origin is proved.
    for reject in [false, true] {
        let mut game = DebugRunner::builder().start().game;
        if reject {
            game.require_projected_world_read_guard_for_test();
        }
        verify(
            reject,
            || lookup(&game, route, "FACTORY-MISSING", CardHandle(999)),
            None,
        );
    }
}

#[test]
fn full_lookup_rejects_unproved_factories_and_preexisting_cache_metadata() {
    exercise(Route::Full);
}

#[test]
fn registry_lookup_rejects_unproved_factories_and_preexisting_cache_metadata() {
    exercise(Route::Registry);
}

#[test]
fn owned_lookup_rejects_unproved_factory_before_body() {
    exercise(Route::Owned);
}

#[test]
fn compiled_dsl_label_does_not_authorize_raw_factory_execution() {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(
        "card: FACTORY-DSL\nname: Factory DSL\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: captured\n").unwrap();
    let compiled = Arc::new(digimon_dsl::compile::compile(&spec).unwrap());
    for route in [Route::Full, Route::Registry, Route::Owned] {
        for warm in [false, true] {
            for reject in [false, true] {
                let calls = Arc::new(AtomicUsize::new(0));
                let mut raw = EngineRawRustRegistry::new();
                let seen = calls.clone();
                let captured = Some(CardHandle(42));
                raw.register_declarative("captured", move |card| {
                    seen.fetch_add(1, Ordering::SeqCst);
                    vec![Effect::on_play(card)
                        .name(if captured.is_some() {
                            "raw-capture"
                        } else {
                            "erased"
                        })
                        .build()]
                });
                let mut game = DebugRunner::builder()
                    .add_card(make_test_card("FACTORY-DSL", "Factory DSL"))
                    .hand(0, &["FACTORY-DSL"])
                    .memory(5)
                    .start()
                    .game;
                let card = game.players[0].hand[0].handle();
                game.effect_registry.insert(
                    "FACTORY-DSL",
                    Arc::new(DslCardEffect::new_with_raw(compiled.clone(), Arc::new(raw))),
                );
                let expected = Some(vec!["raw-capture".into()]);
                if warm {
                    assert_eq!(lookup(&game, route, "FACTORY-DSL", card), expected);
                }
                if reject {
                    game.require_projected_world_read_guard_for_test();
                }
                let game = game.clone();
                let before = calls.load(Ordering::SeqCst);
                verify(
                    reject,
                    || lookup(&game, route, "FACTORY-DSL", card),
                    expected,
                );
                let cached = warm && !matches!(route, Route::Owned);
                let debug_rebuild = cfg!(debug_assertions) && matches!(route, Route::Full);
                assert_eq!(
                    calls.load(Ordering::SeqCst) - before,
                    usize::from(!reject && (!cached || debug_rebuild))
                );
            }
        }
    }
}
