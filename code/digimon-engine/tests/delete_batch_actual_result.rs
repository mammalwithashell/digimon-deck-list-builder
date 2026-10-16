//! Actual deletion-result logging, source driver EX3-012 no-deletion fallback.
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::enums::{EffectTiming,ModifierType,Expiry,GamePhase};
use digimon_engine::selection::TriggerSource;
use digimon_engine::modifiers::ModifierEntry;
const DRIVER:&str=r#"
card: DELETE-RESULT
name: Deletion result driver
kind: digimon
color: [red]
level: 6
cost: 12
dp: 12000
effects:
  - when: on_play
    process:
      - delete_all_permanents:
          over: { of: opponent, kind: digimon, zone: [battle_area] }
          selector: lowest_dp
      - if:
          condition: { effect_deleted_any_opponent_digimon: false }
          then:
            - draw: { of: you, count: 2 }
          else:
            - draw: { of: you, count: 1 }
"#;
fn setup()->DebugRunner {
 let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
 for (id,dp) in [("LOW",3000),("HIGH",8000)] {
  let mut c=make_test_card(id,id);c.dp=Some(dp);b=b.add_card(c);
 }
 let mut r=b.deck(0,&["HIGH";20]).deck(1,&["HIGH";20]).memory(10).start();
 r.skip_mulligan();r.game.current_phase=GamePhase::Main;r.place_on_field(0,"DELETE-RESULT",None);r
}
fn trigger(r:&mut DebugRunner){
 let h=digimon_engine::permanent::PermanentHandle{player:0,index:0};
 r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(h));r.game.drain_effect_queue();
}
#[test]
fn prevented_lowest_deletion_runs_failure_branch_without_deleting_higher_target(){
 let mut r=setup();let low=r.place_on_field(1,"LOW",None);r.place_on_field(1,"HIGH",None);
 r.game.modifiers.add(low,ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,1));
 let n=r.hand_size(0);trigger(&mut r);
 assert_eq!(r.battle_area_size(1),2);assert_eq!(r.trash_size(1),0);
 assert_eq!(r.hand_size(0)-n,2,"attempted deletion is not an actual deletion");
}
#[test]
fn empty_and_partially_successful_batches_report_actual_any_deletion(){
 for partial in [false,true] {
  let mut r=setup();
  if partial {
   let low=r.place_on_field(1,"LOW",None);r.place_on_field(1,"LOW",None);r.place_on_field(1,"HIGH",None);
   r.game.modifiers.add(low,ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,1));
  }
  let n=r.hand_size(0);trigger(&mut r);
  assert_eq!(r.hand_size(0)-n,if partial {1}else{2});
  assert_eq!(r.trash_size(1),usize::from(partial));
 }
}
struct CancelDeletion;
impl digimon_engine::effect::CardEffect for CancelDeletion {
 fn effects(&self,c:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_be_deleted(c).optional().name("Declinable deletion protection")
   .replacement_process(|ctx|ctx.cancel()).build()]
 }
}
#[test]
fn deferred_replacement_result_precedes_fallback_and_survives_clone(){
 let mut r=setup();r.register_effect("LOW",std::sync::Arc::new(CancelDeletion));r.place_on_field(1,"LOW",None);
 let n=r.hand_size(0);trigger(&mut r);
 assert!(r.pending_selection_view().is_some());
 assert_eq!(r.hand_size(0),n,"fallback waits for the attempted deletion to finish");
 let snapshot=r.game.clone();
 for protect in [false,true] {for cloned in [false,true] {
  r.game=snapshot.clone();if cloned {r.game=r.game.clone();}
  let action=if protect {digimon_engine::action::space::REPLACEMENT_ACCEPT}else{digimon_engine::action::space::PASS};
  assert!(digimon_engine::action::build_action_mask(&r.game,1)[action as usize]>0.0);
  r.game.resolve_selection(1,action).unwrap();
  assert_eq!(r.battle_area_size(1),usize::from(protect));
  assert_eq!(r.hand_size(0)-n,if protect {2}else{1},"actual completed result, protection={protect}");
 }}
}
#[test]
fn deleting_tamer_is_not_deleting_an_opponent_digimon() {
 let driver=DRIVER.replace("kind: digimon, zone", "kind: tamer, zone").replace("          selector: lowest_dp\n", "");
 let mut t=make_test_card("TAMER","Tamer");t.card_kind=digimon_engine::enums::CardKind::Tamer;t.dp=None;t.level=None;
 let mut r=DebugRunner::builder().from_dsl_yaml(&driver).unwrap().add_card(t).add_card(make_test_card("HIGH","High")).deck(0,&["HIGH";20]).deck(1,&["HIGH";20]).memory(10).start();
 r.skip_mulligan();r.game.current_phase=GamePhase::Main;r.place_on_field(0,"DELETE-RESULT",None);r.place_on_field(1,"TAMER",None);
 let n=r.hand_size(0);trigger(&mut r);
 assert_eq!(r.battle_area_size(1),0);assert_eq!(r.trash_size(1),1);
 assert_eq!(r.hand_size(0)-n,2,"actual Tamer deletion does not satisfy a Digimon result predicate");
}
struct ReplayDeletedCard;
impl digimon_engine::effect::CardEffect for ReplayDeletedCard {
 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::on_deletion(card).name("Replay the deleted physical card").process(move |ctx|{
   let player=ctx.player;
   if let Some(index)=ctx.game.player(player).trash.iter().position(|c|c.handle()==card){
    ctx.play_from_trash_with_cost(player,index,digimon_engine::enums::CostDelta::Free);
   }
  }).build()]
 }
}
#[test]
fn on_deletion_replay_does_not_erase_the_completed_deletion_result(){
 let mut r=setup();r.register_effect("LOW",std::sync::Arc::new(ReplayDeletedCard));let low=r.place_on_field(1,"LOW",None);
 let physical=r.game.player(1).battle_area[low.index as usize].top_card().handle();let n=r.hand_size(0);trigger(&mut r);
 assert_eq!(r.battle_area_size(1),1);assert_eq!(r.trash_size(1),0);
 assert_eq!(r.game.player(1).battle_area[0].top_card().handle(),physical);
 assert_eq!(r.hand_size(0)-n,1,"a replayed card still was deleted by the original operation");
}
struct CancelAndDeleteUnrelated;
impl digimon_engine::effect::CardEffect for CancelAndDeleteUnrelated {
 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_be_deleted(card).name("Protect self, delete another").optional()
   .replacement_condition(move |ctx,subject| matches!(subject,digimon_engine::replacement::ReplacementSubject::Permanent(h) if ctx.game.player(h.player).battle_area.get(h.index as usize).and_then(|p|p.card_sources.last()).map(|c|c.handle())==Some(card)))
   .replacement_process(|ctx|{
    ctx.cancel();
    let owner=ctx.effect.player;
    let other=ctx.effect.game.player(owner).battle_area.iter().position(|p|p.top_card().card_id(&ctx.effect.game.card_data)=="HIGH");
    if let Some(index)=other {ctx.effect.delete_permanent(digimon_engine::permanent::PermanentHandle{player:owner,index:index as u8});}
   }).build()]
 }
}
#[test]
fn replacement_own_deletion_does_not_credit_cancelled_original_operation(){
 let mut r=setup();r.register_effect("LOW",std::sync::Arc::new(CancelAndDeleteUnrelated));r.place_on_field(1,"LOW",None);r.place_on_field(1,"HIGH",None);
 let n=r.hand_size(0);trigger(&mut r);let snapshot=r.game.clone();
 for cloned in [false,true] {
  r.game=snapshot.clone();if cloned {r.game=r.game.clone();}
  let actor=r.game.pending_selection.as_ref().unwrap().selecting_player;
  let accept=digimon_engine::action::space::REPLACEMENT_ACCEPT;
  assert!(digimon_engine::action::build_action_mask(&r.game,actor)[accept as usize]>0.0);
  r.game.resolve_selection(actor,accept).unwrap();
  assert!(r.game.pending_selection.is_none());assert_eq!(r.battle_area_size(1),1);assert_eq!(r.trash_size(1),1);
  assert_eq!(r.game.player(1).battle_area[0].top_card().card_id(&r.game.card_data),"LOW");
  assert_eq!(r.hand_size(0)-n,2,"nested replacement deletion has its own operation identity");
 }
}
#[test]
fn direct_single_step_logs_only_committed_rows_including_tamers(){
 use digimon_engine::{dsl_cards::{bindings::Bindings,step::run_step},effect_context::EffectContext};
 use digimon_dsl::compiled::{CompiledStep,CompiledBindingRef};
 for protected in [false,true] {
  let mut r=setup();let h=r.place_on_field(1,"LOW",None);
  if protected{r.game.modifiers.add(h,ModifierEntry::simple(ModifierType::CannotBeDestroyedByEffect,0,Expiry::Permanent,1));}
  let source=r.game.player(0).battle_area[0].top_card().handle();
  let mut b=Bindings::default();b.insert_permanent("target",h);
  let mut ctx=EffectContext::new(&mut r.game,source,Some(digimon_engine::permanent::PermanentHandle{player:0,index:0}),0);
  run_step(&CompiledStep::DeletePermanent{target:CompiledBindingRef::Named("target".into())},&mut ctx,&mut b);
  assert_eq!(b.result_log().deleted.len(),usize::from(!protected));
  assert_eq!(r.battle_area_size(1),usize::from(protected));
 }
}

struct CancelOnlySelf;
impl digimon_engine::effect::CardEffect for CancelOnlySelf {
 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::when_would_be_deleted(card).name("Protect only this permanent").optional()
   .replacement_condition(move |ctx,subject| matches!(subject,digimon_engine::replacement::ReplacementSubject::Permanent(h) if ctx.game.player(h.player).battle_area.get(h.index as usize).and_then(|p|p.card_sources.last()).map(|c|c.handle())==Some(card)))
   .replacement_process(|ctx|ctx.cancel()).build()]
 }
}
#[test]
fn protected_member_waits_for_unprotected_tied_lowest_remainder_before_result(){
 for protected_first in [false,true]{
  let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
  for id in ["LOW","LOW2","HIGH"] {let mut c=make_test_card(id,id);c.dp=Some(if id=="HIGH"{8000}else{3000});b=b.add_card(c);}
  let mut r=b.deck(0,&["HIGH";20]).deck(1,&["HIGH";20]).memory(10).start();
  r.skip_mulligan();r.game.current_phase=GamePhase::Main;r.place_on_field(0,"DELETE-RESULT",None);
  r.register_effect("LOW",std::sync::Arc::new(CancelOnlySelf));
  let order=if protected_first {["LOW","LOW2"]}else{["LOW2","LOW"]};
  for id in order {r.place_on_field(1,id,None);}
  let n=r.hand_size(0);trigger(&mut r);assert_eq!(r.hand_size(0),n);
  let snapshot=r.game.clone();
  for cloned in [false,true]{
   r.game=snapshot.clone();if cloned{r.game=r.game.clone();}
   let actor=r.game.pending_selection.as_ref().unwrap().selecting_player;let action=digimon_engine::action::space::REPLACEMENT_ACCEPT;
   assert!(digimon_engine::action::build_action_mask(&r.game,actor)[action as usize]>0.0);r.game.resolve_selection(actor,action).unwrap();
   assert!(r.game.pending_selection.is_none());assert_eq!(r.battle_area_size(1),1);assert_eq!(r.trash_size(1),1);
   assert_eq!(r.hand_size(0)-n,1,"remaining unprotected member deletion belongs to the same effect, order={protected_first}");
  }
 }
}
#[test]
fn each_reparked_batch_member_preserves_finalizer_and_clone(){
 let mut b=DebugRunner::builder().from_dsl_yaml(DRIVER).unwrap();
 for id in ["LOW","LOW2","HIGH"] {let mut c=make_test_card(id,id);c.dp=Some(if id=="HIGH"{8000}else{3000});b=b.add_card(c);}
 let mut r=b.deck(0,&["HIGH";20]).deck(1,&["HIGH";20]).memory(10).start();r.skip_mulligan();r.game.current_phase=GamePhase::Main;r.place_on_field(0,"DELETE-RESULT",None);
 for id in ["LOW","LOW2"] {r.register_effect(id,std::sync::Arc::new(CancelOnlySelf));r.place_on_field(1,id,None);}
 let physical:Vec<_>=r.game.player(1).battle_area.iter().map(|p|p.top_card().handle()).collect();
 let n=r.hand_size(0);trigger(&mut r);let first=r.game.clone();
 // Both self-protection effects are eligible in one simultaneous window.
 // The owner chooses their order before accepting or declining either one.
 for chosen in [0usize,1usize] {for cloned in [false,true]{
  r.game=first.clone();if cloned{r.game=r.game.clone();}
  let pending=r.game.pending_selection.as_ref().expect("two immediate effects require an order choice");
  assert_eq!(pending.kind,digimon_engine::selection::SelectionKind::EffectChoice);
  assert_eq!(pending.selecting_player,1);assert!(!pending.is_optional);
  let choices=pending.effect_choices.as_ref().unwrap();assert_eq!(choices.len(),2);
  for card in &physical {assert_eq!(choices.iter().filter(|x|x.source_card==Some(*card)).count(),1);}
  let order_action=choices.iter().find(|x|x.source_card==Some(physical[chosen])).unwrap().action_id;
  let mask=digimon_engine::action::build_action_mask(&r.game,1);
  for entry in choices {assert!(mask[entry.action_id as usize]>0.0);}
  let accept=digimon_engine::action::space::REPLACEMENT_ACCEPT;let decline=digimon_engine::action::space::PASS;
  assert_eq!(mask[accept as usize],0.0);assert_eq!(mask[decline as usize],0.0);
  r.game.resolve_selection(1,order_action).unwrap();r.game=r.game.clone();
  let pending=r.game.pending_selection.as_ref().expect("chosen effect still requires consent");
  assert_eq!(pending.kind,digimon_engine::selection::SelectionKind::Replacement);
  assert_eq!(pending.source_card,physical[chosen]);assert_eq!(pending.selecting_player,1);
  assert_eq!(r.hand_size(0),n,"ordering an effect does not finalize the batch");
  let mask=digimon_engine::action::build_action_mask(&r.game,1);
  assert!(mask[accept as usize]>0.0 && mask[decline as usize]>0.0);
  r.game.resolve_selection(1,accept).unwrap();
  let pending=r.game.pending_selection.as_ref().expect("second independent replacement is still a legal choice");
  assert_eq!(pending.kind,digimon_engine::selection::SelectionKind::Replacement);
  assert_eq!(pending.source_card,physical[1-chosen]);assert_eq!(pending.selecting_player,1);
  assert_eq!(r.hand_size(0),n,"result waits for every original batch member");
  r.game=r.game.clone();
  let mask=digimon_engine::action::build_action_mask(&r.game,1);
  assert!(mask[accept as usize]>0.0 && mask[decline as usize]>0.0);
  r.game.resolve_selection(1,decline).unwrap();
  assert!(r.game.pending_selection.is_none());assert_eq!(r.battle_area_size(1),1);assert_eq!(r.trash_size(1),1);assert_eq!(r.hand_size(0)-n,1);
  assert_eq!(r.game.player(1).battle_area[0].top_card().handle(),physical[chosen]);
  assert_eq!(r.game.player(1).trash[0].handle(),physical[1-chosen]);
 }}
}
