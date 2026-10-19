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

 let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().dsl_card("BT7-059").unwrap().dsl_card("BT7-058").unwrap()

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
fn first_selected_physical_card_stays_in_its_origin_until_second_choice_finishes() {
 for actor in [0,1] { for order in [0,1] { for from_trash in [false,true] {
  let mut r=setup(actor);let host=r.place_stack(actor,&["BT7-063"]);
  let (first_id,second_id)=if order==0{("BT7-058","BT7-059")}else{("BT7-059","BT7-058")};
  if from_trash {r.inject_trash(actor,first_id);} else {r.add_to_hand(actor,first_id);}
  r.add_to_hand(actor,second_id);let hand=r.hand_size(actor);let trash=r.trash_size(actor);
  fire(&mut r,host);branch(&mut r,actor,order);first(&mut r,actor);
  assert!(r.pending_selection_view().is_some(),"second physical choice remains available");
  assert_eq!(stack(&r,actor),["BT7-063"],"collect both choices before placing either card");
  assert_eq!(r.hand_size(actor),hand);assert_eq!(r.trash_size(actor),trash);
  let parked=r.game.clone();
  for cloned in [false,true] {r.game=parked.clone();if cloned{r.game=r.game.clone();}first(&mut r,actor);
   assert!(r.pending_selection_view().is_none());assert_eq!(stack(&r,actor),[first_id,second_id,"BT7-063"]);
  }
 }}}
}

#[test]
fn actual_deadlyaxemon_inherited_dp_waits_until_collection_completes() {
 for actor in [0,1] {
  let mut r=setup(actor);let host=r.place_stack(actor,&["BT7-063"]);
  r.add_to_hand(actor,"BT7-059");r.add_to_hand(actor,"BT7-058");
  assert_eq!(r.effective_dp(host),Some(7000));fire(&mut r,host);branch(&mut r,actor,1);first(&mut r,actor);
  assert!(r.pending_selection_view().is_some());
  assert_eq!(r.effective_dp(host),Some(7000),"real BT7-059 inherited aura cannot exist before placement");
  let parked=r.game.clone();for decline in [false,true]{r.game=parked.clone();let action=if decline{PASS}else{*r.pending_selection_view().unwrap().valid_action_ids.iter().find(|a|**a!=PASS).unwrap()};act(&mut r,actor,action);
   assert!(r.pending_selection_view().is_none());assert_eq!(r.effective_dp(host),Some(9000));
   assert_eq!(stack(&r,actor),if decline{vec!["BT7-059","BT7-063"]}else{vec!["BT7-059","BT7-058","BT7-063"]});
  }
 }
}

#[test]
fn collected_dual_name_physical_card_is_excluded_without_early_zone_movement() {
 for actor in [0,1] {
  let mut r=setup(actor);let host=r.place_stack(actor,&["BT7-063"]);
  r.add_to_hand(actor,"BT7-DUAL");r.add_to_hand(actor,"BT7-059");let hand=r.hand_size(actor);
  fire(&mut r,host);branch(&mut r,actor,0);first(&mut r,actor);
  assert_eq!(r.hand_size(actor),hand,"physical exclusion cannot be implemented by prematurely moving the chosen card");
  let choices=r.pending_selection_view().unwrap().valid_action_ids.iter().filter(|a|**a!=PASS).count();
  assert_eq!(choices,1,"only the distinct DeadlyAxemon remains legal for the second role");
  let parked=r.game.clone();for cloned in [false,true]{r.game=parked.clone();if cloned{r.game=r.game.clone();}first(&mut r,actor);assert_eq!(stack(&r,actor),["BT7-DUAL","BT7-059","BT7-063"]);assert_eq!(r.hand_size(actor),hand-2);}
 }
}


struct OneSourceObserver;
impl digimon_engine::effect::CardEffect for OneSourceObserver {
 fn effects(&self,card:digimon_engine::card_source::CardHandle)->Vec<digimon_engine::effect::Effect>{
  vec![digimon_engine::effect::Effect::on_add_digivolution_cards(card)
   .condition(|ctx|ctx.added_source_cards().len()==1)
   .process(|ctx|ctx.gain_memory(1)).build()]
 }
}
#[test]
fn lone_dual_name_card_skips_empty_second_role_and_places_once_with_one_event() {
 for actor in [0,1] {for order in [0,1] {for from_trash in [false,true] {
  let mut r=setup(actor);r.game.memory=3;let host=r.place_stack(actor,&["BT7-063"]);
  r.register_effect("BT7-FILL",std::sync::Arc::new(OneSourceObserver));r.place_on_field(actor,"BT7-FILL",None);
  let physical=if from_trash {r.inject_trash(actor,"BT7-DUAL");r.game.player(actor).trash.last().unwrap().handle()}
      else {r.add_to_hand(actor,"BT7-DUAL");r.game.player(actor).hand.last().unwrap().handle()};
  fire(&mut r,host);branch(&mut r,actor,order);
  assert_eq!(r.pending_selection_view().unwrap().valid_action_ids.iter().filter(|a|**a!=PASS).count(),1);
  let before=r.game.clone();
  for cloned in [false,true] {
   r.game=before.clone();if cloned {r.game=r.game.clone();}first(&mut r,actor);
   assert!(r.pending_selection_view().is_none(),"empty second named-role selection must finish without another action");
   assert_eq!(stack(&r,actor),["BT7-DUAL","BT7-063"]);
   assert_eq!(r.game.player(actor).battle_area[0].card_sources[0].handle(),physical);
   assert!(!r.game.player(actor).hand.iter().chain(r.game.player(actor).trash.iter()).any(|c|c.handle()==physical));
   assert_eq!(r.game.memory,4,"exactly one source-added event contains the one selected physical card");
  }
 }}}
}
