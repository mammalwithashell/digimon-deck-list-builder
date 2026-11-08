use crate::{
 action::space::PASS, card_choice_page::CARD_CHOICE_START, card_source::CardHandle,
 debug_runner::{DebugRunner,make_test_card}, game::Game,
 enums::{GamePhase,EffectSourceKind}, selection::{PendingSelection,SelectionKind,SelectionError},
 owned_card_choice::{OwnedCardChoice,ResolvedCardChoice,CardChoiceRecovery},
 dsl_cards::{bindings::Bindings,step::{StepRuntime,selections::abandon_owned_choice}},
 resume::{ResumeStack,ResumeFrame,ResumeProvenance,ResumeSelectKind,ResumeDecline,OuterContinuation},
 trigger_context::TriggerContext,
};
use digimon_dsl::compiled::CompiledStep;
use std::{sync::Arc,panic::{catch_unwind,AssertUnwindSafe}};

fn stack(seat:u8, wrapped:bool)->ResumeStack {
 let stack=ResumeStack {frames:vec![ResumeFrame::RunTail {
  prov:ResumeProvenance {source_card:CardHandle(0),source_permanent:None,
   source_kind:EffectSourceKind::Digimon,controller:seat,override_pin:None},
  select_kind:ResumeSelectKind::EffectChoice{post:None},bind_as:None,
  inner_tail:Arc::new(vec![CompiledStep::ReturnReceiptFinalize{receipt:41},CompiledStep::GainMemory(9)]),
  outer_conts:vec![OuterContinuation {
   tail:Arc::new(vec![CompiledStep::ReturnReceiptFinalize{receipt:42},CompiledStep::GainMemory(9)]),
   bindings:Bindings::new(),runtime:StepRuntime::default(),trigger_context:Some(TriggerContext::default()),
   source_card:CardHandle(0),source_permanent:None,player:seat}],
  bindings:Bindings::new(),runtime:StepRuntime::default(),trigger_context:Some(TriggerContext::default()),
  decline:ResumeDecline::None,
 }]};
 if wrapped {ResumeStack{frames:vec![ResumeFrame::ReplacementBody{inner:Box::new(stack),outer_conts:vec![]}]}} else {stack}
}
fn fixture(seat:u8,typed:bool)->Game {
 let mut r=DebugRunner::builder().add_card(make_test_card("REC-WORLD","Recovery"))
  .deck(0,&["REC-WORLD";20]).deck(1,&["REC-WORLD";20])
  .security(seat,&["REC-WORLD"]).memory(3).start();
 r.skip_mulligan();r.set_first_player(seat);r.set_phase(GamePhase::Main);
 let game=&mut r.game;let card=game.player(seat).security[0].handle();
 let owner=OwnedCardChoice::security(game,seat,vec![0],false).unwrap();
 game.pending_selection=Some(PendingSelection{
  card_choice:Some(owner),kind:SelectionKind::EffectChoice,selecting_player:seat,
  previous_phase:GamePhase::Main,valid_action_ids:vec![CARD_CHOICE_START],is_optional:false,
  prompt:"Impossible owned recovery".into(),effect_choices:None,source_card:card,
  source_permanent:None,source_kind:EffectSourceKind::Digimon,zone_owner:Some(seat),
  callback:Box::new(|_,_|panic!("impossible choice cannot pay")),
  on_decline:Some(Box::new(|_|panic!("impossible recovery is not a paid decline"))),
 });
 game.current_phase=GamePhase::EffectChoice;
 game.resolved_card_choice=Some(ResolvedCardChoice::Security{owner:seat,index:0,card});
 game.return_receipts=vec![(41,vec![]),(42,vec![]),(99,vec![])];
 if typed {game.pending_selection_resume=Some(stack(seat,true));} else {
  game.dsl_outer_tail=Some((vec![CompiledStep::ReturnReceiptFinalize{receipt:41},
   CompiledStep::ReturnReceiptFinalize{receipt:42},CompiledStep::GainMemory(9)],
   Bindings::new(),StepRuntime::default()));
 }
 game.player_mut(seat).security.clear();
 assert_eq!(game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().recovery(game),CardChoiceRecovery::Impossible);
 assert_eq!(game.pending_selection.as_ref().unwrap().card_choice.as_ref().unwrap().valid_action_ids(game),vec![PASS]);
 r.game
}
fn caller(game:&mut Game,mode:u8) {
 game.current_trigger_context=match mode {
  0=>None,1=>Some(TriggerContext::default()),_=>{
   let mut root=TriggerContext::default();let mut child=TriggerContext::default();
   child.simultaneous_play_members.push(TriggerContext::default());
   root.simultaneous_play_members.push(child);Some(root)
  }
 };
}
fn receipts(game:&Game)->Vec<u64>{game.return_receipts.iter().map(|(id,_)|*id).collect()}
fn panic_at(error:Box<dyn std::any::Any+Send>,label:&str){
 let message=error.downcast_ref::<String>().map(String::as_str)
  .or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("");
 assert_eq!(message,format!("closed-list projected world reached unproven continuation: {label}"));
}
#[test]
fn impossible_owned_recovery_rejects_before_pin_prompt_or_receipt_cleanup(){
 let mut cases=0;
 for seat in [0,1]{for typed in [false,true]{for probe in [false,true]{for mode in 0..3{
 for cloned in [false,true]{for marked in [false,true]{
  let original=fixture(seat,typed);let mut game=if cloned{original.clone()}else{original};
  caller(&mut game,mode);let context=format!("{:?}",game.current_trigger_context);
  let pin=game.resolved_card_choice.clone();let depths=(game.draining_deferred,game.selection_resolution_depth);
  let resume=format!("{:?}",game.pending_selection_resume);let tail=format!("{:?}",game.dsl_outer_tail);
  if marked{game.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||if probe{game.resolve_payment_probe_selection(seat,PASS)}
    else{game.resolve_selection(seat,PASS)}));
  if marked{
   panic_at(result.expect_err("world recovery must reject"),"owned_choice_recovery");
   assert_eq!(game.resolved_card_choice,pin);assert!(game.pending_selection.is_some());
   assert_eq!(game.current_phase,GamePhase::EffectChoice);
   assert_eq!(format!("{:?}",game.pending_selection_resume),resume);
   assert_eq!(format!("{:?}",game.dsl_outer_tail),tail);
   assert_eq!(receipts(&game),vec![41,42,99]);
  }else{
   assert_eq!(result.unwrap(),Ok(()));assert!(game.pending_selection.is_none());
   assert!(game.pending_selection_resume.is_none());assert!(game.dsl_outer_tail.is_none());
   assert_eq!(receipts(&game),vec![99]);assert_eq!(game.resolved_card_choice,None);
   assert_eq!(game.current_phase,GamePhase::Main);
  }
  assert_eq!(game.memory,3);assert_eq!((game.draining_deferred,game.selection_resolution_depth),depths);
  assert_eq!(format!("{:?}",game.current_trigger_context),context);cases+=1;
 }}}}}}
 assert_eq!(cases,96);
}
#[test]
fn direct_abandon_rejects_before_inner_or_outer_receipt_release(){
 let mut cases=0;
 for seat in [0,1]{for wrapped in [false,true]{for mode in 0..3{for marked in [false,true]{
  let mut game=fixture(seat,true);let retained=stack(seat,wrapped).clone();caller(&mut game,mode);
  let context=format!("{:?}",game.current_trigger_context);
  if marked{game.require_projected_world_read_guard_for_test();}
  let result=catch_unwind(AssertUnwindSafe(||abandon_owned_choice(&mut game,retained)));
  if marked{panic_at(result.expect_err("direct abandon must reject"),"abandon_owned_choice");
   assert_eq!(receipts(&game),vec![41,42,99]);
  }else{result.unwrap();assert_eq!(receipts(&game),vec![99]);}
  assert_eq!(game.memory,3);assert!(game.pending_selection.is_some());
  assert_eq!(format!("{:?}",game.current_trigger_context),context);cases+=1;
 }}}}
 assert_eq!(cases,24);
}
#[test]
fn impossible_owned_recovery_keeps_invalid_action_errors_before_world_rejection(){
 for seat in [0,1]{for typed in [false,true]{
  let mut game=fixture(seat,typed);game.require_projected_world_read_guard_for_test();
  let pin=game.resolved_card_choice.clone();
  assert_eq!(game.resolve_selection(1-seat,PASS),Err(SelectionError::WrongPlayer));
  assert_eq!(game.resolve_selection(seat,CARD_CHOICE_START),Err(SelectionError::InvalidAction));
  assert_eq!(game.resolved_card_choice,pin);assert_eq!(receipts(&game),vec![41,42,99]);
  assert!(game.pending_selection.is_some());
 }}
}
