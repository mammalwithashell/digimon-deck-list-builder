//! A compiler-owned construction record is NOT permission to execute.
use super::*;
use crate::{
    card_source::CardHandle,
    debug_runner::DebugRunner,
    dsl_cards::{raw_rust::EngineRawRustRegistry, DslCardEffect},
    effect::Effect,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::atomic::{AtomicUsize, Ordering},
};

fn spec() -> digimon_dsl::CardSpec {
    serde_yml::from_str("card: ORIGIN\nname: Origin\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: captured\n").unwrap()
}

fn raw(calls: Arc<AtomicUsize>, answer: bool) -> Arc<EngineRawRustRegistry> {
    let mut raw = EngineRawRustRegistry::new();
    let captured = answer.then_some(CardHandle(42));
    raw.register_declarative("captured", move |card| {
        calls.fetch_add(1, Ordering::SeqCst);
        vec![Effect::on_play(card)
            .name(if captured.is_some() {
                "captured"
            } else {
                "empty"
            })
            .build()]
    });
    Arc::new(raw)
}

#[test]
fn compiler_origin_binds_exact_inputs_without_executing_factory() {
    for answer in [false, true] {
        let mut compiled = Arc::new(digimon_dsl::compile::compile(&spec()).unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let mut raw = raw(calls.clone(), answer);
        let mut registry = CardEffectRegistry::new();
        registry.insert_compiled("ORIGIN", compiled.clone(), raw.clone());
        let origin = registry
            .compiled_origin("ORIGIN")
            .expect("construction origin required");
        assert!(Arc::ptr_eq(&origin.compiled, &compiled));
        assert!(Arc::ptr_eq(&origin.raw, &raw));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "registration must not evaluate captures"
        );
        let cloned = registry.clone();
        let prefetched = registry.get("ORIGIN").unwrap();
        assert!(Arc::ptr_eq(&prefetched, &cloned.get("ORIGIN").unwrap()));
        assert!(Arc::ptr_eq(
            &origin.compiled,
            &cloned.compiled_origin("ORIGIN").unwrap().compiled
        ));
        // Caller mutation must fork both inputs, never change retained factory/origin.
        Arc::make_mut(&mut compiled).name = "caller changed".into();
        Arc::make_mut(&mut raw).register_declarative("captured", |card| {
            vec![Effect::on_play(card).name("caller changed").build()]
        });
        assert_eq!(origin.compiled.name, "Origin");
        assert!(!Arc::ptr_eq(&origin.raw, &raw));
        for card in [CardHandle(7), CardHandle(8)] {
            let effects = prefetched.effects(card);
            assert_eq!(effects.len(), 1);
            assert_eq!(effects[0].name, if answer { "captured" } else { "empty" });
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(registry.compiled_origin("absent").is_none());
    }
}

struct Opaque;
impl CardEffect for Opaque {
    fn effects(&self, card: CardHandle) -> Vec<Effect> {
        vec![Effect::on_play(card).name("opaque").build()]
    }
}

#[test]
fn opaque_replacement_clears_origin_even_for_actual_dsl_type() {
    for disguised_dsl in [false, true] {
        let compiled = Arc::new(digimon_dsl::compile::compile(&spec()).unwrap());
        let calls = Arc::new(AtomicUsize::new(0));
        let raw = raw(calls.clone(), true);
        let mut registry = CardEffectRegistry::new();
        registry.insert_compiled("ORIGIN", compiled.clone(), raw.clone());
        assert!(registry.compiled_origin("ORIGIN").is_some());
        let prior = registry.clone();
        let retained = registry.get("ORIGIN").unwrap();
        let replacement: Arc<dyn CardEffect> = if disguised_dsl {
            Arc::new(DslCardEffect::new_with_raw(compiled.clone(), raw.clone()))
        } else {
            Arc::new(Opaque)
        };
        registry.insert("ORIGIN", replacement);
        assert!(
            registry.compiled_origin("ORIGIN").is_none(),
            "type labels cannot claim origin"
        );
        assert!(registry.clone().compiled_origin("ORIGIN").is_none());
        assert!(prior.compiled_origin("ORIGIN").is_some());
        assert!(!Arc::ptr_eq(&retained, &registry.get("ORIGIN").unwrap()));
        assert_eq!(retained.effects(CardHandle(7))[0].name, "captured");
        registry.insert_compiled("ORIGIN", compiled, raw);
        assert!(registry.compiled_origin("ORIGIN").is_some());
        assert_eq!(registry.len(), 1);
    }
}

#[test]
fn dsl_loader_records_origin_but_world_guard_still_rejects_before_raw_factory() {
    let pack = digimon_dsl::CardRegistry::from_specs("origin-test", &[spec()]).unwrap();
    for cloned in [false, true] {
        for marked in [false, true] {
            let calls = Arc::new(AtomicUsize::new(0));
            let raw = raw(calls.clone(), true);
            let mut registry = CardEffectRegistry::new();
            crate::dsl_cards::register_dsl_cards_with_raw(&mut registry, &pack, raw.clone());
            let origin = registry
                .compiled_origin("ORIGIN")
                .expect("loader must bind actual construction");
            assert_eq!(origin.compiled.as_ref(), pack.lookup("ORIGIN").unwrap());
            assert!(Arc::ptr_eq(&origin.raw, &raw));
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            let mut game = DebugRunner::builder().start().game;
            game.effect_registry = registry;
            if marked {
                game.require_projected_world_read_guard_for_test();
            }
            if cloned {
                game = game.clone();
            }
            assert!(game.effect_registry.compiled_origin("ORIGIN").is_some());
            let result = catch_unwind(AssertUnwindSafe(|| {
                game.registry_effects("ORIGIN", CardHandle(7))
            }));
            if marked {
                let err = result
                    .err()
                    .expect("origin must not exempt unknown factory");
                let message = err
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| err.downcast_ref::<&str>().copied())
                    .unwrap_or("");
                assert!(message.contains("closed-list projected world reached unproven factory"));
                assert_eq!(calls.load(Ordering::SeqCst), 0);
            } else {
                let effects = result.unwrap().unwrap();
                assert_eq!(effects[0].name, "captured");
                assert_eq!(calls.load(Ordering::SeqCst), 1);
            }
        }
    }
}
