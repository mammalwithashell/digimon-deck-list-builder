//! Exact immutable registry-output construction relations, never runtime permission.
use crate::{
    cards::CardEffectRegistry, card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner}, dsl_cards::raw_rust::EngineRawRustRegistry,
    effect::{CardEffect, Effect}, effect_context::EffectContext, game::Game,
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn spec() -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec = serde_yml::from_str(
        "card: OUTPUT\nname: Output\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: captured\n").unwrap();
    Arc::new(digimon_dsl::compile::compile(&spec).unwrap())
}
fn effects(card: CardHandle, value: i16) -> Vec<Effect> {
    vec![Effect::on_play(card).name(&format!("output-{value}"))
        .process(move |ctx| ctx.game.memory=value).build()]
}
fn raw(calls: Arc<AtomicUsize>, value: i16) -> Arc<EngineRawRustRegistry> {
    let mut raw=EngineRawRustRegistry::new();
    raw.register_declarative("captured",move |card| {
        calls.fetch_add(1,Ordering::SeqCst);effects(card,value)
    });Arc::new(raw)
}
struct Native { calls: Arc<AtomicUsize>, value: i16 }
impl CardEffect for Native {
    fn effects(&self, card: CardHandle)->Vec<Effect> {
        self.calls.fetch_add(1,Ordering::SeqCst);effects(card,self.value)
    }
}
fn install(registry:&mut CardEffectRegistry,id:&str,compiled:bool,value:i16,calls:Arc<AtomicUsize>) {
    if compiled {registry.insert_compiled(id,spec(),raw(calls,value));}
    else {registry.insert(id,Arc::new(Native{calls,value}));}
}
fn fixture(seat:u8)->(Game,CardHandle) {
    let mut runner=DebugRunner::builder().add_card(make_test_card("OUTPUT","Output"))
        .hand(seat,&["OUTPUT"]).start();
    runner.set_first_player(seat);let mut game=runner.game;
    let card=game.players[seat as usize].hand[0].handle();
    game.effect_registry=CardEffectRegistry::new();(game,card)
}
fn rejected(run:impl FnOnce(),expected:&str) {
    let err=catch_unwind(AssertUnwindSafe(run)).expect_err("must reject");
    let msg=err.downcast_ref::<String>().map(String::as_str)
        .or_else(||err.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(msg.contains(expected),"{msg}");
}
#[test]
fn derived_vectors_bind_only_their_exact_producer_and_remain_immutable() {
    let calls=Arc::new(AtomicUsize::new(0));let compiled=spec();let raw=raw(calls.clone(),7);
    let mut original=CardEffectRegistry::new();
    original.insert_compiled("OUTPUT",compiled.clone(),raw.clone());
    let a=original.derive_effects("OUTPUT",CardHandle(42)).unwrap();
    let b=original.derive_effects("OUTPUT",CardHandle(43)).unwrap();
    assert!(!Arc::ptr_eq(&a.shared_effects(),&b.shared_effects()));
    assert_eq!(a.shared_effects()[0].source_card,CardHandle(42));
    assert_eq!(b.shared_effects()[0].source_card,CardHandle(43));
    let cloned=original.clone();
    let mut alias=original.clone();alias.insert_compiled("ALIAS",compiled.clone(),raw.clone());
    let c=alias.derive_effects("ALIAS",CardHandle(42)).unwrap();
    let mut opaque=CardEffectRegistry::new();
    install(&mut opaque,"OUTPUT",false,7,calls.clone());
    let d=opaque.derive_effects("OUTPUT",CardHandle(42)).unwrap();
    let mut reinserted=original.clone();reinserted.insert_compiled("OUTPUT",compiled.clone(),raw.clone());
    let e=reinserted.derive_effects("OUTPUT",CardHandle(42)).unwrap();
    let records=[a,b,c,d,e];
    let registries=[original,cloned,alias,reinserted,opaque,CardEffectRegistry::new()];
    assert_eq!(calls.load(Ordering::SeqCst),5);
    let mut positive=0;let mut negative=0;
    for (row,registry) in registries.iter().enumerate() {
        for key in ["OUTPUT","ALIAS","MISSING"] {
            for (col,record) in records.iter().enumerate() {
                let expected=matches!((row,key,col),
                    (0|1|2,"OUTPUT",0|1)|(2,"ALIAS",2)|(3,"OUTPUT",4));
                let origin=registry.compiled_origin_for_derived(key,record);
                assert_eq!(origin.is_some(),expected,"row={row} key={key} col={col}");
                if let Some(origin)=origin {
                    positive+=1;
                    assert!(Arc::ptr_eq(&origin.compiled,&compiled));
                    assert!(Arc::ptr_eq(&origin.raw,&raw));
                    assert_eq!(origin.direct_raw_factory_requests()[0].name,"captured");
                } else {negative+=1;}
            }
        }
    }
    assert_eq!((positive,negative),(8,82));
    assert_eq!(calls.load(Ordering::SeqCst),5,"queries must not execute factories");
    assert!(registries[0].derive_effects("MISSING",CardHandle(42)).is_none());
    let retained=records[0].clone();
    let mut exposed=retained.shared_effects();
    assert!(Arc::get_mut(&mut exposed).is_none());
    assert!(Arc::try_unwrap(exposed).is_err());
    assert_eq!(retained.shared_effects()[0].name,"output-7");
}
struct DropToken(Arc<AtomicUsize>);
impl Drop for DropToken {fn drop(&mut self) {self.0.fetch_add(1,Ordering::SeqCst);}}
struct DropFactory { token: DropToken, empty:bool }
impl CardEffect for DropFactory {
    fn effects(&self,card:CardHandle)->Vec<Effect> {
        let _=&self.token;
        if self.empty {Vec::new()} else {effects(card,7)}
    }
}
#[test]
fn output_records_do_not_extend_opaque_or_compiled_factory_capture_lifetimes() {
    for compiled in [false,true] {for empty in [false,true] {
        let drops=Arc::new(AtomicUsize::new(0));
        let token=DropToken(drops.clone());let mut registry=CardEffectRegistry::new();
        if compiled {
            let mut raw=EngineRawRustRegistry::new();
            raw.register_declarative("captured",move |card| {
                let _=&token;
                if empty {Vec::new()} else {effects(card,7)}
            });
            registry.insert_compiled("OUTPUT",spec(),Arc::new(raw));
        } else {registry.insert("OUTPUT",Arc::new(DropFactory{token,empty}));}
        let factory=registry.get("OUTPUT").unwrap();
        let weak=Arc::downgrade(&factory);
        let record=registry.derive_effects("OUTPUT",CardHandle(42)).unwrap();
        let cloned=record.clone();let mut vector=record.shared_effects();
        assert_eq!(vector.len(),if empty {0}else{1});
        assert_eq!(registry.compiled_origin_for_derived("OUTPUT",&record).is_some(),compiled);
        drop(factory);drop(registry);
        assert!(weak.upgrade().is_none(),"record must not own the producer");
        assert_eq!(drops.load(Ordering::SeqCst),1,"factory-owned capture must be dropped");
        let mut replacement=CardEffectRegistry::new();
        install(&mut replacement,"OUTPUT",true,9,Arc::new(AtomicUsize::new(0)));
        assert!(replacement.compiled_origin_for_derived("OUTPUT",&record).is_none());
        assert!(replacement.compiled_origin_for_derived("OUTPUT",&cloned).is_none());
        assert!(Arc::get_mut(&mut vector).is_none());
        drop(record);assert!(Arc::get_mut(&mut vector).is_none());drop(cloned);
        Arc::get_mut(&mut vector).expect("no provenance record remains").clear();
        assert!(vector.is_empty());
    }}
}
#[test]
fn game_registry_records_preserve_cache_identity_and_replacement_behavior() {
    let mut cases=0;let mut callbacks=0;
    for seat in 0..2 {for cloned in [false,true] {for known in [false,true] {
        for old_compiled in [false,true] {for replacement_compiled in [false,true] {
            let (mut game,card)=fixture(seat);
            let id=if known {"OUTPUT"}else{"UNKNOWN-OUTPUT"};
            assert!(game.registry_effects_record(id,card).is_none());
            let calls=Arc::new(AtomicUsize::new(0));
            install(&mut game.effect_registry,id,old_compiled,7,calls.clone());
            let old=game.registry_effects_record(id,card).unwrap();
            let hit=game.registry_effects_record(id,card).unwrap();
            assert_eq!(Arc::ptr_eq(&old.shared_effects(),&hit.shared_effects()),known);
            let api=game.registry_effects(id,card).unwrap();
            assert_eq!(Arc::ptr_eq(&old.shared_effects(),&api),known);
            assert_eq!(calls.load(Ordering::SeqCst),if known {1}else{3});
            let prior=game.clone();if cloned {game=game.clone();}
            install(&mut game.effect_registry,id,replacement_compiled,9,calls.clone());
            assert!(game.effect_registry.compiled_origin_for_derived(id,&old).is_none());
            assert_eq!(prior.effect_registry.compiled_origin_for_derived(id,&old).is_some(),old_compiled);
            let fresh=game.registry_effects_record(id,card).unwrap();
            assert_eq!(game.effect_registry.compiled_origin_for_derived(id,&fresh).is_some(),replacement_compiled);
            assert!(!Arc::ptr_eq(&old.shared_effects(),&fresh.shared_effects()));
            for (record,value) in [(&old,7),(&fresh,9)] {
                let vector=record.shared_effects();game.memory=3;
                vector[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game,card,None,seat));
                assert_eq!(game.memory,value);callbacks+=1;
            }
            game.effect_registry=CardEffectRegistry::new();
            assert!(game.registry_effects_record(id,card).is_none());
            assert!(game.effect_registry.compiled_origin_for_derived(id,&fresh).is_none());
            cases+=1;
        }}
    }}}
    assert_eq!((cases,callbacks),(32,64));
}
#[test]
fn world_checks_precede_record_cache_factory_work_and_retained_callbacks() {
    let mut cases=0;let mut rejected_count=0;let mut ordinary=0;
    for seat in 0..2 {for cloned in [false,true] {for caller in 0..3 {
        for old_compiled in [false,true] {for marked in [false,true] {
            let (mut game,card)=fixture(seat);let calls=Arc::new(AtomicUsize::new(0));
            install(&mut game.effect_registry,"OUTPUT",old_compiled,7,calls.clone());
            let retained=game.registry_effects_record("OUTPUT",card).unwrap();
            let prior=game.effect_registry.clone();
            if cloned {game=game.clone();}
            let mut context=TriggerContext::default();
            if caller==2 {
                let mut child=TriggerContext::default();
                child.simultaneous_play_members.push(TriggerContext::default());
                context.simultaneous_play_members.push(child);
            }
            game.current_trigger_context=(caller!=0).then_some(context);
            if marked {game.require_projected_world_read_guard_for_test();}
            let before_context=format!("{:?}",game.current_trigger_context);
            // Exercise warm cache, then replacement requiring invalidation,
            // then absent-key and unknown-data-index factories.
            for stage in 0..4 {
                if stage==1 {install(&mut game.effect_registry,"OUTPUT",true,9,calls.clone());}
                if stage==3 {install(&mut game.effect_registry,"UNKNOWN-OUTPUT",true,9,calls.clone());}
                let id=match stage {2=>"MISSING",3=>"UNKNOWN-OUTPUT",_=>"OUTPUT"};
                let before=calls.load(Ordering::SeqCst);
                if marked {
                    rejected(||{let _=game.registry_effects_record(id,card);},
                        "closed-list projected world reached unproven factory: registry_effects");
                    assert_eq!(calls.load(Ordering::SeqCst),before);rejected_count+=1;
                } else {
                    let result=game.registry_effects_record(id,card);
                    assert_eq!(result.is_some(),stage!=2);
                    assert_eq!(calls.load(Ordering::SeqCst),before+usize::from(stage==1||stage==3));
                }
            }
            assert_eq!(prior.compiled_origin_for_derived("OUTPUT",&retained).is_some(),old_compiled);
            assert!(game.effect_registry.compiled_origin_for_derived("OUTPUT",&retained).is_none());
            game.memory=3;let vector=retained.shared_effects();
            if marked {
                rejected(||vector[0].process.as_ref().unwrap()(
                    &mut EffectContext::new(&mut game,card,None,seat)),
                    "closed-list projected context reached unproven callback");
                assert_eq!(game.memory,3);rejected_count+=1;
            } else {
                vector[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game,card,None,seat));
                assert_eq!(game.memory,7);ordinary+=1;
            }
            assert_eq!(format!("{:?}",game.current_trigger_context),before_context);cases+=1;
        }}
    }}}
    assert_eq!((cases,rejected_count,ordinary),(48,120,24));
}
