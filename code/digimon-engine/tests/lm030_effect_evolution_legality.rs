//! Official LM-030 Q4048/Q4049: normal evolution requirements still apply.
//! Real printed cards; actual paid Option action; no mocked evolution metadata.
use digimon_engine::action::{mask::build_action_mask, space::{ATTACK_START, PLAY_HAND_START, PASS}};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::selection::SelectionKind;
fn fixture() -> DebugRunner {
 let mut r=DebugRunner::builder().dsl_card("LM-030").unwrap()
  .dsl_card("BT20-042").unwrap().dsl_card("ST4-02").unwrap()
  .dsl_card("ST4-13").unwrap().dsl_card("EX3-074").unwrap()
  .dsl_card("ST1-10").unwrap().dsl_card("ST1-03").unwrap()
  .hand(0,&["LM-030","ST4-13","EX3-074","ST1-10"])
  .deck(0,&["ST1-03";8]).deck(1,&["ST1-03";8]).memory(10).start();
 r.skip_mulligan(); r.place_on_field(0,"BT20-042",Some(0));
 r.place_on_field(0,"ST4-02",Some(0));r.game.enter_main_phase();
 assert_eq!(build_action_mask(&r.game,0)[PLAY_HAND_START as usize],1.0);
 r.game.decode_action(PLAY_HAND_START,0);assert_eq!(r.memory(),8);r
}
fn legal(r:&DebugRunner)->Vec<u16>{build_action_mask(&r.game,0).iter().enumerate().filter_map(|(i,&x)|if x>0.0{Some(i as u16)}else{None}).collect()}
fn hand(r:&mut DebugRunner){assert_eq!(r.pending_kind(),Some(SelectionKind::OwnField));r.execute_action(0,ATTACK_START).unwrap();assert_eq!(r.pending_kind(),Some(SelectionKind::Hand));}
fn tail(r:&DebugRunner){assert_eq!(r.pending_kind(),None);assert!(r.game.players[0].battle_area.iter().any(|p|p.top_card().card_id(&r.game.card_data)=="LM-030"));}
#[test]fn green_scramble_host_mask_excludes_host_without_legal_evolution(){let r=fixture();assert_eq!(r.pending_kind(),Some(SelectionKind::OwnField));let mut expected=vec![PASS,ATTACK_START];expected.sort();assert_eq!(legal(&r),expected);}
#[test]fn green_scramble_hand_mask_excludes_invalid_level_and_color(){let mut r=fixture();hand(&mut r);assert_eq!(legal(&r),vec![PLAY_HAND_START,PASS]);}
#[test]fn green_scramble_accepts_real_level6_and_pays_reduced_cost(){let mut r=fixture();hand(&mut r);r.execute_action(0,PLAY_HAND_START).unwrap();tail(&r);assert_eq!(r.memory(),7);assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"ST4-13");assert_eq!(r.game.players[0].battle_area[0].card_sources.len(),2);assert_eq!(r.hand_size(0),3,"two retained cards plus evolution draw");}
#[test]fn green_scramble_declining_host_skips_dependent_hand_choice(){let mut r=fixture();r.execute_action(0,PASS).unwrap();tail(&r);assert_eq!(r.memory(),8);assert_eq!(r.hand_size(0),3);assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT20-042");}
#[test]fn green_scramble_hand_decline_retains_cards_and_places_delay(){let mut r=fixture();hand(&mut r);assert!(legal(&r).contains(&PASS));r.execute_action(0,PASS).unwrap();tail(&r);assert_eq!(r.memory(),8);assert_eq!(r.hand_size(0),3);assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT20-042");}
#[test]fn green_scramble_cloned_pending_hand_supports_independent_accept_and_decline(){let mut r=fixture();hand(&mut r);let saved=r.game.clone();assert_eq!(legal(&r),vec![PLAY_HAND_START,PASS]);r.execute_action(0,PLAY_HAND_START).unwrap();tail(&r);assert_eq!(r.memory(),7);r.game=saved;assert_eq!(legal(&r),vec![PLAY_HAND_START,PASS]);r.execute_action(0,PASS).unwrap();tail(&r);assert_eq!(r.memory(),8);assert_eq!(r.game.players[0].battle_area[0].top_card().card_id(&r.game.card_data),"BT20-042");}
