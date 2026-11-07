use crate::{
 card_source::CardHandle,debug_runner::{DebugRunner,make_test_card},
 dsl_cards::{bindings::Bindings,raw_rust::EngineRawRustRegistry,step::{StepRuntime,selections::run_outer_conts}},
 enums::{EffectSourceKind,GamePhase},game::Game,resume::OuterContinuation,trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,Mutex}};
fn context(mode:u8,generation:u64)->Option<TriggerContext> {
 if mode==0 {return None;}
 let mut c=TriggerContext::default();c.security_check_face_generation=Some(generation);
 if mode==2 {c.projected_reference_reads_guarded=true;}
 if mode==3 {
  let mut member=TriggerContext::default();member.projected_reference_reads_guarded=true;
  c.simultaneous_play_members.push(member);
 }
 if mode==4 {
  let mut member=TriggerContext::default();let mut deep=TriggerContext::default();
  deep.projected_reference_reads_guarded=true;member.simultaneous_play_members.push(deep);
  c.simultaneous_play_members.push(TriggerContext::default());
  c.simultaneous_play_members.push(member);
 }
 Some(c)
}

fn fixture(seat:u8,mode:u8)->(Game,CardHandle) {
 let mut r=DebugRunner::builder().add_card(make_test_card("CONTS","Continuations"))
  .deck(0,&["CONTS";20]).deck(1,&["CONTS";20]).hand(seat,&["CONTS"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let card=r.game.players[seat as usize].hand[0].handle();
 r.game.current_trigger_context=context(mode,90);r.game.current_dna_origin=Some(false);
 r.game.effect_source_player=Some(1-seat);r.game.effect_source_card=None;r.game.effect_source_permanent=None;
 (r.game,card)
}
fn snapshot(g:&Game)->String {
 format!("{:?}|{:?}",(g.memory,g.current_phase,g.current_dna_origin,&g.current_trigger_context,
  &g.dsl_resolved_tail_bindings,&g.dsl_outer_tail,&g.pending_selection,&g.pending_selection_resume,
  g.selection_resolution_depth,g.draining_deferred,g.game_over),
  (g.effect_source_player,g.effect_source_card,g.effect_source_permanent))
}
fn panic_at(error:Box<dyn std::any::Any+Send>) {
 let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,"closed-list projected context reached unproven continuation: run_outer_conts");
}
fn matrix(shape:u8)->usize {
 let mut cases=0;let mut rejects=0;let mut resolutions=0;let mut observations=0;
 let layouts:Vec<(usize,u8)>=if shape==0 {vec![(0,0)]}else{
  let mut v=vec![(3,3),(0,0)];for position in 0..4 {for mark in 1..4 {if (position,mark)!=(3,3){v.push((position,mark));}}}v
 };
 for seat in [0,1] {for mode in 0..5 {for cloned in [false,true] {for published in [true,false] {for &(position,mark) in &layouts {
  let marked=mode>=2||mark>0;
  let (mut original,card)=fixture(seat,mode);
  if published {let mut b=Bindings::new();b.insert_literal("keep",29);b.clear_for_merge("retired");original.dsl_resolved_tail_bindings=Some(b);}
  let hits=Arc::new(Mutex::new(Vec::<usize>::new()));let mut conts=vec![];
  if shape!=0 {
   for i in 0..3usize {
    let seen=hits.clone();let mut raw=EngineRawRustRegistry::new();
    raw.register_step("observe",move |ctx,b|{
     assert_eq!(ctx.game.current_dna_origin,Some(true));
     assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation,Some(7+i as u64));
     assert_eq!(ctx.game.effect_source_player,Some(seat));assert_eq!(ctx.game.effect_source_card,Some(card));
     assert_eq!(b.get_literal("keep"),Some(if i==0 {if published {29}else{17}}else if i==1 {31}else{37}));
     assert_eq!(b.get_literal("crumb"),if i==0 {None}else if i==1 {Some(41)}else{Some(43)});
     assert_eq!(b.get_literal("retired"),if i==0&&!published {Some(5)}else{None});
     let mut seen=seen.lock().unwrap();assert_eq!(seen.len(),i);seen.push(i);
     ctx.game.memory+=(i+1) as i16;
     b.insert_literal("keep",if i==0 {31}else{37});b.insert_literal("crumb",if i==0 {41}else{43});b.clear_for_merge("retired");
    });
    let mut steps=vec![];
    if shape==2&&i==0 {steps.push(CompiledStep::SelectEffectChoice{labels:vec!["A".into(),"B".into()],legal_when:None,
     bind_as:Some("choice".into()),prompt:"Continuation choice".into(),prompt_key:None});}
    steps.push(CompiledStep::RawRust{fn_name:"observe".into(),consumes:vec![],binds:vec![]});
    let mut bindings=Bindings::new();bindings.insert_literal("keep",17);bindings.insert_literal("retired",5);
    let mut captured=TriggerContext::default();captured.security_check_face_generation=Some(7+i as u64);
    let cont=OuterContinuation{tail:Arc::new(steps),bindings,
     runtime:StepRuntime::new(Arc::new(raw)).with_dna_origin(Some(true)).with_source_kind(EffectSourceKind::Digimon),
     trigger_context:Some(captured),source_card:card,source_permanent:None,player:seat};
    conts.push(cont.clone());
    if i==0 {let mut empty=cont;empty.tail=Arc::new(vec![]);conts.push(empty);}
   }
  }
  if mark>0 {
   let generation=conts[position].trigger_context.as_ref().unwrap().security_check_face_generation.unwrap();
   conts[position].trigger_context=context(mark+1,generation);
  }
  let mut g=if cloned {original.clone()}else{original};
  let retained=conts.clone();
  assert!(!g.projected_world_reads_guarded());
  let before=snapshot(&g);let caller=format!("{:?}",g.current_trigger_context);
  let attribution=(g.effect_source_player,g.effect_source_card,g.effect_source_permanent);
  let outcome=catch_unwind(AssertUnwindSafe(||run_outer_conts(&mut g,retained)));
  if marked {
   eprintln!("entry outcome: shape={shape} mode={mode} position={position} mark={mark} memory={} observers={} published={} pending={}",
    g.memory,hits.lock().unwrap().len(),g.dsl_resolved_tail_bindings.is_some(),g.pending_selection.is_some());
   panic_at(outcome.expect_err("CONTEXT continuation preflight must reject"));
   assert_eq!(snapshot(&g),before);assert!(hits.lock().unwrap().is_empty());rejects+=1;
  } else {
   outcome.unwrap();
   assert_eq!(format!("{:?}",g.current_trigger_context),caller);
   assert_eq!((g.effect_source_player,g.effect_source_card,g.effect_source_permanent),attribution);
   if shape==0 {assert_eq!(snapshot(&g),before);}
   else if shape==1 {
    assert_eq!(g.memory,9);assert_eq!(*hits.lock().unwrap(),vec![0,1,2]);assert!(g.dsl_resolved_tail_bindings.is_none());observations+=3;
   } else {
    assert_eq!(g.memory,3);assert!(hits.lock().unwrap().is_empty());assert!(g.dsl_resolved_tail_bindings.is_none());
    assert!(g.pending_selection_resume.is_some());
    if cloned {g=g.clone();}
    let p=g.pending_selection.as_ref().unwrap();assert_eq!(p.selecting_player,seat);let action=p.valid_action_ids[0];
    g.resolve_selection(seat,action).unwrap();
    assert!(g.pending_selection.is_none());assert!(g.pending_selection_resume.is_none());
    assert_eq!(g.memory,9);assert_eq!(*hits.lock().unwrap(),vec![0,1,2]);resolutions+=1;observations+=3;
   }
  }
  assert_eq!(g.current_dna_origin,Some(false));cases+=1;
 }}}}}
 assert_eq!(cases,if shape==0 {40}else{520});assert_eq!(rejects,if shape==0 {24}else{504});
 assert_eq!(resolutions,if shape==2 {16}else{0});assert_eq!(observations,if shape==0 {0}else{48});cases
}
#[test]fn empty_outer_continuations_reject_current_context_before_return(){assert_eq!(matrix(0),40);}
#[test]fn synchronous_outer_continuations_preflight_late_saved_context_before_earlier_bodies(){assert_eq!(matrix(1),520);}
#[test]fn parked_outer_continuations_preflight_all_saved_contexts_before_composition(){assert_eq!(matrix(2),520);}
