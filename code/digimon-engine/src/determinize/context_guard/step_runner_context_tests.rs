
use crate::{
 action::space::HAND_EFFECT_START,card_source::CardHandle,
 debug_runner::{DebugRunner,make_test_card},dsl_cards::{bindings::Bindings,step::{run_steps,run_steps_with_runtime,RunOutcome,StepRuntime}},
 effect_context::EffectContext,enums::{EffectSourceKind,GamePhase},game::Game,
 selection::{PendingSelection,SelectionKind},trigger_context::TriggerContext,
};
use digimon_dsl::compiled::{CompiledBindingRef,CompiledStep};
use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,atomic::{AtomicUsize,Ordering}}};
fn fixture(seat:u8)->(Game,CardHandle) {
 let mut r=DebugRunner::builder().add_card(make_test_card("ENTRY","Entry"))
  .deck(0,&["ENTRY";20]).deck(1,&["ENTRY";20]).hand(seat,&["ENTRY"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let card=r.game.players[seat as usize].hand[0].handle();
 r.game.current_dna_origin=Some(false);(r.game,card)
}
fn caller(g:&mut Game,mode:u8,group:bool) {
 let mut c=TriggerContext::default();c.security_check_face_generation=Some(90);
 if group||mode>=3 {c.simultaneous_play_members=vec![TriggerContext::default(),TriggerContext::default()];}
 if mode==2 {c.projected_reference_reads_guarded=true;}
 if mode==3 {c.simultaneous_play_members[0].projected_reference_reads_guarded=true;}
 if mode==4 {let mut deep=TriggerContext::default();deep.projected_reference_reads_guarded=true;c.simultaneous_play_members[1].simultaneous_play_members.push(deep);}
 g.current_trigger_context=if mode==0&&!group {None}else{Some(c)};
}
fn prompt(g:&mut Game,card:CardHandle,seat:u8,calls:Arc<AtomicUsize>) {
 g.pending_selection=Some(PendingSelection{
  card_choice:None,kind:SelectionKind::EffectChoice,selecting_player:seat,previous_phase:GamePhase::Main,
  valid_action_ids:vec![HAND_EFFECT_START],is_optional:false,prompt:"Original prompt".into(),effect_choices:None,
  source_card:card,source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:None,
  callback:Box::new(move |g,_| {calls.fetch_add(1,Ordering::SeqCst);g.memory+=1;}),on_decline:None,
 });g.current_phase=GamePhase::EffectChoice;
}
fn callback_ptr(g:&Game)->Option<usize> {
 g.pending_selection.as_ref().map(|p| &*p.callback as *const _ as *const () as usize)
}
fn snapshot(g:&Game)->String {
 format!("{:?}",(g.memory,g.current_phase,g.current_dna_origin,g.dsl_clause_aborted,
  &g.current_trigger_context,&g.pending_selection,&g.pending_selection_resume,
  &g.dsl_outer_tail,&g.return_receipts,g.game_over,g.selection_resolution_depth,g.draining_deferred))
}
fn run_shape(shape:u8)->usize {
 let mut cases=0;let mut rejected=0;
 for seat in [0,1] {for mode in 0..5 {for cloned in [false,true] {for explicit_runtime in [false,true] {
  let marked=mode>=2;
  let (mut original,card)=fixture(seat);caller(&mut original,mode,shape==4);
  let calls=Arc::new(AtomicUsize::new(0));
  if shape==2 {original.dsl_clause_aborted=true;original.return_receipts=vec![(7,vec![]),(99,vec![])];}
  let mut g=if cloned {original.clone()} else {original};
  if shape==3 {prompt(&mut g,card,seat,calls.clone());}
  let steps=match shape {
   0=>vec![],
   2=>vec![CompiledStep::ReturnReceiptFinalize{receipt:7},CompiledStep::GainMemory(2)],
   4=>vec![CompiledStep::Unsuspend{target:CompiledBindingRef::EventTarget},CompiledStep::GainMemory(2)],
   _=>vec![CompiledStep::GainMemory(2)],
  };
  let retained=steps.clone();let runtime=StepRuntime::default().with_dna_origin(Some(true));
  let mut bindings=Bindings::new();bindings.insert_literal("keep",17);
  assert!(!g.projected_world_reads_guarded());
  let bindings_before=format!("{:?}",bindings);
  let before=snapshot(&g);let ptr=callback_ptr(&g);
  let outcome=catch_unwind(AssertUnwindSafe(|| {
   let mut ctx=EffectContext::new(&mut g,card,None,seat);
   if explicit_runtime {run_steps_with_runtime(&retained,&mut ctx,&mut bindings,&runtime)}
   else {run_steps(&retained,&mut ctx,&mut bindings)}
  }));
  if marked {
   eprintln!("entry shape={shape} mode={mode}: memory={} dna={:?} receipts={:?} pending={}",g.memory,g.current_dna_origin,g.return_receipts.iter().map(|(id,_)|*id).collect::<Vec<_>>(),g.pending_selection.is_some());
   let err=outcome.expect_err("CONTEXT runner must reject at entry even for empty or parked work");
   let msg=err.downcast_ref::<String>().map(String::as_str).or_else(||err.downcast_ref::<&str>().copied()).unwrap_or("");
   assert_eq!(msg,"closed-list projected context reached unproven continuation: run_steps_with_runtime");
   assert_eq!(format!("{:?}",bindings),bindings_before);
   assert_eq!(calls.load(Ordering::SeqCst),0);
   assert_eq!(snapshot(&g),before);assert_eq!(callback_ptr(&g),ptr);rejected+=1;
  } else {
   let out=outcome.unwrap();assert_eq!(g.current_dna_origin,Some(false));
   if shape==3||shape==4 {
    assert!(matches!(out,RunOutcome::Parked));assert_eq!(g.memory,3);
    let p=g.pending_selection.as_ref().unwrap();
    assert_eq!(p.selecting_player,seat);
    if shape==3 {assert_eq!(p.prompt,"Original prompt");assert_ne!(callback_ptr(&g),ptr);}
    else {assert_eq!(p.valid_action_ids.len(),2);assert!(g.pending_selection_resume.is_some());}
    let action=p.valid_action_ids[0];
    g.resolve_selection(seat,action).unwrap();
    assert!(g.pending_selection.is_none());assert!(g.pending_selection_resume.is_none());
    assert_eq!(g.memory,if shape==3 {6}else{5});
    assert_eq!(calls.load(Ordering::SeqCst),usize::from(shape==3));
   } else {
    assert!(matches!(out,RunOutcome::Synchronous));
    assert_eq!(g.memory,if shape==1 {5}else{3});
    if shape==2 {assert_eq!(g.return_receipts.iter().map(|(id,_)|*id).collect::<Vec<_>>(),vec![99]);}
   }
  }
  assert_eq!(bindings.get_literal("keep"),Some(17));
  assert_eq!(g.current_dna_origin,Some(false));cases+=1;
 }}}}
 assert_eq!((cases,rejected),(40,24));cases
}
#[test]fn runner_entry_rejects_empty_and_literal_work_before_dna_scope(){assert_eq!(run_shape(0)+run_shape(1),80);}
#[test]fn runner_entry_rejects_aborted_work_before_dna_scope_and_receipts(){assert_eq!(run_shape(2),40);}
#[test]fn runner_entry_rejects_before_wrapping_an_existing_raw_prompt(){assert_eq!(run_shape(3),40);}
#[test]fn runner_entry_rejects_before_simultaneous_subject_expansion(){assert_eq!(run_shape(4),40);}
