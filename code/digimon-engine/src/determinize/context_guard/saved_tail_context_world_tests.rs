use crate::{
 card_source::CardHandle,debug_runner::{DebugRunner,make_test_card},effect_context::EffectContext,
 dsl_cards::{bindings::Bindings,raw_rust::EngineRawRustRegistry,step::{StepRuntime,selections::run_tail_preserving_trigger_context}},
 enums::{EffectSourceKind,GamePhase},game::Game,permanent::PermanentHandle,trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,atomic::{AtomicUsize,Ordering}}};
fn fixture(seat:u8,mode:u8,source:u8)->(Game,CardHandle,Option<PermanentHandle>,Option<PermanentHandle>) {
 let mut r=DebugRunner::builder().add_card(make_test_card("CTX","Context"))
  .deck(0,&["CTX";20]).deck(1,&["CTX";20]).hand(seat,&["CTX"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 r.place_stack(seat,&["CTX"]);let host=r.place_stack(seat,&["CTX"]);
 let (card,previous,live)=match source {
  0=>(r.game.players[seat as usize].hand[0].handle(),None,None),
  1=>(r.game.players[seat as usize].battle_area[1].card_sources[0].handle(),Some(host),Some(host)),
  2=>{
   let card=r.game.players[seat as usize].battle_area[1].card_sources[0].handle();
   let old=r.game.players[seat as usize].battle_area.remove(0);r.game.players[seat as usize].put_all_in_trash(old.card_sources);
   (card,Some(host),Some(PermanentHandle{player:seat,index:0}))
  },
  3=>{
   let old=r.game.players[seat as usize].battle_area.remove(1);let card=old.card_sources[0].handle();
   r.game.players[seat as usize].put_all_in_trash(old.card_sources);(card,Some(host),None)
  },
  _=>{
   let linked=r.game.players[seat as usize].hand.remove(0);let card=linked.handle();
   r.game.players[seat as usize].battle_area[1].linked_cards.push(linked);(card,Some(host),Some(host))
  },
 };
 let mut c=TriggerContext::default();c.security_check_face_generation=Some(90);
 if mode==2 {let mut child=TriggerContext::default();child.simultaneous_play_members.push(TriggerContext::default());c.simultaneous_play_members.push(child);}
 r.game.current_trigger_context=(mode!=0).then_some(c);r.game.current_dna_origin=Some(false);
 r.game.effect_source_player=Some(1-seat);r.game.effect_source_card=None;r.game.effect_source_permanent=None;
 let mut published=Bindings::new();published.insert_literal("old",99);r.game.dsl_resolved_tail_bindings=Some(published);
 (r.game,card,previous,live)
}
fn snapshot(g:&Game)->String {
 format!("{:?}|{:?}",(g.memory,g.current_phase,g.current_dna_origin,&g.current_trigger_context,
  &g.dsl_resolved_tail_bindings,&g.dsl_outer_tail,&g.pending_selection,&g.pending_selection_resume,
  g.selection_resolution_depth,g.draining_deferred,g.game_over),
  (g.effect_source_player,g.effect_source_card,g.effect_source_permanent))
}
fn panic_at(error:Box<dyn std::any::Any+Send>) {
 let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,"closed-list projected world reached unproven continuation: run_tail_preserving_trigger_context");
}
fn matrix(kind:u8)->usize {
 let mut cases=0;let mut rejects=0;let mut observations=0;let mut resolutions=0;
 for seat in [0,1] {for mode in 0..3 {for cloned in [false,true] {for source in 0..5 {for fail_cost in 0..if kind==1 {2}else{1} {for marked in [false,true] {
  let (mut original,card,previous,live)=fixture(seat,mode,source);
  let calls=Arc::new(AtomicUsize::new(0));let seen=calls.clone();let mut raw=EngineRawRustRegistry::new();
  raw.register_step("observe",move |ctx,b|{
   assert_eq!(ctx.source_permanent,live);assert_eq!(ctx.game.effect_source_permanent,live);
   assert_eq!(ctx.game.effect_source_player,Some(seat));assert_eq!(ctx.game.effect_source_card,Some(card));
   assert_eq!(ctx.game.current_trigger_context.as_ref().unwrap().security_check_face_generation,Some(7));
   assert_eq!(ctx.game.current_dna_origin,Some(true));assert_eq!(b.get_literal("keep"),Some(17));
   seen.fetch_add(1,Ordering::SeqCst);ctx.game.memory+=1;b.insert_literal("observed",41);
   if fail_cost==1 {ctx.cost_unpayable=true;}
  });
  let runtime=StepRuntime::new(Arc::new(raw)).with_dna_origin(Some(true)).with_source_kind(EffectSourceKind::Digimon);
  let mut tail=vec![];
  if kind==2 {tail.push(CompiledStep::SelectEffectChoice{labels:vec!["A".into(),"B".into()],legal_when:None,
   bind_as:Some("choice".into()),prompt:"Tail choice".into(),prompt_key:None});}
  if kind!=0 {tail.push(CompiledStep::RawRust{fn_name:"observe".into(),consumes:vec![],binds:vec![]});}
  if kind!=0 {original.dsl_outer_tail=Some((vec![CompiledStep::GainMemory(2)],Bindings::new(),runtime.clone()));}
  let mut captured=TriggerContext::default();captured.security_check_face_generation=Some(7);
  let mut bindings=Bindings::new();bindings.insert_literal("keep",17);let old_bindings=format!("{:?}",bindings);
  let mut g=if cloned {original.clone()}else{original};
  if marked {g.require_projected_world_read_guard_for_test();}
  let before=snapshot(&g);let caller=format!("{:?}",g.current_trigger_context);
  let attribution=(g.effect_source_player,g.effect_source_card,g.effect_source_permanent);
  let mut ctx=EffectContext::new_with_source_kind(&mut g,card,previous,EffectSourceKind::Digimon,seat);
  let outcome=catch_unwind(AssertUnwindSafe(||run_tail_preserving_trigger_context(&mut ctx,Some(captured.clone()),&tail,&mut bindings,&runtime)));
  if marked {
   panic_at(outcome.expect_err("WORLD saved-context entry must reject"));
   assert_eq!(ctx.source_permanent,previous);assert!(!ctx.cost_unpayable);
   assert_eq!(snapshot(ctx.game),before);assert_eq!(format!("{:?}",bindings),old_bindings);
   assert_eq!(calls.load(Ordering::SeqCst),0);rejects+=1;
  } else {
   outcome.unwrap();assert_eq!(ctx.source_permanent,previous);assert_eq!(ctx.cost_unpayable,fail_cost==1);
   assert_eq!(format!("{:?}",ctx.game.current_trigger_context),caller);
   assert_eq!((ctx.game.effect_source_player,ctx.game.effect_source_card,ctx.game.effect_source_permanent),attribution);
   if kind==0 {
    assert_eq!(ctx.game.memory,3);assert_eq!(calls.load(Ordering::SeqCst),0);
    let b=ctx.game.dsl_resolved_tail_bindings.as_ref().unwrap();assert_eq!(b.get_literal("keep"),Some(17));assert_eq!(b.get_literal("old"),None);
   } else if kind==1 {
    assert_eq!(calls.load(Ordering::SeqCst),1);assert_eq!(bindings.get_literal("observed"),Some(41));observations+=1;
    assert_eq!(ctx.game.memory,if fail_cost==1 {4}else{6});assert_eq!(ctx.game.dsl_outer_tail.is_some(),fail_cost==1);
    let b=ctx.game.dsl_resolved_tail_bindings.as_ref().unwrap();
    assert_eq!(b.get_literal("old"),if fail_cost==1 {Some(99)}else{None});
    assert_eq!(b.get_literal("observed"),if fail_cost==1 {None}else{Some(41)});
   } else {
    assert_eq!(ctx.game.memory,3);assert_eq!(calls.load(Ordering::SeqCst),0);assert!(ctx.game.dsl_outer_tail.is_none());
    assert!(ctx.game.pending_selection_resume.is_some());
   }
  }
  drop(ctx);
  if !marked&&kind==2 {
   if cloned {g=g.clone();}
   let p=g.pending_selection.as_ref().unwrap();assert_eq!(p.selecting_player,seat);let action=p.valid_action_ids[0];
   g.resolve_selection(seat,action).unwrap();assert!(g.pending_selection.is_none());assert!(g.pending_selection_resume.is_none());
   assert_eq!(g.memory,6);assert_eq!(calls.load(Ordering::SeqCst),1);resolutions+=1;observations+=1;
  }
  assert_eq!(g.current_dna_origin,Some(false));cases+=1;
 }}}}}}
 let expected=if kind==1 {240}else{120};assert_eq!(cases,expected);assert_eq!(rejects,expected/2);
 assert_eq!(observations,if kind==0 {0}else{expected/2});assert_eq!(resolutions,if kind==2 {60}else{0});cases
}
#[test]fn saved_context_empty_tail_rejects_before_source_and_context_replacement(){assert_eq!(matrix(0),120);}
#[test]fn saved_context_body_rejects_before_resolution_and_preserves_cost_exit_contract(){assert_eq!(matrix(1),240);}
#[test]fn saved_context_parked_tail_rejects_before_resolution_and_preserves_cloned_vm(){assert_eq!(matrix(2),120);}
