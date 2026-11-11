use super::*;
use crate::{
    card_source::CardHandle, cards::CardEffectRegistry, debug_runner::DebugRunner, effect::Effect,
    effect_context::EffectContext,
};
use digimon_dsl::compiled::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

fn base() -> CompiledCard {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str("card: FACTS\nname: Facts\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects: []\n").unwrap();
    digimon_dsl::compile::compile(&spec).unwrap()
}
fn declaration(name: &str, scope: CompiledScope) -> CompiledClause {
    CompiledClause::Declarative(CompiledDeclarativeClause::RawRust {
        fn_name: name.into(),
        triggers: vec![],
        scope,
        summary: None,
        summary_key: None,
    })
}
fn runtime_step() -> CompiledClause {
    CompiledClause::Triggered(CompiledTriggeredClause {
        when: vec![CompiledTiming::OnPlay],
        scope: CompiledScope::FaceUp,
        active_when: None,
        condition: None,
        trigger_if: None,
        timing_conditions: vec![],
        optional: false,
        outer_prompt: false,
        once_per_turn: false,
        as_option_effect: false,
        max_per_turn: None,
        process: vec![CompiledStep::RawRust {
            fn_name: "step_only".into(),
            consumes: vec![],
            binds: vec![],
        }],
        summary: None,
        summary_key: None,
    })
}
fn block(name: &str, scope: CompiledScope) -> Vec<CompiledClause> {
    vec![
        declaration(name, scope),
        runtime_step(),
        declaration("step_only", scope),
        declaration("shared", scope),
    ]
}

#[test]
fn direct_requests_match_actual_top_and_dual_factory_order_without_execution() {
    for scope in [
        CompiledScope::FaceUp,
        CompiledScope::Inherited,
        CompiledScope::Both,
    ] {
        for kind in [CompiledCardKind::Dual, CompiledCardKind::Digimon] {
            let mut card = base();
            card.kind = kind;
            card.effects = block("top", scope);
            // Lowering inspects present dual data even if a manually supplied IR
            // has another kind. Origin does not certify compiler validation.
            card.dual = Some(CompiledDual {
                digimon: CompiledDualDigimon {
                    level: 3,
                    dp: 1000,
                    colors: vec![CompiledColor::Red],
                    traits: vec![],
                    effect_text: String::new(),
                    inherited_text: String::new(),
                    effects: block("digimon", scope),
                },
                option: CompiledDualOption {
                    use_cost: 1,
                    colors: vec![CompiledColor::Red],
                    effect_text: String::new(),
                    security_text: String::new(),
                    keywords: vec![],
                    use_requirement: None,
                    effects: block("option", scope),
                },
            });
            let log = Arc::new(Mutex::new(Vec::<String>::new()));
            let steps = Arc::new(AtomicUsize::new(0));
            let mut raw = EngineRawRustRegistry::new();
            for name in ["top", "digimon", "option", "shared"] {
                let log = log.clone();
                raw.register_declarative(name, move |card| {
                    log.lock().unwrap().push(name.into());
                    vec![Effect::on_play(card)
                        .name(&format!("factory:{name}"))
                        .build()]
                });
            }
            let seen = steps.clone();
            raw.register_step("step_only", move |_, _| {
                seen.fetch_add(1, Ordering::SeqCst);
            });
            assert!(raw.contains_fn("step_only"));
            let mut registry = CardEffectRegistry::new();
            registry.insert_compiled("FACTS", Arc::new(card), Arc::new(raw));
            let origin = registry.compiled_origin("FACTS").unwrap();
            let mut expected = Vec::new();
            for (region, name) in [
                (ClauseRegion::Card, "top"),
                (ClauseRegion::DigimonFace, "digimon"),
                (ClauseRegion::OptionFace, "option"),
            ] {
                for (index, name, registered) in [
                    (0, name, true),
                    (2, "step_only", false),
                    (3, "shared", true),
                ] {
                    expected.push(RawFactoryRequest {
                        region,
                        clause_index: index,
                        name: name.into(),
                        registered_declarative: registered,
                    });
                }
            }
            assert_eq!(origin.direct_raw_factory_requests(), expected);
            assert!(
                log.lock().unwrap().is_empty(),
                "facts must not execute any factory"
            );
            assert_eq!(steps.load(Ordering::SeqCst), 0);
            let emitted = registry.get("FACTS").unwrap().effects(CardHandle(7));
            let expected_calls = ["top", "shared", "digimon", "shared", "option", "shared"];
            assert_eq!(*log.lock().unwrap(), expected_calls);
            let names: Vec<_> = emitted
                .iter()
                .filter_map(|e| e.name.strip_prefix("factory:"))
                .collect();
            assert_eq!(names, expected_calls);
            assert_eq!(
                steps.load(Ordering::SeqCst),
                0,
                "runtime calls are not construction calls"
            );
        }
    }
}

#[test]
fn empty_direct_factory_facts_do_not_certify_captured_runtime_step() {
    for seat in 0..2 {
        for marked in [false, true] {
            let mut card = base();
            card.effects = vec![runtime_step()];
            let calls = Arc::new(AtomicUsize::new(0));
            let seen = calls.clone();
            let captured = Some(CardHandle(42));
            let mut raw = EngineRawRustRegistry::new();
            raw.register_step("step_only", move |ctx, _| {
                seen.fetch_add(1, Ordering::SeqCst);
                ctx.gain_memory(if captured.is_some() { 1 } else { 2 });
            });
            let mut registry = CardEffectRegistry::new();
            registry.insert_compiled("FACTS", Arc::new(card), Arc::new(raw));
            assert!(registry
                .compiled_origin("FACTS")
                .unwrap()
                .direct_raw_factory_requests()
                .is_empty());
            let effects = registry.get("FACTS").unwrap().effects(CardHandle(7));
            assert_eq!(effects.len(), 1);
            assert_eq!(calls.load(Ordering::SeqCst), 0);
            let mut game = DebugRunner::builder().memory(5).start().game;
            game.memory_pair = (seat, 1 - seat);
            game.turn_player_idx = seat as usize;
            if marked {
                game.require_projected_world_read_guard_for_test();
            }
            let result = catch_unwind(AssertUnwindSafe(|| {
                let mut ctx = EffectContext::new(&mut game, CardHandle(7), None, seat);
                effects[0].process.as_ref().unwrap()(&mut ctx);
            }));
            if marked {
                let err = result
                    .err()
                    .expect("empty construction facts must not exempt runtime callback");
                let msg = err
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| err.downcast_ref::<&str>().copied())
                    .unwrap_or("");
                assert!(msg.contains("closed-list projected context reached unproven callback"));
                assert_eq!(calls.load(Ordering::SeqCst), 0);
                assert_eq!(game.memory, 5);
            } else {
                result.unwrap();
                assert_eq!(calls.load(Ordering::SeqCst), 1);
                assert_eq!(game.memory, 6);
            }
        }
    }
}

#[test]
fn registry_namespace_facts_follow_bound_snapshot_not_later_caller_mutation() {
    let mut card = base();
    card.effects = vec![declaration("late", CompiledScope::FaceUp)];
    let card = Arc::new(card);
    let mut raw = Arc::new(EngineRawRustRegistry::new());
    let mut registry = CardEffectRegistry::new();
    registry.insert_compiled("FACTS", card.clone(), raw.clone());
    let prior = registry.clone();
    let retained = prior.get("FACTS").unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    Arc::make_mut(&mut raw).register_declarative("late", move |card| {
        seen.fetch_add(1, Ordering::SeqCst);
        vec![Effect::on_play(card).name("late").build()]
    });
    let before = registry
        .compiled_origin("FACTS")
        .unwrap()
        .direct_raw_factory_requests();
    assert_eq!(before.len(), 1);
    assert!(!before[0].registered_declarative);
    registry.insert_compiled("FACTS", card, raw);
    let after = registry
        .compiled_origin("FACTS")
        .unwrap()
        .direct_raw_factory_requests();
    assert_eq!(after.len(), 1);
    assert!(after[0].registered_declarative);
    assert_eq!(
        prior
            .compiled_origin("FACTS")
            .unwrap()
            .direct_raw_factory_requests(),
        before
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(retained.effects(CardHandle(7)).is_empty());
    assert_eq!(
        registry.get("FACTS").unwrap().effects(CardHandle(7))[0].name,
        "late"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
