//! Exact factory identity is a construction fact, never execution permission.
use crate::{
    cards::CardEffectRegistry, card_source::CardHandle,
    debug_runner::DebugRunner, dsl_cards::{DslCardEffect, raw_rust::EngineRawRustRegistry},
    effect::{CardEffect, Effect}, effect_context::EffectContext,
    trigger_context::TriggerContext,
};
use std::{sync::{Arc, atomic::{AtomicUsize, Ordering}}, panic::{catch_unwind, AssertUnwindSafe}};
fn spec() -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(
        "card: BIND\nname: Binding\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: captured\n").unwrap();
    Arc::new(digimon_dsl::compile::compile(&spec).unwrap())
}
fn effects(card: CardHandle, value: i16) -> Vec<Effect> {
    vec![Effect::on_play(card).name(&format!("captured-{value}"))
        .process(move |ctx| ctx.game.memory=value).build()]
}
fn raw(calls: Arc<AtomicUsize>, value: i16) -> Arc<EngineRawRustRegistry> {
    let mut raw=EngineRawRustRegistry::new();
    raw.register_declarative("captured", move |card| {
        calls.fetch_add(1,Ordering::SeqCst); effects(card,value)
    });
    Arc::new(raw)
}
struct Opaque(Arc<AtomicUsize>);
impl CardEffect for Opaque {
    fn effects(&self, card: CardHandle)->Vec<Effect> {
        self.0.fetch_add(1,Ordering::SeqCst); effects(card,7)
    }
}
#[test]
fn exact_factory_query_distinguishes_clones_reinsertions_aliases_and_opaque_dsl() {
    let compiled=spec();let calls=Arc::new(AtomicUsize::new(0));
    let raw7=raw(calls.clone(),7);let raw9=raw(calls.clone(),9);
    let mut original=CardEffectRegistry::new();
    original.insert_compiled("BIND",compiled.clone(),raw7.clone());
    let cloned=original.clone();
    let mut reinserted=original.clone();
    reinserted.insert_compiled("BIND",compiled.clone(),raw7.clone());
    let mut changed=original.clone();
    changed.insert_compiled("BIND",compiled.clone(),raw9.clone());
    let mut disguised=original.clone();
    disguised.insert("BIND",Arc::new(DslCardEffect::new_with_raw(compiled.clone(),raw7.clone())));
    let mut opaque=CardEffectRegistry::new();opaque.insert("BIND",Arc::new(Opaque(calls.clone())));
    let mut alias=original.clone();alias.insert_compiled("ALIAS",compiled.clone(),raw7.clone());
    // Stable labels describe how these fixtures were constructed, independent
    // of the identity-comparison implementation under test.
    let factories=[
        (original.get("BIND").unwrap(),0),
        (cloned.get("BIND").unwrap(),0),
        (reinserted.get("BIND").unwrap(),1),
        (changed.get("BIND").unwrap(),2),
        (disguised.get("BIND").unwrap(),3),
        (opaque.get("BIND").unwrap(),4),
        (alias.get("ALIAS").unwrap(),5),
    ];
    let registries=[original,cloned,reinserted,changed,disguised,opaque,alias];
    let mut positive=0;let mut negative=0;
    for (row,registry) in registries.iter().enumerate() {
        for key in ["BIND","ALIAS","MISSING"] {
            let expected_label=match (row,key) {
                (0|1|6,"BIND")=>Some(0),(2,"BIND")=>Some(1),(3,"BIND")=>Some(2),
                (6,"ALIAS")=>Some(5),_=>None,
            };
            for (factory,label) in &factories {
                let origin=registry.compiled_origin_for_factory(key,factory);
                assert_eq!(origin.is_some(),expected_label==Some(*label),"row={row} key={key} factory={label}");
                if let Some(origin)=origin {
                    positive+=1;
                    assert!(Arc::ptr_eq(&origin.compiled,&compiled));
                    assert!(Arc::ptr_eq(&origin.raw,if *label==2 {&raw9}else{&raw7}));
                    let requests=origin.direct_raw_factory_requests();
                    assert_eq!(requests.len(),1);
                    assert_eq!(requests[0].name,"captured");
                    assert!(requests[0].registered_declarative);
                    assert_eq!(requests[0].clause_index,0);
                } else {negative+=1;}
            }
        }
    }
    assert_eq!((positive,negative),(9,138));
    assert_eq!(calls.load(Ordering::SeqCst),0,"identity/fact queries must not construct effects");
}
#[test]
fn retained_factory_facts_do_not_follow_replacement_or_exempt_runtime_callbacks() {
    let mut cases=0;let mut rejections=0;
    for seat in 0..2 {for cloned in [false,true] {for caller in 0..3 {
        for old_compiled in [false,true] {for marked in [false,true] {
            let compiled=spec();let calls=Arc::new(AtomicUsize::new(0));
            let raw7=raw(calls.clone(),7);let raw9=raw(calls.clone(),9);
            let mut runner=DebugRunner::builder().start();runner.set_first_player(seat);
            let mut game=runner.game;game.effect_registry=CardEffectRegistry::new();
            if old_compiled {game.effect_registry.insert_compiled("BIND",compiled.clone(),raw7);}
            else {game.effect_registry.insert("BIND",Arc::new(Opaque(calls.clone())));}
            let prior=game.effect_registry.clone();
            let old_factory=prior.get("BIND").unwrap();
            let card=CardHandle(42);
            let old_effects=old_factory.effects(card);
            game.effect_registry.insert_compiled("BIND",compiled,raw9);
            let new_factory=game.effect_registry.get("BIND").unwrap();
            let new_effects=new_factory.effects(card);
            if cloned {game=game.clone();}
            let mut context=TriggerContext::default();
            if caller==2 {
                let mut child=TriggerContext::default();
                child.simultaneous_play_members.push(TriggerContext::default());
                context.simultaneous_play_members.push(child);
            }
            game.current_trigger_context=(caller!=0).then_some(context);
            if marked {game.require_projected_world_read_guard_for_test();}
            let before=calls.load(Ordering::SeqCst);assert_eq!(before,2);
            assert_eq!(prior.compiled_origin_for_factory("BIND",&old_factory).is_some(),old_compiled);
            assert!(prior.compiled_origin_for_factory("BIND",&new_factory).is_none());
            assert!(game.effect_registry.compiled_origin_for_factory("BIND",&old_factory).is_none());
            assert!(game.effect_registry.compiled_origin_for_factory("BIND",&new_factory).is_some());
            assert_eq!(calls.load(Ordering::SeqCst),before);
            let context_before=format!("{:?}",game.current_trigger_context);
            for (list,value) in [(&old_effects,7),(&new_effects,9)] {
                assert_eq!(list.len(),1);assert_eq!(list[0].name,format!("captured-{value}"));
                game.memory=3;
                let outcome=catch_unwind(AssertUnwindSafe(||list[0].process.as_ref().unwrap()(
                    &mut EffectContext::new(&mut game,card,None,seat))));
                if marked {
                    let error=outcome.expect_err("matching compiler origin is not runtime permission");
                    let message=error.downcast_ref::<String>().map(String::as_str)
                        .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
                    assert!(message.contains("closed-list projected context reached unproven callback"));
                    assert_eq!(game.memory,3);rejections+=1;
                } else {outcome.unwrap();assert_eq!(game.memory,value);}
            }
            assert_eq!(format!("{:?}",game.current_trigger_context),context_before);
            cases+=1;
        }}
    }}}
    assert_eq!((cases,rejections),(48,48));
}
