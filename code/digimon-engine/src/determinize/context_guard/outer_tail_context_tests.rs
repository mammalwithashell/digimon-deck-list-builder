use crate::{
 action::space::HAND_EFFECT_START,card_source::CardHandle,
 debug_runner::{DebugRunner,make_test_card},dsl_cards::{bindings::Bindings,raw_rust::EngineRawRustRegistry,
 step::{run_steps,RunOutcome,StepRuntime,drain_dsl_outer_tail,drain_dsl_outer_tail_with_bindings,
 drain_or_rewrap_pending_tail,drain_or_rewrap_pending_tail_with_bindings}},
 effect_context::EffectContext,enums::{EffectSourceKind,GamePhase},game::Game,
 selection::{PendingSelection,SelectionKind},trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,atomic::{AtomicUsize,Ordering}}};

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
 let mut r=DebugRunner::builder().add_card(make_test_card("TAIL","Tail"))
  .deck(0,&["TAIL";20]).deck(1,&["TAIL";20]).hand(seat,&["TAIL"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let card=r.game.players[seat as usize].hand[0].handle();
 r.game.current_trigger_context=context(mode,90);
 r.game.current_dna_origin=Some(false);
 r.game.effect_source_player=Some(1-seat);r.game.effect_source_card=None;r.game.effect_source_permanent=None;
 (r.game,card)
}
fn raw_prompt(g:&mut Game,card:CardHandle,seat:u8,calls:Arc<AtomicUsize>) {
 g.pending_selection=Some(PendingSelection{
  card_choice:None,kind:SelectionKind::EffectChoice,selecting_player:seat,previous_phase:GamePhase::Main,
  valid_action_ids:vec![HAND_EFFECT_START],is_optional:false,prompt:"Existing raw prompt".into(),effect_choices:None,
  source_card:card,source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:None,
  callback:Box::new(move |g,_|{calls.fetch_add(1,Ordering::SeqCst);g.memory+=1;}),on_decline:None,
 });g.current_phase=GamePhase::EffectChoice;
}
fn structured_prompt(g:&mut Game,card:CardHandle,seat:u8) {
 let steps=vec![CompiledStep::SelectEffectChoice{labels:vec!["A".into(),"B".into()],legal_when:None,
  bind_as:Some("choice".into()),prompt:"Existing VM prompt".into(),prompt_key:None},CompiledStep::GainMemory(1)];
 let mut bindings=Bindings::new();
 assert!(matches!(run_steps(&steps,&mut EffectContext::new(g,card,None,seat),&mut bindings),RunOutcome::Parked));
}
fn callback_ptr(g:&Game)->Option<usize> {
 g.pending_selection.as_ref().map(|p| &*p.callback as *const _ as *const () as usize)
}
fn snapshot(g:&Game)->String {
 format!("{:?}|{:?}",(g.memory,g.current_phase,g.current_dna_origin,g.dsl_clause_aborted,
  &g.current_trigger_context,&g.pending_selection,&g.pending_selection_resume,&g.dsl_outer_tail,
  g.game_over,g.selection_resolution_depth,g.draining_deferred),
  (g.effect_source_player,g.effect_source_card,g.effect_source_permanent))
}
fn panic_at(error:Box<dyn std::any::Any+Send>,lexical:bool) {
 let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,format!("closed-list projected context reached unproven continuation: {}",
  if lexical {"drain_or_rewrap_pending_tail_with_bindings"}else{"drain_dsl_outer_tail_with_bindings"}));
}
// shape0 absent ambient tail / empty lexical input, shape1 empty ambient tail /
// nonempty lexical input, shape2 nonempty ambient tail. Pending uses shape0 empty
// and shape1 nonempty for both entries, always an actual saved/input tail.
fn matrix(lexical:bool,pending:bool)->usize {
 let mut cases=0;let mut rejects=0;let mut resolutions=0;
 let shapes=if pending||lexical {2}else{3};
 for seat in [0,1] {for mode in 0..5 {for cloned in [false,true] {for with_bindings in [false,true] {
  for shape in 0..shapes {for vm in 0..if pending {2}else{1} {for capture in 0..if lexical {5}else{1} {
   let marked=mode>=2||(lexical&&capture>=2);
   let (mut original,card)=fixture(seat,mode);
   if pending&&vm==1 {
    // Park through the strict runner first; restore the marked caller before
    // Game clone and direct drain entry. Saved VM context remains unmarked.
    let caller=original.current_trigger_context.take();
    structured_prompt(&mut original,card,seat);
    original.current_trigger_context=caller;
   }
   let body_calls=Arc::new(AtomicUsize::new(0));let seen=body_calls.clone();
   let want=if !lexical&&with_bindings {29}else{17};
   let mut raw=EngineRawRustRegistry::new();
   raw.register_step("observe",move |ctx,b|{
    seen.fetch_add(1,Ordering::SeqCst);
    assert_eq!(ctx.game.current_dna_origin,Some(true));
    assert_eq!(ctx.override_selecting_player(),None);
    assert_eq!(b.get_literal("keep"),Some(want));
    if lexical {
     assert_eq!(ctx.game.current_trigger_context.as_ref().and_then(|c|c.security_check_face_generation),if capture==0 {None}else{Some(7)});
     assert_eq!(ctx.game.effect_source_player,Some(seat));assert_eq!(ctx.game.effect_source_card,Some(card));
    }
    ctx.game.memory=want as i16;b.insert_literal("observed",101);
   });
   let runtime=StepRuntime::new(Arc::new(raw)).with_dna_origin(Some(true)).with_source_kind(EffectSourceKind::Digimon);
   let nonempty=if pending||lexical {shape==1}else{shape==2};
   let steps=if nonempty {
    if pending {vec![CompiledStep::GainMemory(2)]}
    else {vec![CompiledStep::RawRust{fn_name:"observe".into(),consumes:vec![],binds:vec![]}]}
   }else{vec![]};
   let mut saved=Bindings::new();saved.insert_literal("keep",17);
   let mut overlay=Bindings::new();overlay.insert_literal("keep",29);
   if !lexical&&(pending||shape!=0) {original.dsl_outer_tail=Some((steps.clone(),saved.clone(),runtime.clone()));}
   let mut g=if cloned {original.clone()}else{original};
   let primary_calls=Arc::new(AtomicUsize::new(0));
   if pending&&vm==0 {raw_prompt(&mut g,card,seat,primary_calls.clone());}
   let captured=context(capture,7);
   assert!(!g.projected_world_reads_guarded(),"WORLD stays unset in context-only fixtures");
   let before=snapshot(&g);let ptr=callback_ptr(&g);
   let caller_context=format!("{:?}",g.current_trigger_context);
   let attribution=(g.effect_source_player,g.effect_source_card,g.effect_source_permanent);
   let outcome=if lexical {
    catch_unwind(AssertUnwindSafe(|| {
     if with_bindings {drain_or_rewrap_pending_tail_with_bindings(&mut g,card,None,seat,steps.clone(),saved.clone(),runtime.clone(),captured.clone())}
     else {drain_or_rewrap_pending_tail(&mut g,card,None,seat,steps.clone(),saved.clone(),runtime.clone(),captured.clone());None}
    }))
   } else {
    let mut ctx=EffectContext::new(&mut g,card,None,seat);
    ctx.set_override_selecting_player(Some(1-seat));
    let out=catch_unwind(AssertUnwindSafe(|| {
     if with_bindings {drain_dsl_outer_tail_with_bindings(&mut ctx,Some(&overlay));}
     else {drain_dsl_outer_tail(&mut ctx);}
     None::<Bindings>
    }));
    if marked {assert_eq!(ctx.override_selecting_player(),Some(1-seat));}
    else {assert_eq!(ctx.override_selecting_player(),if !pending&&shape==0 {Some(1-seat)}else{None});}
    out
   };
   if marked {
    panic_at(outcome.expect_err("CONTEXT tail entry must reject before mutation"),lexical);
    assert_eq!(snapshot(&g),before);assert_eq!(callback_ptr(&g),ptr);
    assert_eq!(body_calls.load(Ordering::SeqCst),0);assert_eq!(primary_calls.load(Ordering::SeqCst),0);rejects+=1;
   } else {
    let returned=outcome.unwrap();
    if lexical&&with_bindings {
     assert_eq!(returned.is_some(),!pending);
     if let Some(b)=returned {assert_eq!(b.get_literal("keep"),Some(17));assert_eq!(b.get_literal("observed"),nonempty.then_some(101));}
    }
    assert_eq!(g.current_dna_origin,Some(false));
    assert_eq!(format!("{:?}",g.current_trigger_context),caller_context);
    assert_eq!((g.effect_source_player,g.effect_source_card,g.effect_source_permanent),attribution);
    assert!(g.dsl_outer_tail.is_none());
    if pending {
     assert_eq!(g.memory,3);let p=g.pending_selection.as_ref().unwrap();
     assert_eq!(p.selecting_player,seat);let action=p.valid_action_ids[0];
     g.resolve_selection(seat,action).unwrap();
     assert!(g.pending_selection.is_none());assert!(g.pending_selection_resume.is_none());
     assert_eq!(g.memory,if nonempty {6}else{4});
     assert_eq!(primary_calls.load(Ordering::SeqCst),usize::from(vm==0));resolutions+=1;
    } else {
     assert_eq!(g.memory,if nonempty {want as i16}else{3});
     assert_eq!(body_calls.load(Ordering::SeqCst),usize::from(nonempty));
    }
   }
   assert_eq!(g.current_dna_origin,Some(false));cases+=1;
  }}}}}}}
 let expected=if lexical {if pending {800}else{400}}else if pending {160}else{120};
 let expected_rejects=if lexical {if pending {672}else{336}}else if pending {96}else{72};
 assert_eq!(cases,expected);assert_eq!(rejects,expected_rejects);
 assert_eq!(resolutions,if pending {if lexical {128}else{64}}else{0});cases
}
#[test]fn ambient_tail_entry_rejects_before_take_merge_override_and_scope(){assert_eq!(matrix(false,false),120);}
#[test]fn lexical_tail_entry_rejects_before_context_attribution_and_empty_return(){assert_eq!(matrix(true,false),400);}
#[test]fn ambient_tail_entry_rejects_before_raw_or_structured_prompt_composition(){assert_eq!(matrix(false,true),160);}
#[test]fn lexical_tail_entry_rejects_before_raw_or_structured_prompt_composition(){assert_eq!(matrix(true,true),800);}
