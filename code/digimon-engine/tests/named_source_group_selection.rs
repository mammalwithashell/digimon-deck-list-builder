//! Named roles are assigned to distinct physical sources, including aliases.
//! BASELINE env uses the closest old primary-name selector to expose real mask failures.
use digimon_engine::action::{build_action_mask,space::PASS};
use digimon_engine::debug_runner::{make_test_card,DebugRunner};
use digimon_engine::enums::{EffectTiming,GamePhase};
use digimon_engine::selection::TriggerSource;
const YAML:&str=r#"
card: ROLE-HOST
name: Role Host
kind: digimon
level: 5
color: [black]
cost: 7
dp: 7000
effects:
  - when: on_play
    process:
      - select_materials:
          of_permanent: source
          max: 2
          named_groups: [SkullKnightmon, DeadlyAxemon]
          require_max: true
          optional_zero: true
          filter: { name_in: [SkullKnightmon, DeadlyAxemon] }
          bind_as: selected
          prompt: Choose one of each available named role
      - play_from_materials:
          target: source
          source_index: selected
          cost_delta: free
          suspended: true
      - draw: { of: you, count: 1 }
"#;
fn setup(actor:u8,ids:&[&str])->DebugRunner{setup_inner(actor,ids,false)}
fn setup_inner(actor:u8,ids:&[&str],runtime_alias:bool)->DebugRunner{
    let mut yaml=if std::env::var_os("NAMED_SOURCE_GROUP_BASELINE").is_some(){YAML.replace("          named_groups: [SkullKnightmon, DeadlyAxemon]\n          require_max: true","          uniqueness: name\n          min: 2")}else{YAML.to_string()};
    if runtime_alias {yaml=yaml.replace("filter: { name_in: [SkullKnightmon, DeadlyAxemon] }", "filter: {}");}
    let mut sa=make_test_card("ROLE-SA","Skull Form Alpha");sa.also_treated_as=vec!["SkullKnightmon".into()];
    let mut sb=make_test_card("ROLE-SB","Skull Form Beta");sb.also_treated_as=vec!["SkullKnightmon".into()];
    let mut dual=make_test_card("ROLE-DUAL","Dual Named Form");dual.also_treated_as=vec!["SkullKnightmon".into(),"DeadlyAxemon".into()];
    let mut r=DebugRunner::builder().from_dsl_yaml(&yaml).unwrap().add_card(sa).add_card(sb).add_card(dual)
      .add_card(make_test_card("ROLE-A","DeadlyAxemon")).add_card(make_test_card("ROLE-FILL","Role Filler"))
      .deck(0,&["ROLE-FILL";20]).deck(1,&["ROLE-FILL";20]).memory(10).start();
    r.skip_mulligan();r.set_first_player(actor);r.game.current_phase=GamePhase::Main;
    let mut stack=ids.to_vec();stack.push("ROLE-HOST");let host=r.place_stack(actor,&stack);
    if runtime_alias {r.game.player_mut(actor).battle_area[host.index as usize].card_sources[0].also_treated_as.push("SkullKnightmon".into());}
    r.game.enqueue_triggered(EffectTiming::OnPlay,TriggerSource::Permanent(host));r.game.drain_effect_queue();r
}
fn pick(r:&mut DebugRunner,actor:u8,action:u16){assert!(build_action_mask(&r.game,actor)[action as usize]>0.0);r.game.resolve_selection(actor,action).unwrap();}
fn source_action(r:&DebugRunner,actor:u8,position:usize)->Option<u16>{
 let expected=position.to_string();let mask=build_action_mask(&r.game,actor);
 r.game.pending_selection.as_ref()?.card_choice.as_ref()?.view(&r.game,actor).rows.into_iter()
  .find(|row|matches!(row.operation,digimon_engine::owned_card_choice::CardChoiceOperation::CommitCard)
   && row.position.as_deref()==Some(expected.as_str()) && mask[row.action_id as usize]>0.0).map(|row|row.action_id)
}
fn pick_source(r:&mut DebugRunner,actor:u8,position:usize){let action=source_action(r,actor,position).expect("physical source must be offered");pick(r,actor,action);}
fn ids(r:&DebugRunner,actor:u8)->Vec<String>{r.game.player(actor).battle_area.iter().skip(1).map(|p|p.top_card().card_id(&r.game.card_data).to_string()).collect()}
#[test]
fn two_different_primary_names_cannot_both_fill_the_skull_role(){
 for actor in [0,1]{let mut r=setup(actor,&["ROLE-SA","ROLE-SB","ROLE-A"]);let snap=r.game.clone();
  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}
   pick_source(&mut r,actor,0);let mask=build_action_mask(&r.game,actor);
   assert!(source_action(&r,actor,1).is_none(),"second Skull alias cannot fill DeadlyAxemon role");
   assert!(source_action(&r,actor,2).is_some());assert_eq!(mask[PASS as usize],0.0);
   pick_source(&mut r,actor,2);assert_eq!(ids(&r,actor),["ROLE-SA","ROLE-A"]);
   assert!(r.game.player(actor).battle_area.iter().skip(1).all(|p|p.is_suspended));
  }
 }
}
#[test]
fn dual_named_first_card_keeps_both_complementary_role_choices_in_clones(){
 for actor in [0,1]{let mut r=setup(actor,&["ROLE-DUAL","ROLE-SA","ROLE-A"]);pick_source(&mut r,actor,0);let snap=r.game.clone();
  for next in [1,2]{r.game=snap.clone();let m=build_action_mask(&r.game,actor);assert!(source_action(&r,actor,0).is_none());assert!(source_action(&r,actor,1).is_some()&&source_action(&r,actor,2).is_some());assert_eq!(m[PASS as usize],0.0);
   pick_source(&mut r,actor,next);assert_eq!(ids(&r,actor).len(),2);
  }
 }
}
#[test]
fn a_single_available_role_requires_one_choice_and_preserves_each_alias_alternative(){
 for actor in [0,1]{let mut r=setup(actor,&["ROLE-SA","ROLE-SB"]);let m=build_action_mask(&r.game,actor);assert!(r.pending_selection_view().is_some(),"one available named role must remain playable");assert!(source_action(&r,actor,0).is_some()&&source_action(&r,actor,1).is_some());let snap=r.game.clone();
  for next in [0,1]{r.game=snap.clone();pick_source(&mut r,actor,next);assert!(r.pending_selection_view().is_none());assert_eq!(ids(&r,actor).len(),1);}
 }
}
#[test]
fn a_single_dual_named_physical_card_is_one_pick_not_two(){
 for actor in [0,1]{let mut r=setup(actor,&["ROLE-DUAL"]);assert!(r.pending_selection_view().is_some());pick_source(&mut r,actor,0);assert!(r.pending_selection_view().is_none());assert_eq!(ids(&r,actor),["ROLE-DUAL"]);}
}
#[test]
fn declining_zero_keeps_all_sources_and_runs_the_unconditional_tail_once(){
 for actor in [0,1]{let mut r=setup(actor,&["ROLE-SA","ROLE-A"]);let hand=r.hand_size(actor);let snap=r.game.clone();
  for cloned in [false,true]{r.game=snap.clone();if cloned{r.game=r.game.clone();}pick(&mut r,actor,PASS);assert!(r.pending_selection_view().is_none());assert_eq!(r.battle_area_size(actor),1);assert_eq!(r.game.player(actor).battle_area[0].card_sources.len(),3);assert_eq!(r.hand_size(actor),hand+1);}
 }
}

#[test]
fn physical_source_runtime_alias_participates_in_named_role_matching(){
 for actor in [0,1]{let mut r=setup_inner(actor,&["ROLE-FILL","ROLE-A"],true);let m=build_action_mask(&r.game,actor);assert!(source_action(&r,actor,0).is_some(),"runtime Skull identity must be offered");let snap=r.game.clone();
  for first in [0,1]{r.game=snap.clone();pick_source(&mut r,actor,first);assert!(r.pending_selection_view().is_some(),"two effective names require both sources");pick_source(&mut r,actor,1-first);assert_eq!(ids(&r,actor).len(),2);}
 }
}
