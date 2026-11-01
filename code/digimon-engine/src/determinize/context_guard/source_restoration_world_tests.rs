use crate::{debug_runner::{DebugRunner,make_test_card},game::{Game,PendingWouldPlayOrigin,PendingWouldPlayResume,TerminalOutcomeReason},
 card_source::CardHandle,enums::GamePhase,trigger_context::TriggerContext,permanent::PermanentHandle};
use std::panic::{catch_unwind,AssertUnwindSafe};
fn fixture(seat:u8)->(Game,PendingWouldPlayResume,PermanentHandle,u64){
 let mut r=DebugRunner::builder().add_card(make_test_card("REST-SOURCE","Source"))
  .add_card(make_test_card("REST-TOP","Top")).deck(0,&["REST-TOP";20]).deck(1,&["REST-TOP";20])
  .hand(seat,&[]).memory(10).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 r.place_stack(seat,&["REST-TOP"]);let host=r.place_stack(seat,&["REST-SOURCE","REST-TOP"]);
 let card=r.game.player(seat).battle_area[host.index as usize].card_sources[0].handle();
 let identity=r.game.capture_source_play_host_identity(host).unwrap();
 let declaration=PendingWouldPlayResume{player:seat,card,effective_cost:2,
  origin:PendingWouldPlayOrigin::Source{permanent:host,host_identity:identity,source_index:0,suspended:false,receipt:Some(7)},
  effect_initiated:true,play_controller:seat,suppress_on_play:false};
 r.game.prepared_source_plays=vec![(7,None),(99,None)];
 r.game.source_play_receipts=vec![(7,card,None),(99,CardHandle(65535),None)];
 assert_eq!(r.game.prepare_source_play_if_registered(declaration.clone()),Some(true));
 assert_eq!(r.game.memory,8);assert!(r.game.prepared_source_plays[0].1.is_some());
 assert!(!r.game.player(seat).hand.iter().any(|c|c.handle()==card));
 (r.game,declaration,host,identity)
}
fn caller(g:&mut Game,mode:u8){g.current_trigger_context=match mode{0=>None,1=>Some(TriggerContext::default()),_=>{
 let mut c=TriggerContext::default();let mut child=TriggerContext::default();child.simultaneous_play_members.push(TriggerContext::default());
 c.simultaneous_play_members.push(child);Some(c)}};}
fn snapshot(g:&Game)->String{
 let zones=g.players.iter().map(|p|(p.hand.iter().map(|c|c.handle()).collect::<Vec<_>>(),
  p.trash.iter().map(|c|c.handle()).collect::<Vec<_>>(),
  p.battle_area.iter().map(|p|p.card_sources.iter().map(|c|c.handle()).collect::<Vec<_>>()).collect::<Vec<_>>())).collect::<Vec<_>>();
 format!("{:?}",(zones,&g.source_play_receipts,&g.prepared_source_plays,g.memory,g.current_phase))
}
fn occurrences(g:&Game,card:CardHandle)->usize{g.players.iter().map(|p|
 p.hand.iter().chain(p.trash.iter()).chain(p.deck.iter()).chain(p.security.iter()).chain(p.digitama_deck.iter()).filter(|c|c.handle()==card).count()
 +p.battle_area.iter().map(|p|p.card_sources.iter().filter(|c|c.handle()==card).count()).sum::<usize>()
 +p.breeding_area.as_ref().map_or(0,|p|p.card_sources.iter().filter(|c|c.handle()==card).count())).sum()}
fn panic_at(error:Box<dyn std::any::Any+Send>,label:&str){
 let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,format!("closed-list projected world reached unproven continuation: {label}"));
}
#[test]fn actual_populated_preparation_cancellation_and_terminal_route_reject_world(){
 let mut cases=0;
 for seat in [0,1]{for route in 0..3{for mode in 0..3{for cloned in [false,true]{for marked in [false,true]{
  let(original,decl,_,_)=fixture(seat);let mut g=if cloned{original.clone()}else{original};caller(&mut g,mode);
  let before=snapshot(&g);let events=g.events.len();let context=format!("{:?}",g.current_trigger_context);
  if marked{g.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||match route{
   0=>g.cancel_prepared_source_play(7),1=>g.cancel_all_prepared_source_plays(),
   _=>g.finish_game(None,TerminalOutcomeReason::EngineDeclared)}));
  if marked{
   panic_at(result.expect_err("world cancellation must reject"),if route==0{"cancel_prepared_source_play"}else{"cancel_all_prepared_source_plays"});
   assert_eq!(snapshot(&g),before);assert_eq!(g.events.len(),events);
   // finish_game sets game_over before invoking cancellation: not rollback.
   assert_eq!(g.game_over,route==2);assert_eq!(g.winner,None);assert_eq!(g.terminal_outcome_reason,None);
  }else{
   result.unwrap();
   assert_eq!(g.prepared_source_plays.iter().map(|(id,_)|*id).collect::<Vec<_>>(),if route==0{vec![99]}else{vec![]});
   assert_eq!(g.source_play_receipts.iter().map(|(id,_,_)|*id).collect::<Vec<_>>(),if route==2{vec![]}else{vec![99]});
   assert_eq!(g.game_over,route==2);
   if route==2{assert_eq!(g.current_phase,GamePhase::GameOver);assert_eq!(g.events.len(),events+1);}
   else{assert_eq!(g.current_phase,GamePhase::Main);assert_eq!(g.events.len(),events);}
  }
  assert_eq!(g.memory,8);assert_eq!(occurrences(&g,decl.card),1);
  assert!(!g.player(seat).hand.iter().any(|c|c.handle()==decl.card));
  assert_eq!(format!("{:?}",g.current_trigger_context),context);cases+=1;
 }}}}}
 assert_eq!(cases,72);
}
fn restoration(direct:bool)->usize{
 let mut cases=0;
 for seat in [0,1]{for shape in 0..3{for mode in 0..3{for cloned in [false,true]{for marked in [false,true]{
  let(mut original,decl,host,identity)=fixture(seat);
  // Controlled hand-staged restoration state, distinct from live preparation above.
  let card=original.player_mut(seat).battle_area[host.index as usize].card_sources.remove(0);
  original.player_mut(seat).put_in_hand(card);
  if shape==1{let lower=original.player_mut(seat).battle_area.remove(0);original.player_mut(seat).put_all_in_trash(lower.card_sources);}
  if shape==2{let old=original.player_mut(seat).battle_area.remove(host.index as usize);original.player_mut(seat).put_all_in_trash(old.card_sources);}
  let mut g=if cloned{original.clone()}else{original};caller(&mut g,mode);
  let retained=decl.clone();let before=snapshot(&g);let context=format!("{:?}",g.current_trigger_context);
  if marked{g.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||if direct{
   g.restore_source_play_card(seat,retained.card,host,identity,0);
  }else{g.restore_pending_would_play_origin(retained);}));
  if marked{panic_at(result.expect_err("world restoration must reject"),if direct{"restore_source_play_card"}else{"restore_pending_would_play_origin"});assert_eq!(snapshot(&g),before);}
  else{
   result.unwrap();assert!(!g.player(seat).hand.iter().any(|c|c.handle()==decl.card));
   if shape==2{assert_eq!(g.player(seat).trash.last().unwrap().handle(),decl.card);}
   else{let restored=g.resolve_source_play_host_identity(host,identity).unwrap();
    assert_eq!(restored.index,if shape==1{0}else{host.index});
    assert_eq!(g.player(seat).battle_area[restored.index as usize].card_sources[0].handle(),decl.card);}
   assert_eq!(g.source_play_receipts.iter().map(|(id,_,_)|*id).collect::<Vec<_>>(),if direct{vec![7,99]}else{vec![99]});
  }
  assert_eq!(occurrences(&g,decl.card),1);assert_eq!(g.memory,8);assert_eq!(g.current_phase,GamePhase::Main);
  assert_eq!(format!("{:?}",g.current_trigger_context),context);cases+=1;
 }}}}}
 cases
}
#[test]fn captured_origin_restore_checks_world_before_receipt_or_zone_mutation(){assert_eq!(restoration(false),72);}
#[test]fn direct_source_restore_checks_world_before_host_resolution_and_movement(){assert_eq!(restoration(true),72);}
#[test]fn absent_and_empty_cancellation_still_reject_world(){
 let mut cases=0;
 for seat in [0,1]{for all in [false,true]{for marked in [false,true]{
  let(mut g,_,_,_)=fixture(seat);g.prepared_source_plays.clear();let before=snapshot(&g);
  if marked{g.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||if all{g.cancel_all_prepared_source_plays()}else{g.cancel_prepared_source_play(1000)}));
  if marked{panic_at(result.expect_err("empty cancellation still checks world"),if all{"cancel_all_prepared_source_plays"}else{"cancel_prepared_source_play"});}
  else{result.unwrap();}assert_eq!(snapshot(&g),before);cases+=1;
 }}}
 assert_eq!(cases,8);
}
