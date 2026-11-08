use crate::{
 card_source::CardHandle,debug_runner::{DebugRunner,make_test_card},effect_context::EffectContext,
 dsl_cards::{bindings::Bindings,step::{StepRuntime,selections::run_tail_preserving_trigger_context}},
 enums::GamePhase,game::Game,permanent::PermanentHandle,trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::panic::{catch_unwind,AssertUnwindSafe};
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
fn fixture(seat:u8,compact:bool)->(Game,CardHandle,Option<PermanentHandle>) {
 let mut r=DebugRunner::builder().add_card(make_test_card("OBL","Obligation"))
  .deck(0,&["OBL";20]).deck(1,&["OBL";20]).hand(seat,&["OBL"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let (card,previous)=if compact {
  r.place_stack(seat,&["OBL"]);let host=r.place_stack(seat,&["OBL"]);
  let card=r.game.players[seat as usize].battle_area[1].card_sources[0].handle();
  let old=r.game.players[seat as usize].battle_area.remove(0);
  r.game.players[seat as usize].put_all_in_trash(old.card_sources);(card,Some(host))
 } else {(r.game.players[seat as usize].hand[0].handle(),None)};
 r.game.current_dna_origin=Some(false);r.game.effect_source_player=Some(1-seat);
 let mut b=Bindings::new();b.insert_literal("old",99);r.game.dsl_resolved_tail_bindings=Some(b);
 (r.game,card,previous)
}
fn snapshot(g:&Game)->String {
 format!("{:?}|{:?}",(g.memory,g.current_phase,g.current_dna_origin,&g.current_trigger_context,
  &g.dsl_resolved_tail_bindings,&g.dsl_outer_tail,&g.pending_selection,&g.pending_selection_resume,
  g.selection_resolution_depth,g.draining_deferred,g.game_over),
  (g.effect_source_player,g.effect_source_card,g.effect_source_permanent))
}
fn matrix(kind:u8) {
 let mut cases=0;let mut rejects=0;let mut controls=0;let mut resolutions=0;
 for seat in [0,1] {for cloned in [false,true] {for compact in [false,true] {
  for caller in if kind==1 {[2,0,1,3,4]}else{[0,1,2,3,4]} {for saved in 0..5 {
   let (mut original,card,previous)=fixture(seat,compact);
   original.current_trigger_context=context(caller,90);
   let captured=context(saved,7);let runtime=StepRuntime::default().with_dna_origin(Some(true));
   let mut tail=vec![];
   if kind==2 {tail.push(CompiledStep::SelectEffectChoice{labels:vec!["A".into(),"B".into()],
    legal_when:None,bind_as:Some("choice".into()),prompt:"Obligation choice".into(),prompt_key:None});}
   if kind!=0 {tail.push(CompiledStep::GainMemory(2));}
   let mut g=if cloned {original.clone()}else{original};
   assert!(!g.projected_world_reads_guarded(),"only context markers are under test");
   let before=snapshot(&g);let caller_before=format!("{:?}",g.current_trigger_context);
   let attrs=(g.effect_source_player,g.effect_source_card,g.effect_source_permanent);
   let mut bindings=Bindings::new();bindings.insert_literal("keep",17);let b_before=format!("{:?}",bindings);
   let mut ctx=EffectContext::new(&mut g,card,previous,seat);
   let result=catch_unwind(AssertUnwindSafe(||run_tail_preserving_trigger_context(
    &mut ctx,captured.clone(),&tail,&mut bindings,&runtime)));
   if caller>=2||saved>=2 {
    let error=result.expect_err("marked current or saved context must reject at entry");
    let message=error.downcast_ref::<String>().map(String::as_str)
     .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
    assert_eq!(message,"closed-list projected context reached unproven continuation: run_tail_preserving_trigger_context");
    assert_eq!(ctx.source_permanent,previous);assert!(!ctx.cost_unpayable);
    assert_eq!(snapshot(ctx.game),before);assert_eq!(format!("{:?}",bindings),b_before);rejects+=1;
   } else {
    result.unwrap();assert_eq!(ctx.source_permanent,previous);
    assert_eq!(format!("{:?}",ctx.game.current_trigger_context),caller_before);
    assert_eq!((ctx.game.effect_source_player,ctx.game.effect_source_card,ctx.game.effect_source_permanent),attrs);
    assert_eq!(ctx.game.memory,if kind==1 {5}else{3});
    let published=ctx.game.dsl_resolved_tail_bindings.as_ref().unwrap();
    assert_eq!(published.get_literal("keep"),Some(17));assert_eq!(published.get_literal("old"),None);
    assert_eq!(ctx.game.pending_selection.is_some(),kind==2);controls+=1;
   }
   drop(ctx);
   if caller<2&&saved<2&&kind==2 {
    if cloned {g=g.clone();}
    assert!(g.pending_selection_resume.is_some());
    let p=g.pending_selection.as_ref().unwrap();assert_eq!(p.selecting_player,seat);
    let action=p.valid_action_ids[0];g.resolve_selection(seat,action).unwrap();
    assert_eq!(g.memory,5);assert!(g.pending_selection.is_none());
    assert!(g.pending_selection_resume.is_none());resolutions+=1;
   }
   assert_eq!(g.current_dna_origin,Some(false));cases+=1;
  }}
 }}}
 assert_eq!((cases,rejects,controls),(200,168,32));
 assert_eq!(resolutions,if kind==2 {32}else{0});
}
#[test]fn empty_tail_rejects_current_or_saved_context_before_publication(){matrix(0);}
#[test]fn native_literal_tail_rejects_current_or_saved_context_before_memory_change(){matrix(1);}
#[test]fn parked_tail_rejects_current_or_saved_context_before_prompt_install(){matrix(2);}
