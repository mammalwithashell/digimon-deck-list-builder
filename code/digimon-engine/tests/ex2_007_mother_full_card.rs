//! Full Mother card expectations from primary official observation + image.
//! Runtime override permits a declaration-only scratch baseline for behavioral RED.
//! Baseline is test instrumentation and must never be shipped as a card implementation.
use digimon_engine::debug_runner::{DebugRunner,make_test_card};
use digimon_engine::action::{mask::build_action_mask,space::{FIELD_EFFECT_START,FIELD_EFFECT_SLOT_FOR_MAIN,PLAY_HAND_START,PASS,encode_attack}};
use digimon_engine::enums::{CardKind,EffectSourceKind};
use digimon_engine::permanent::PermanentHandle;
fn mother_yaml()->String { std::fs::read_to_string(std::env::var("MOTHER_TEST_YAML").unwrap_or_else(|_|concat!(env!("CARGO_MANIFEST_DIR"),"/cards/ex2/EX2-007.yaml").to_owned())).expect("Mother source or explicit scratch metadata baseline") }
const SEARCHER_METADATA:&str=r#"
card: EX2-046
name: ADR-02 Searcher
kind: digimon
color: [white]
cost: 3
dp: 1000
form: D-Reaper
traits: [Intel Acquisition Agent]
effects: []
"#;
fn runner(sources:usize)->DebugRunner{
 let mother=mother_yaml();
 let mut r=DebugRunner::builder().from_dsl_yaml(&mother).unwrap().from_dsl_yaml(SEARCHER_METADATA).unwrap()
 .add_card(make_test_card("PROBE-SEARCHER","ADR-02 Searcher"))
 .add_card(make_test_card("PROBE-SOURCE","Unrelated source"))
 .hand(0,&["EX2-046","EX2-046","PROBE-SEARCHER"])
 .deck(0,&["PROBE-SOURCE";10]).deck(1,&["PROBE-SOURCE";10]).memory(10).start();
 r.skip_mulligan();let mut stack=vec!["PROBE-SOURCE";sources];stack.push("EX2-007");r.place_stack(0,&stack);
 r.game.enter_main_phase();r
}
fn h()->PermanentHandle{PermanentHandle{player:0,index:0}}
fn main_id()->u16{FIELD_EFFECT_START+FIELD_EFFECT_SLOT_FOR_MAIN}
fn accept(r:&mut DebugRunner){let s=r.game.pending_selection.as_ref().expect("actual optional reducer");assert!(s.is_optional);let a=*s.valid_action_ids.iter().find(|a|**a!=PASS).unwrap();assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.execute_action(0,a).unwrap();}
#[test]fn mother_has_printed_egg_kind_no_level_and_15000_dp(){let r=runner(0);let c=r.game.players[0].battle_area[0].top_card();assert_eq!(c.card_kind(&r.game.card_data),CardKind::DigiEgg);assert_eq!(c.level(&r.game.card_data),None);assert_eq!(c.dp(&r.game.card_data),Some(15000));}
#[test]fn mother_cannot_attack_even_when_ready(){let mut r=runner(0);r.game.players[0].battle_area[0].turn_played=0;let mask=build_action_mask(&r.game,0);for target in 0..15{assert_eq!(mask[encode_attack(0,target)as usize],0.0);}}
#[test]fn mother_immune_to_all_opponent_effect_kinds_but_not_own(){let r=runner(0);for kind in [EffectSourceKind::Digimon,EffectSourceKind::Tamer,EffectSourceKind::Option]{assert!(r.game.permanent_is_unaffected_by_effect(h(),1,kind));assert!(!r.game.permanent_is_unaffected_by_effect(h(),0,kind));}}
#[test]fn mother_main_can_choose_searcher_from_hand_and_then_is_spent(){let mut r=runner(0);assert_eq!(build_action_mask(&r.game,0)[main_id()as usize],1.0);r.game.decode_action(main_id(),0);let a=2;assert_eq!(build_action_mask(&r.game,0)[a],1.0);r.execute_action(0,a as u16).unwrap();assert_eq!(r.game.players[0].battle_area[0].card_sources[0].card_id(&r.game.card_data),"PROBE-SEARCHER");assert_eq!(build_action_mask(&r.game,0)[main_id()as usize],0.0);}
#[test]fn mother_main_is_unavailable_with_another_mother(){let mut r=runner(0);r.place_on_field(0,"EX2-007",Some(0));r.game.tick_declarative_effects();assert_eq!(build_action_mask(&r.game,0)[main_id()as usize],0.0);}
#[test]fn mother_optional_reducer_counts_own_sources_and_works_once(){let mut r=runner(2);assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);r.game.decode_action(PLAY_HAND_START,0);accept(&mut r);assert_eq!(r.memory(),9);r.game.decode_action(PLAY_HAND_START,0);assert!(r.pending_kind().is_none());assert_eq!(r.memory(),6);}
#[test]fn mother_reducer_decline_preserves_use_for_second_play_and_clone(){let mut r=runner(2);r.game.decode_action(PLAY_HAND_START,0);let saved=r.game.clone();assert_eq!(build_action_mask(&r.game,0)[PASS as usize],1.0);r.execute_action(0,PASS).unwrap();assert_eq!(r.memory(),7);r.game.decode_action(PLAY_HAND_START,0);accept(&mut r);assert_eq!(r.memory(),6);r.game=saved;accept(&mut r);assert_eq!(r.memory(),9);}
#[test]fn mother_in_breeding_does_not_reduce_cost(){let mut r=runner(2);let p=r.game.players[0].battle_area.remove(0);r.game.players[0].breeding_area=Some(p);r.game.tick_declarative_effects();r.game.decode_action(PLAY_HAND_START,0);assert!(r.pending_kind().is_none());assert_eq!(r.memory(),7);}

#[test]fn mother_form_and_type_are_loaded_by_same_dsl_path_as_play_target(){let r=runner(0);let m=r.game.players[0].battle_area[0].top_card();assert!(m.traits(&r.game.card_data).iter().any(|s|s=="D-Reaper"));let c=&r.game.players[0].hand[0];assert!(c.traits(&r.game.card_data).iter().any(|s|s=="D-Reaper"));assert!(c.traits(&r.game.card_data).iter().any(|s|s=="Intel Acquisition Agent"));}
#[test]fn mother_main_printed_mandatory_pick_has_no_decline_after_activation(){let mut r=runner(0);r.game.decode_action(main_id(),0);assert!(r.pending_kind().is_some());assert_eq!(build_action_mask(&r.game,0)[PASS as usize],0.0);}
#[test]fn mother_main_field_searcher_moves_top_only_and_discards_old_sources(){let mut r=runner(0);r.game.players[0].hand.clear();r.place_stack(0,&["PROBE-SOURCE","EX2-046"]);r.game.decode_action(main_id(),0);let a=digimon_engine::action::space::encode_source_select(1,1).unwrap();assert_eq!(build_action_mask(&r.game,0)[a as usize],1.0);r.execute_action(0,a).unwrap();assert_eq!(r.game.players[0].battle_area.len(),1);assert_eq!(r.game.players[0].battle_area[0].card_sources[0].card_id(&r.game.card_data),"EX2-046");assert_eq!(r.game.players[0].trash.len(),1);assert_eq!(r.game.players[0].trash[0].card_id(&r.game.card_data),"PROBE-SOURCE");}


#[test]fn mother_zero_sources_offers_no_reducer_and_preserves_later_use(){
 let mut r=runner(0);r.game.decode_action(PLAY_HAND_START,0);assert!(r.pending_kind().is_none());assert_eq!(r.memory(),7);
 let source=r.game.players[0].hand.remove(1);r.game.players[0].battle_area[0].push_under(source);
 r.game.decode_action(PLAY_HAND_START,0);accept(&mut r);assert_eq!(r.memory(),5);
}
#[test]fn mother_does_not_reduce_paid_effect_play_from_trash(){
 let mut r=runner(2);let c=r.game.players[0].hand.remove(0);r.game.players[0].trash.push(c);
 let played=r.game.play_from_trash_with_cost(0,0,digimon_engine::enums::CostDelta::Reduce(0),digimon_engine::enums::PlaySource::ByEffect);
 assert!(played.is_some());assert!(r.pending_kind().is_none());assert_eq!(r.memory(),7);
 r.game.decode_action(PLAY_HAND_START,0);accept(&mut r);assert_eq!(r.memory(),6);
}
#[test]fn mother_does_not_reduce_own_paid_hand_effect_play_during_opponent_turn(){
 let mut r=runner(2);r.game.turn_player_idx=1;r.game.memory=0;r.game.tick_declarative_effects();
 let played=r.game.play_from_hand_with_cost(0,0,digimon_engine::enums::CostDelta::Reduce(0),digimon_engine::enums::PlaySource::ByEffect);
 assert!(played.is_some());assert!(r.pending_kind().is_none());assert_eq!(r.memory(),3,"opponent turn memory moves toward opponent when player0 pays");
}
