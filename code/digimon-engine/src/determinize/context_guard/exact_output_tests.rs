//! Exact vector identity is a construction fact, never runtime permission.
use crate::{
    cards::{CardEffectRegistry, keyword_effects}, card_source::CardHandle,
    debug_runner::{make_test_card, DebugRunner}, dsl_cards::raw_rust::EngineRawRustRegistry,
    effect::{CardEffect, Effect}, effect_context::EffectContext, enums::Keyword, game::Game,
    trigger_context::TriggerContext,
};
use std::{panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, atomic::{AtomicUsize, Ordering}}};

fn spec() -> Arc<digimon_dsl::compiled::CompiledCard> {
    let spec: digimon_dsl::CardSpec=serde_yml::from_str(
        "card: EXACT\nname: Exact\nkind: digimon\nlevel: 3\ncolor: [red]\ncost: 3\ndp: 1000\neffects:\n  - kind: raw_rust\n    fn: captured\n").unwrap();
    Arc::new(digimon_dsl::compile::compile(&spec).unwrap())
}
fn effects(card:CardHandle, value:Arc<AtomicUsize>)->Vec<Effect> {
    vec![Effect::on_play(card).name("same-metadata")
        .process(move |ctx| ctx.game.memory=value.load(Ordering::SeqCst) as i16).build()]
}
struct Native {calls:Arc<AtomicUsize>,value:Arc<AtomicUsize>}
impl CardEffect for Native {
    fn effects(&self, card:CardHandle)->Vec<Effect> {
        self.calls.fetch_add(1,Ordering::SeqCst);effects(card,self.value.clone())
    }
}
fn install(registry:&mut CardEffectRegistry,id:&str,compiled:bool,
    calls:Arc<AtomicUsize>,value:Arc<AtomicUsize>) {
    if compiled {
        let mut raw=EngineRawRustRegistry::new();
        raw.register_declarative("captured",move |card| {
            calls.fetch_add(1,Ordering::SeqCst);effects(card,value.clone())
        });
        registry.insert_compiled(id,spec(),Arc::new(raw));
    } else {registry.insert(id,Arc::new(Native{calls,value}));}
}
fn fixture(seat:u8,keyword:bool)->(Game,CardHandle) {
    let mut data=make_test_card("EXACT","Exact");
    if keyword {data.keywords.push(Keyword::Barrier);}
    let mut runner=DebugRunner::builder().add_card(data).hand(seat,&["EXACT"]).start();
    runner.set_first_player(seat);let mut game=runner.game;
    let card=game.players[seat as usize].hand[0].handle();
    game.effect_registry=CardEffectRegistry::new();(game,card)
}
fn rejected(run:impl FnOnce()) {
    let err=catch_unwind(AssertUnwindSafe(run)).expect_err("guard must reject");
    let msg=err.downcast_ref::<String>().map(String::as_str)
        .or_else(||err.downcast_ref::<&str>().copied()).unwrap_or("");
    assert!(msg.contains("closed-list projected context reached unproven callback: process"),"{msg}");
}
#[test]
fn candidate_identity_rejects_equal_independent_vectors_and_wrong_records() {
    let calls=Arc::new(AtomicUsize::new(0));let value=Arc::new(AtomicUsize::new(7));
    let mut original=CardEffectRegistry::new();
    install(&mut original,"EXACT",true,calls.clone(),value.clone());
    let a=original.derive_effects("EXACT",CardHandle(42)).unwrap();
    let b=original.derive_effects("EXACT",CardHandle(42)).unwrap();
    let mut alias=original.clone();
    install(&mut alias,"ALIAS",true,calls.clone(),value.clone());
    let c=alias.derive_effects("ALIAS",CardHandle(42)).unwrap();
    let mut replacement=original.clone();
    install(&mut replacement,"EXACT",true,calls.clone(),value.clone());
    let d=replacement.derive_effects("EXACT",CardHandle(42)).unwrap();
    let mut opaque=CardEffectRegistry::new();
    install(&mut opaque,"EXACT",false,calls.clone(),value.clone());
    let e=opaque.derive_effects("EXACT",CardHandle(42)).unwrap();
    let records=[a,b,c,d,e];
    let vectors:Vec<_>=records.iter().map(|r|r.shared_effects()).collect();
    for vector in &vectors {
        assert_eq!(vector[0].name,"same-metadata");
        assert_eq!(vector[0].source_card,CardHandle(42));
    }
    let cloned=original.clone();
    let registries=[original,cloned,alias,replacement,opaque,CardEffectRegistry::new()];
    assert_eq!(calls.load(Ordering::SeqCst),5);
    let mut matches=0;let mut queries=0;
    for (row,registry) in registries.iter().enumerate() {
        for key in ["EXACT","ALIAS","MISSING"] {
            for (ri,record) in records.iter().enumerate() {
                for (vi,vector) in vectors.iter().enumerate() {
                    let expected=ri==vi && matches!((row,key,ri),
                        (0|1|2,"EXACT",0|1)|(2,"ALIAS",2)|(3,"EXACT",3));
                    let result=registry.compiled_origin_for_output(key,record,vector);
                    assert_eq!(result.is_some(),expected,"registry={row} key={key} record={ri} vector={vi}");
                    if let Some(origin)=result {
                        assert_eq!(origin.direct_raw_factory_requests()[0].name,"captured");matches+=1;
                    }
                    queries+=1;
                }
            }
        }
    }
    assert_eq!((queries,matches),(450,8));
    assert_eq!(calls.load(Ordering::SeqCst),5,"queries execute no factory");
    let mut vector=records[0].shared_effects();
    assert!(Arc::get_mut(&mut vector).is_none());
    let record_clone=records[0].clone();
    assert!(registries[0].compiled_origin_for_output("EXACT",&record_clone,&vector).is_some());
    let weak=Arc::downgrade(&registries[0].get("EXACT").unwrap());
    drop(registries);
    assert!(weak.upgrade().is_none(),"records and vectors do not retain the producer");
    let replacement=CardEffectRegistry::new();
    assert!(replacement.compiled_origin_for_output("EXACT",&record_clone,&vector).is_none());
}
#[test]
fn full_owned_and_keyword_outputs_cannot_borrow_registry_vector_facts() {
    let mut cases=0;let mut queries=0;
    for seat in 0..2 {for cloned in [false,true] {for known in [false,true] {
        for compiled in [false,true] {for keyword in [false,true] {
            let (mut game,card)=fixture(seat,keyword);
            let id=if known {"EXACT"}else{"UNKNOWN-EXACT"};
            let calls=Arc::new(AtomicUsize::new(0));let value=Arc::new(AtomicUsize::new(7));
            install(&mut game.effect_registry,id,compiled,calls.clone(),value);
            let record=game.registry_effects_record(id,card).unwrap();
            if cloned {game=game.clone();}
            let original=record.shared_effects();
            let api=game.registry_effects(id,card).unwrap();
            let full=game.effects_for_card(id,card).unwrap();
            let owned=Arc::new(game.effects_for_card_owned(id,card).unwrap());
            let keywords=Arc::new(keyword_effects::keyword_to_auto_effect(Keyword::Barrier,card));
            assert_eq!(full.len(),1+usize::from(known&&keyword));
            assert_eq!(owned.len(),full.len());
            assert_eq!(full[0].name,original[0].name);
            assert_eq!(owned[0].source_card,original[0].source_card);
            if known&&keyword {
                assert_eq!(full[1].keyword_source,Some(Keyword::Barrier));
                assert_eq!(owned[1].keyword_source,Some(Keyword::Barrier));
            }
            assert_eq!(keywords.len(),1);
            let before=calls.load(Ordering::SeqCst);
            assert_eq!(before,if known {3}else{4});
            for (vector,expected) in [(&original,compiled),(&api,known&&compiled),
                (&full,false),(&owned,false),(&keywords,false)] {
                assert_eq!(game.effect_registry.compiled_origin_for_output(id,&record,vector).is_some(),expected);
                queries+=1;
            }
            assert_eq!(calls.load(Ordering::SeqCst),before);
            cases+=1;
        }}
    }}}
    assert_eq!((cases,queries),(32,160));
}
#[test]
fn mutable_captures_and_replaced_or_moved_callbacks_do_not_gain_permission() {
    let mut cases=0;let mut rejects=0;let mut ordinary=0;let mut replacement_runs=0;
    for seat in 0..2 {for cloned in [false,true] {for compiled in [false,true] {
        for caller in 0..3 {for marked in [false,true] {
            let (mut game,card)=fixture(seat,false);
            let calls=Arc::new(AtomicUsize::new(0));let value=Arc::new(AtomicUsize::new(7));
            install(&mut game.effect_registry,"EXACT",compiled,calls.clone(),value.clone());
            let record=game.registry_effects_record("EXACT",card).unwrap();
            let original=record.shared_effects();
            let mut owned=game.effects_for_card_owned("EXACT",card).unwrap();
            let moved=owned[0].process.take().unwrap();
            let replacement_calls=Arc::new(AtomicUsize::new(0));let count=replacement_calls.clone();
            owned[0].process=Some(Box::new(move |ctx| {
                count.fetch_add(1,Ordering::SeqCst);ctx.game.memory=23;
            }));
            let changed=Arc::new(owned);
            if cloned {game=game.clone();}
            let mut context=TriggerContext::default();
            if caller==2 {
                let mut child=TriggerContext::default();
                child.simultaneous_play_members.push(TriggerContext::default());
                context.simultaneous_play_members.push(child);
            }
            game.current_trigger_context=(caller!=0).then_some(context);
            if marked {game.require_projected_world_read_guard_for_test();}
            let context_before=format!("{:?}",game.current_trigger_context);
            assert_eq!(calls.load(Ordering::SeqCst),2);
            for next_value in [7,11] {
                value.store(next_value,Ordering::SeqCst);
                assert_eq!(game.effect_registry.compiled_origin_for_output("EXACT",&record,&original).is_some(),compiled);
                assert!(game.effect_registry.compiled_origin_for_output("EXACT",&record,&changed).is_none());
                game.memory=3;
                if marked {
                    rejected(||original[0].process.as_ref().unwrap()(
                        &mut EffectContext::new(&mut game,card,None,seat)));
                    assert_eq!(game.memory,3);rejects+=1;
                    rejected(||moved(&mut EffectContext::new(&mut game,card,None,seat)));
                    assert_eq!(game.memory,3);rejects+=1;
                } else {
                    original[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game,card,None,seat));
                    assert_eq!(game.memory,next_value as i16);ordinary+=1;
                    game.memory=3;moved(&mut EffectContext::new(&mut game,card,None,seat));
                    assert_eq!(game.memory,next_value as i16);ordinary+=1;
                }
            }
            // Public replacement is deliberately unwrapped. This direct call
            // demonstrates the boundary, not a legal engine bypass or permission.
            changed[0].process.as_ref().unwrap()(&mut EffectContext::new(&mut game,card,None,seat));
            assert_eq!(game.memory,23);assert_eq!(replacement_calls.load(Ordering::SeqCst),1);
            assert_eq!(calls.load(Ordering::SeqCst),2);
            assert_eq!(format!("{:?}",game.current_trigger_context),context_before);
            replacement_runs+=1;cases+=1;
        }}
    }}}
    assert_eq!((cases,rejects,ordinary,replacement_runs),(48,96,96,48));
}
