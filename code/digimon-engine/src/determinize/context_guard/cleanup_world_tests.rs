use crate::{debug_runner::{DebugRunner,make_test_card},game::{Game,SecurityPlayReceipt},
 card_source::CardHandle,enums::GamePhase,trigger_context::TriggerContext,
 effect_context::EffectContext,dsl_cards::{bindings::Bindings,step::{cleanup_aborted_steps,run_steps,RunOutcome}}};
use digimon_dsl::compiled::{CompiledStep,CompiledPredicate,CompiledReplacementCause};
use std::panic::{catch_unwind,AssertUnwindSafe};
fn steps(kind:u8,seat:u8)->Vec<CompiledStep>{
 let step=match kind{
  0=>CompiledStep::ReturnReceiptFinalize{receipt:7},
  1=>CompiledStep::SecurityPlacementCancelFinalize{card:7},
  2=>CompiledStep::SecurityPlacementSuccessFinalize{card:7,bind_as:"unused".into()},
  3=>CompiledStep::FieldSourceFinalize{receipt:7,bind_as:"unused".into(),source_anchor:None,source_breeding:false,rebind:vec![]},
  4=>CompiledStep::DeleteReceiptFinalize{receipt:7,cost_credit:None},
  5=>CompiledStep::PaidDeletionPreventionFinalize{receipt:7,payer:(seat,7,7)},
  6=>CompiledStep::PaidDeletionBatchPreventionFinalize{receipt:7,payer:(seat,7,7),members:vec![],
   filter:CompiledPredicate::default(),protect_all:false,cause:CompiledReplacementCause::Cost,cause_player:Some(seat)},
  7=>CompiledStep::SecurityPlayFinalize{receipt:7,player:seat,card:7},
  _=>CompiledStep::SourcePlayFinalize{receipt:7,card:7,bind_as:None},
 };
 vec![step,CompiledStep::GainMemory(9)]
}
fn fixture(seat:u8)->Game{
 let mut r=DebugRunner::builder().add_card(make_test_card("CLEAN-WORLD","Cleanup"))
  .deck(0,&["CLEAN-WORLD";20]).deck(1,&["CLEAN-WORLD";20]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let g=&mut r.game;
 g.return_receipts=vec![(7,vec![]),(99,vec![]),(7,vec![])];
 g.field_source_receipts=vec![(7,Some(CardHandle(7))),(99,Some(CardHandle(99)))];
 g.deletion_receipts=vec![(7,vec![]),(99,vec![]),(7,vec![])];
 g.security_placement_receipts=vec![(7,false),(99,true),(7,true)];
 g.source_play_receipts=vec![(7,CardHandle(7),None),(99,CardHandle(99),None)];
 // None slots exercise cancellation/removal without claiming populated-origin restoration.
 g.prepared_source_plays=vec![(7,None),(99,None)];
 for (id,player,card) in [(7,seat,7),(7,1-seat,7),(7,seat,99),(99,seat,7)]{
  g.security_play_receipts.push(SecurityPlayReceipt{id,player,card:CardHandle(card),original_index:0,committed:None});
 }
 g.dsl_clause_aborted=true;
 r.game
}
fn snapshot(g:&Game)->String{
 format!("{:?}",(g.return_receipts.iter().map(|(id,_)|*id).collect::<Vec<_>>(),
 &g.field_source_receipts,g.deletion_receipts.iter().map(|(id,_)|*id).collect::<Vec<_>>(),
 &g.security_placement_receipts,&g.source_play_receipts,
 g.prepared_source_plays.iter().map(|(id,p)|(*id,p.is_some())).collect::<Vec<_>>(),
 g.security_play_receipts.iter().map(|r|(r.id,r.player,r.card,r.original_index)).collect::<Vec<_>>()))
}
fn expected(g:&mut Game,kind:u8,seat:u8){
 match kind{
  0=>g.return_receipts=vec![(99,vec![])],
  1|2=>g.security_placement_receipts=vec![(7,false),(99,true)],
  3=>g.field_source_receipts=vec![(99,Some(CardHandle(99)))],
  4|5|6=>g.deletion_receipts=vec![(99,vec![])],
  7=>{g.security_play_receipts=vec![
    SecurityPlayReceipt{id:7,player:1-seat,card:CardHandle(7),original_index:0,committed:None},
    SecurityPlayReceipt{id:7,player:seat,card:CardHandle(99),original_index:0,committed:None},
    SecurityPlayReceipt{id:99,player:seat,card:CardHandle(7),original_index:0,committed:None}];},
  _=>{g.source_play_receipts=vec![(99,CardHandle(99),None)];g.prepared_source_plays=vec![(99,None)];}
 }
}
fn caller(g:&mut Game,mode:u8){
 g.current_trigger_context=match mode{0=>None,1=>Some(TriggerContext::default()),_=>{
 let mut c=TriggerContext::default();let mut child=TriggerContext::default();
 child.simultaneous_play_members.push(TriggerContext::default());c.simultaneous_play_members.push(child);Some(c)}};
}
fn panic_at(error:Box<dyn std::any::Any+Send>,runner:bool){
 let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,format!("closed-list projected world reached unproven continuation: {}",if runner {"run_steps_with_runtime"}else{"cleanup_aborted_steps"}));
}
fn matrix(runner:bool)->usize{
 let mut cases=0;
 for seat in [0,1]{for kind in 0..9{for mode in 0..3{for cloned in [false,true]{for marked in [false,true]{
  let original=fixture(seat);let mut g=if cloned{original.clone()}else{original};caller(&mut g,mode);
  let retained=steps(kind,seat).clone();let before=snapshot(&g);let context=format!("{:?}",g.current_trigger_context);
  let mut expected_game=g.clone();expected(&mut expected_game,kind,seat);
  if marked{g.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||{
   if runner{let mut bindings=Bindings::new();bindings.insert_literal("keep",17);
    let out=run_steps(&retained,&mut EffectContext::new(&mut g,CardHandle(0),None,seat),&mut bindings);
    assert!(matches!(out,RunOutcome::Synchronous));assert_eq!(bindings.get_literal("keep"),Some(17));
   }else{cleanup_aborted_steps(&mut g,&retained);}
  }));
  if marked{panic_at(result.expect_err("marked cleanup must reject"),runner);assert_eq!(snapshot(&g),before);}
  else{result.unwrap();assert_eq!(snapshot(&g),snapshot(&expected_game));}
  assert_eq!(g.memory,3);assert_eq!(g.current_phase,GamePhase::Main);assert!(g.pending_selection.is_none());
  assert!(g.dsl_clause_aborted);assert_eq!(format!("{:?}",g.current_trigger_context),context);cases+=1;
 }}}}}
 cases
}
#[test]fn direct_cleanup_rejects_world_before_all_nine_receipt_match_arms(){assert_eq!(matrix(false),216);}
#[test]fn aborted_step_runner_rejects_world_before_receipt_cleanup(){assert_eq!(matrix(true),216);}
#[test]fn empty_ignored_and_unmatched_cleanup_still_rejects_world(){
 let mut cases=0;
 for seat in [0,1]{for shape in 0..3{for marked in [false,true]{
  let mut g=fixture(seat);let before=snapshot(&g);
  let body=match shape{0=>vec![],1=>vec![CompiledStep::GainMemory(9)],_=>vec![CompiledStep::ReturnReceiptFinalize{receipt:1000}]};
  if marked{g.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||cleanup_aborted_steps(&mut g,&body)));
  if marked{panic_at(result.expect_err("unknown cleanup shape must reject"),false);}else{result.unwrap();}
  assert_eq!(snapshot(&g),before);assert_eq!(g.memory,3);cases+=1;
 }}}
 assert_eq!(cases,12);
}
