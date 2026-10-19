//! Full BT7-063 source-backed behavior; source role/entry primitives have separate tests.

use digimon_engine::action::{build_action_mask,space::{PASS,REPLACEMENT_ACCEPT,SOURCE_SELECT_START}};

use digimon_engine::debug_runner::{make_test_card,DebugRunner};

use digimon_engine::enums::{EffectTiming,GamePhase};

use digimon_engine::selection::{SelectionKind,TriggerSource};

use digimon_engine::permanent::PermanentHandle;

fn setup(actor:u8)->DebugRunner{

 let path=std::env::var_os("BT7_063_CANDIDATE_YAML").map(std::path::PathBuf::from).unwrap_or_else(||std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("cards/bt7/BT7-063.yaml"));

 let mut yaml=std::fs::read_to_string(path).unwrap();

 if std::env::var_os("BT7_063_BASELINE").is_some(){yaml=yaml.split("effects:").next().unwrap().to_string();}

 let mut dual=make_test_card("BT7-DUAL","Dual Named Witness");dual.also_treated_as=vec!["SkullKnightmon".into(),"DeadlyAxemon".into()];

 let mut skull=make_test_card("BT7-SKULL","SkullKnightmon");skull.dp=Some(3000);

 let mut axe=make_test_card("BT7-AXE","DeadlyAxemon");axe.dp=Some(6000);

 let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().dsl_card("BT7-059").unwrap()

 .add_card(skull).add_card(axe).add_card(dual)

 .add_card(make_test_card("BT7-FILL","Unrelated Witness")).deck(0,&["BT7-FILL";20]).deck(1,&["BT7-FILL";20]).memory(10).start();

 r.skip_mulligan();r.set_first_player(actor);r.game.current_phase=GamePhase::Main;r

}

fn act(r:&mut DebugRunner,actor:u8,a:u16){assert!(build_action_mask(&r.game,actor)[a as usize]>0.0,"actual legal decision {a}");r.game.resolve_selection(actor,a).unwrap();}

fn first(r:&mut DebugRunner,actor:u8){let a=*r.pending_selection_view().expect("expected physical choice").valid_action_ids.iter().find(|a|**a!=PASS).unwrap();act(r,actor,a);}

fn branch(r:&mut DebugRunner,actor:u8,index:usize){let p=r.pending_selection_view().expect("placement order choice");let a=p.valid_action_ids[index];act(r,actor,a);}

fn fire(r:&mut DebugRunner,h:PermanentHandle){r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(h));r.game.drain_effect_queue();}

fn stack(r:&DebugRunner,actor:u8)->Vec<String>{r.game.player(actor).battle_area[0].card_sources.iter().map(|c|c.card_id(&r.game.card_data).to_string()).collect()}

#[test]

fn on_play_mixes_hand_and_trash_and_exposes_both_source_orders_in_clones(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-063"]);r.add_to_hand(actor,"BT7-SKULL");r.inject_trash(actor,"BT7-AXE");fire(&mut r,h);let snap=r.game.clone();

  for order in [0,1]{r.game=snap.clone();branch(&mut r,actor,order);first(&mut r,actor);let mid=r.game.clone();

   for cloned in [false,true]{r.game=mid.clone();if cloned{r.game=r.game.clone();}first(&mut r,actor);assert!(r.pending_selection_view().is_none());let expected=if order==0{vec!["BT7-SKULL","BT7-AXE","BT7-063"]}else{vec!["BT7-AXE","BT7-SKULL","BT7-063"]};assert_eq!(stack(&r,actor),expected);assert_eq!(r.trash_size(actor),0);}

  }

 }

}

#[test]

fn on_play_may_skip_first_named_card_and_place_only_the_other(){

 for actor in [0,1]{for order in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-063"]);r.add_to_hand(actor,"BT7-SKULL");r.add_to_hand(actor,"BT7-AXE");let hand=r.hand_size(actor);fire(&mut r,h);branch(&mut r,actor,order);act(&mut r,actor,PASS);first(&mut r,actor);assert!(r.pending_selection_view().is_none());assert_eq!(r.hand_size(actor),hand-1);assert_eq!(stack(&r,actor),[if order==0{"BT7-AXE"}else{"BT7-SKULL"},"BT7-063"]);}}

}

#[test]

fn on_play_decline_changes_no_zones_and_single_dual_name_card_cannot_be_used_twice(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-063"]);r.add_to_hand(actor,"BT7-DUAL");let hand=r.hand_size(actor);fire(&mut r,h);let snap=r.game.clone();branch(&mut r,actor,2);assert_eq!(r.hand_size(actor),hand);assert_eq!(stack(&r,actor),["BT7-063"]);

  for order in [0,1]{r.game=snap.clone();branch(&mut r,actor,order);first(&mut r,actor);assert!(r.pending_selection_view().is_none());assert_eq!(stack(&r,actor),["BT7-DUAL","BT7-063"]);assert_eq!(r.hand_size(actor),hand-1);}

 }

}

#[test]

fn would_delete_requires_both_named_sources_and_the_original_deletion_follows_suspended_entry(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);let memory=r.game.memory;r.game.delete_permanent_with_effects(h);assert_eq!(r.pending_selection_view().expect("immediate pre-deletion choice").kind,SelectionKind::Replacement);assert_eq!(r.battle_area_size(actor),1);assert_eq!(r.trash_size(actor),0);act(&mut r,actor,REPLACEMENT_ACCEPT);let snap=r.game.clone();

  for initial in [0,1]{r.game=snap.clone();act(&mut r,actor,SOURCE_SELECT_START+initial);assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],0.0,"Q1623 cannot stop after only one name");assert_eq!(r.battle_area_size(actor),1,"choices complete before either card plays");let mid=r.game.clone();

   for cloned in [false,true]{r.game=mid.clone();if cloned{r.game=r.game.clone();}act(&mut r,actor,SOURCE_SELECT_START+(1-initial));assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),2);assert!(r.game.player(actor).battle_area.iter().all(|p|p.is_suspended));assert_eq!(r.trash_size(actor),1);assert_eq!(r.game.player(actor).trash[0].card_id(&r.game.card_data),"BT7-063");assert_eq!(r.game.memory,memory);}

  }

 }

}

#[test]

fn decline_pre_deletion_does_not_play_sources_or_cancel_the_deletion(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);r.game.delete_permanent_with_effects(h);assert!(r.pending_selection_view().is_some());let snap=r.game.clone();

  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}act(&mut r,actor,PASS);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),0);assert_eq!(r.trash_size(actor),3);}

 }

}

#[test]

fn a_single_available_name_can_play_and_another_permanents_deletion_cannot_trigger_this_card(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-063"]);let other=r.place_stack(actor,&["BT7-FILL"]);r.game.delete_permanent_with_effects(other);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),1);r.game.delete_permanent_with_effects(h);assert_eq!(r.pending_selection_view().expect("single name may play").kind,SelectionKind::Replacement);act(&mut r,actor,REPLACEMENT_ACCEPT);first(&mut r,actor);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),1);assert_eq!(r.game.player(actor).battle_area[0].top_card().card_id(&r.game.card_data),"BT7-SKULL");assert!(r.game.player(actor).battle_area[0].is_suspended);}

}



#[test]

fn prohibited_named_source_is_excluded_and_remaining_playable_name_is_still_optional(){

 use digimon_engine::enums::{Expiry,ModifierType};use digimon_engine::modifiers::PlayerModifierEntry;

 for actor in [0,1]{for ceiling in [3000,6000]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);r.game.modifiers.add_player_modifier(actor,PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonWithDPAtMost,ceiling,Expiry::Permanent,None,1-actor));r.game.delete_permanent_with_effects(h);

  if ceiling==6000{assert!(r.pending_selection_view().is_none(),"no legal play, so no activation offer");assert_eq!(r.battle_area_size(actor),0);assert_eq!(r.trash_size(actor),3);continue;}

  assert_eq!(r.pending_selection_view().unwrap().kind,SelectionKind::Replacement);let snap=r.game.clone();

  for decline in [false,true]{r.game=snap.clone();act(&mut r,actor,if decline{PASS}else{REPLACEMENT_ACCEPT});if decline{assert_eq!(r.battle_area_size(actor),0);assert_eq!(r.trash_size(actor),3);}else{let m=build_action_mask(&r.game,actor);assert_eq!(m[SOURCE_SELECT_START as usize],0.0);assert!(m[(SOURCE_SELECT_START+1)as usize]>0.0);act(&mut r,actor,SOURCE_SELECT_START+1);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),1);assert_eq!(r.game.player(actor).battle_area[0].top_card().card_id(&r.game.card_data),"BT7-AXE");assert_eq!(r.trash_size(actor),2);}}

 }}

}

#[test]

fn one_remaining_field_slot_keeps_both_required_picks_but_only_first_play_commits(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);let cap=r.game.rules.field_slots as usize;while r.battle_area_size(actor)<cap-1{r.place_on_field(actor,"BT7-FILL",None);}r.game.delete_permanent_with_effects(h);act(&mut r,actor,REPLACEMENT_ACCEPT);let snap=r.game.clone();

  for initial in [0,1]{r.game=snap.clone();act(&mut r,actor,SOURCE_SELECT_START+initial);assert_eq!(build_action_mask(&r.game,actor)[PASS as usize],0.0);act(&mut r,actor,SOURCE_SELECT_START+(1-initial));assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),cap-1,"first entry uses final slot; original deletion then frees its slot");let wanted=if initial==0{"BT7-SKULL"}else{"BT7-AXE"};assert_eq!(r.game.player(actor).battle_area.iter().filter(|p|p.top_card().card_id(&r.game.card_data)==wanted).count(),1);assert_eq!(r.trash_size(actor),2,"unplayed source remains for original deletion");}

 }

}

struct BatchObserver;

impl digimon_engine::effect::CardEffect for BatchObserver{

 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{vec![digimon_engine::effect::Effect::on_add_digivolution_cards(card)

  .condition(|ctx|ctx.added_source_cards().len()==2).process(|ctx|ctx.gain_memory(1)).build()]}

}



struct BothSourcesAtEntry;

impl digimon_engine::effect::CardEffect for BothSourcesAtEntry {

 fn effects(&self, card: digimon_engine::card_source::CardHandle) -> Vec<digimon_engine::effect::Effect> {

  vec![digimon_engine::effect::Effect::on_play(card)

   .condition(|ctx| {

    ["BT7-SKULL", "BT7-AXE"].iter().all(|id| ctx.game.player(ctx.player).battle_area.iter().any(|p| p.top_card().card_id(&ctx.game.card_data)==*id))

   })

   .process(|ctx| { ctx.draw(ctx.player, 1); }).build()]

 }

}

#[test]

fn both_sources_are_present_for_each_simultaneous_on_play_condition() {

 for actor in [0,1] {

  let mut r=setup(actor);

  r.register_effect("BT7-SKULL",std::sync::Arc::new(BothSourcesAtEntry));

  r.register_effect("BT7-AXE",std::sync::Arc::new(BothSourcesAtEntry));

  let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);

  let hand=r.hand_size(actor);

  r.game.delete_permanent_with_effects(h);

  act(&mut r,actor,REPLACEMENT_ACCEPT);

  act(&mut r,actor,SOURCE_SELECT_START);

  act(&mut r,actor,SOURCE_SELECT_START+1);

  r.auto_resolve().unwrap();

  assert_eq!(r.hand_size(actor),hand+2,"both On Play conditions must observe the complete simultaneous entry");

 }

}

#[test]

fn mixed_zone_placements_emit_one_two_card_event_after_both_selections_complete(){

 for actor in [0,1]{let mut r=setup(actor);r.game.memory=3;let h=r.place_stack(actor,&["BT7-063"]);r.register_effect("BT7-FILL",std::sync::Arc::new(BatchObserver));r.place_on_field(actor,"BT7-FILL",None);r.add_to_hand(actor,"BT7-SKULL");r.inject_trash(actor,"BT7-AXE");fire(&mut r,h);branch(&mut r,actor,0);first(&mut r,actor);assert!(r.pending_selection_view().is_some());assert_eq!(r.game.memory,3,"added-source event waits for remaining physical choice");let snap=r.game.clone();

  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}first(&mut r,actor);assert!(r.pending_selection_view().is_none());assert_eq!(r.game.memory,4,"one event contains both sources, including across clone-resume");}

 }

}



#[test]

fn accepting_activation_still_allows_zero_sources_before_any_pick(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-AXE","BT7-063"]);r.game.delete_permanent_with_effects(h);act(&mut r,actor,REPLACEMENT_ACCEPT);let snap=r.game.clone();

  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}act(&mut r,actor,PASS);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),0);assert_eq!(r.trash_size(actor),3);}

 }

}

#[test]

fn on_play_with_no_named_hand_or_trash_card_exposes_no_empty_activation(){

 for actor in [0,1]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-063"]);r.add_to_hand(actor,"BT7-FILL");fire(&mut r,h);assert!(r.pending_selection_view().is_none());assert_eq!(stack(&r,actor),["BT7-063"]);}

}



#[test]

fn runtime_named_source_gate_offers_playable_identity_and_suppresses_blocked_or_unrelated_sources(){

 use digimon_engine::enums::{Expiry,ModifierType};use digimon_engine::modifiers::PlayerModifierEntry;

 for actor in [0,1]{for mode in [0,1,2]{let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-FILL","BT7-063"]);

  if mode!=0{r.game.player_mut(actor).battle_area[0].card_sources[0].also_treated_as.push("SkullKnightmon".into());}

  if mode==2{r.game.modifiers.add_player_modifier(actor,PlayerModifierEntry::simple(ModifierType::CannotPlayDigimonByEffect,0,Expiry::Permanent,None,1-actor));}

  r.game.delete_permanent_with_effects(h);

  if mode!=1{assert!(r.pending_selection_view().is_none(),"no playable named source cannot offer activation");assert_eq!(r.battle_area_size(actor),0);assert_eq!(r.trash_size(actor),2);continue;}

  assert_eq!(r.pending_selection_view().expect("runtime alias participates in activation gate").kind,SelectionKind::Replacement);let snap=r.game.clone();

  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}act(&mut r,actor,REPLACEMENT_ACCEPT);first(&mut r,actor);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),1);assert_eq!(r.game.player(actor).battle_area[0].top_card().card_id(&r.game.card_data),"BT7-FILL");assert!(r.game.player(actor).battle_area[0].is_suspended);assert_eq!(r.trash_size(actor),1);}

 }}

}


#[test]
fn duplicate_hand_and_trash_copies_offer_distinct_physical_actions() {
 use digimon_engine::action::space::{PLAY_HAND_START,TRASH_EFFECT_START};
 for actor in [0,1] {
  let mut r=setup(actor); let h=r.place_stack(actor,&["BT7-063"]);
  let hand_index=r.hand_size(actor); r.add_to_hand(actor,"BT7-SKULL"); r.add_to_hand(actor,"BT7-SKULL");
  r.inject_trash(actor,"BT7-SKULL"); r.inject_trash(actor,"BT7-AXE");
  let choices=[PLAY_HAND_START+hand_index as u16,PLAY_HAND_START+hand_index as u16+1,TRASH_EFFECT_START];
  let handles=[r.game.player(actor).hand[hand_index].handle(),r.game.player(actor).hand[hand_index+1].handle(),r.game.player(actor).trash[0].handle()];
  fire(&mut r,h);branch(&mut r,actor,0);
  let mask=build_action_mask(&r.game,actor);for a in choices {assert_eq!(mask[a as usize],1.0);}
  assert_eq!(r.pending_selection_view().unwrap().valid_action_ids.iter().filter(|a|**a!=PASS).count(),3);
  assert_eq!(mask[(TRASH_EFFECT_START+1)as usize],0.0);
  let saved=r.game.clone();
  for (a,physical) in choices.into_iter().zip(handles) {
   r.game=saved.clone();act(&mut r,actor,a);
   assert_eq!(stack(&r,actor),["BT7-063"],"first physical pick is collected, not placed early");
   let axe_index=r.game.player(actor).trash.iter().position(|c|c.card_id(&r.game.card_data)=="BT7-AXE").unwrap();
   act(&mut r,actor,TRASH_EFFECT_START+axe_index as u16);
   assert!(r.pending_selection_view().is_none());assert_eq!(stack(&r,actor),["BT7-SKULL","BT7-AXE","BT7-063"]);
   assert_eq!(r.game.player(actor).battle_area[0].card_sources[0].handle(),physical);
   assert_eq!(r.game.player(actor).hand.iter().chain(r.game.player(actor).trash.iter()).filter(|c|c.card_id(&r.game.card_data)=="BT7-SKULL").count(),2);
  }
 }
}
#[test]
fn duplicate_and_dual_name_sources_expose_every_valid_two_role_pair_only() {
 for actor in [0,1] {
  let mut r=setup(actor);let h=r.place_stack(actor,&["BT7-SKULL","BT7-SKULL","BT7-AXE","BT7-DUAL","BT7-063"]);
  let physical:Vec<_>=r.game.player(actor).battle_area[0].card_sources.iter().map(|c|c.handle()).collect();
  r.game.delete_permanent_with_effects(h);act(&mut r,actor,REPLACEMENT_ACCEPT);
  let initial=build_action_mask(&r.game,actor);for i in 0..4 {assert_eq!(initial[(SOURCE_SELECT_START+i)as usize],1.0);}assert_eq!(initial[(SOURCE_SELECT_START+4)as usize],0.0);
  let saved=r.game.clone();
  for first_pick in 0..4u16 {
   r.game=saved.clone();act(&mut r,actor,SOURCE_SELECT_START+first_pick);
   let expected:Vec<u16>=match first_pick {0|1=>vec![2,3],2=>vec![0,1,3],3=>vec![0,1,2],_=>unreachable!()};
   let mask=build_action_mask(&r.game,actor);assert_eq!(mask[PASS as usize],0.0);
   for i in 0..5u16 {assert_eq!(mask[(SOURCE_SELECT_START+i)as usize],if expected.contains(&i){1.0}else{0.0});}
   let mid=r.game.clone();
   for second in expected {
    r.game=mid.clone();act(&mut r,actor,SOURCE_SELECT_START+second);
    assert!(r.pending_selection_view().is_none(),"maximum two physical sources");
    let got:Vec<_>=r.game.player(actor).battle_area.iter().map(|p|p.top_card().handle()).collect();
    assert_eq!(got.len(),2);assert!(got.contains(&physical[first_pick as usize]));assert!(got.contains(&physical[second as usize]));
    assert!(r.game.player(actor).battle_area.iter().all(|p|p.is_suspended));assert_eq!(r.trash_size(actor),3);
   }
  }
 }
}
#[test]
fn printed_metadata_and_black_level_four_cost_three_route_are_preserved() {
 use digimon_engine::{action::space::encode_digivolve,enums::{CardColor,CardKind}};
 for actor in [0,1] {
  let mut r=setup(actor);
  let data=r.game.card_data.iter().find(|c|c.card_id=="BT7-063").unwrap();
  assert_eq!(data.card_name,"DarkKnightmon");assert_eq!(data.card_kind,CardKind::Digimon);assert_eq!(data.level,Some(5));assert_eq!(data.play_cost,7);assert_eq!(data.dp,Some(7000));assert_eq!(data.colors,vec![CardColor::Black]);
  for trait_name in ["Ultimate","Virus","Dark Knight"] {assert!(data.traits.iter().any(|v|v==trait_name));}
  // Real BT7-059 carries the printed black Lv.4 base required by this route.
  // Its effects are irrelevant to a direct evolution from a prepared field.
  let h=r.place_on_field(actor,"BT7-059",Some(0));
  let index=r.hand_size(actor);r.add_to_hand(actor,"BT7-063");let hand=r.hand_size(actor);r.game.memory=10;
  let a=encode_digivolve(index as u16,h.index as u16);assert_eq!(build_action_mask(&r.game,actor)[a as usize],1.0);r.game.decode_action(a,actor);
  assert!(r.pending_selection_view().is_none());assert_eq!(r.game.memory,7);assert_eq!(r.hand_size(actor),hand);
  assert_eq!(r.game.player(actor).battle_area[0].top_card().card_id(&r.game.card_data),"BT7-063");
 }
}

struct BeforeSecondSourceEntry;
impl digimon_engine::effect::CardEffect for BeforeSecondSourceEntry {
 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect> {
  vec![digimon_engine::effect::Effect::when_permanent_would_play(card)
   .name("Second pre-play checks prior simultaneous source")
   .replacement_condition(|ctx,subject| {
    let digimon_engine::replacement::ReplacementSubject::Card(card,_) = subject else {return false;};
    ctx.game.card_data_for_handle(*card).is_some_and(|c|c.card_id=="BT7-AXE")
     && ctx.game.player(ctx.player).battle_area.iter().any(|p|p.top_card().card_id(&ctx.game.card_data)=="BT7-SKULL")
   })
   .replacement_process(|ctx|ctx.effect.gain_memory(1)).build()]
 }
}
#[test]
fn simultaneous_source_preplay_windows_finish_before_either_source_enters() {
 let mut r=setup(0);r.game.memory=3;
 r.register_effect("BT7-FILL",std::sync::Arc::new(BeforeSecondSourceEntry));
 let h=r.place_stack(0,&["BT7-SKULL","BT7-AXE","BT7-063"]);
 r.place_on_field(0,"BT7-FILL",None);
 r.game.delete_permanent_with_effects(h);act(&mut r,0,REPLACEMENT_ACCEPT);
 act(&mut r,0,SOURCE_SELECT_START);act(&mut r,0,SOURCE_SELECT_START+1);
 r.auto_resolve().unwrap();
 assert_eq!(r.battle_area_size(0),3);
 assert_eq!(r.game.memory,3,"second pre-play must not see first simultaneous entrant already on field");
}
