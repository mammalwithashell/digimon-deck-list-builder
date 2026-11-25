//! Gekkomon: Bandai BT27 English card image/list, Q7710 (2026-11-22).
//! Own-turn green BEATBREAK digivolution discount; inherited own-turn +2000 DP.
//! No DCGO snapshot available. Synthetic counterpart metadata is test-only.
use digimon_engine::card_data::EvoCost;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor,GamePhase,PlaySource};
fn runner(color: CardColor, beatbreak: bool) -> DebugRunner {
    let mut target=make_test_card("TARGET","Target");
    target.level=Some(4); target.dp=Some(6000); target.colors=vec![color];
    target.traits=if beatbreak {vec!["BEATBREAK".into()]}else{vec![]};
    target.evo_costs=vec![EvoCost{card_color:CardColor::Green as u8,level:3,memory_cost:3}];
    let mut base=make_test_card("OTHER","Other"); base.level=Some(3);base.colors=vec![CardColor::Green];
    let mut r=DebugRunner::builder().dsl_card("BT27-044").expect("Gekkomon compiles")
        .add_card(target).add_card(base).deck(0,&["OTHER";12]).deck(1,&["OTHER";12])
        .hand(0,&["TARGET"]).memory(10).start();
    r.game.turn_player_idx=0;r.game.memory_pair=(0,1);r.game.current_phase=GamePhase::Main;r.game.turn_count=3;r
}
fn evolve(r:&mut DebugRunner, host:usize) {
    assert!(r.game.digivolve_from_hand(0,0,host,PlaySource::ByHand));
}
#[test]
fn bt27_044_matching_target_costs_two_and_inherited_dp_applies() {
    let mut r=runner(CardColor::Green,true);let h=r.place_on_field(0,"BT27-044",Some(0));
    evolve(&mut r,h.index as usize);assert_eq!(r.game.memory,8);assert_eq!(r.effective_dp(h),Some(8000));
}
#[test]
fn bt27_044_missing_trait_pays_full_cost() {
    let mut r=runner(CardColor::Green,false);let h=r.place_on_field(0,"BT27-044",Some(0));
    evolve(&mut r,h.index as usize);assert_eq!(r.game.memory,7);
}
#[test]
fn bt27_044_wrong_color_pays_full_cost() {
    let mut r=runner(CardColor::Black,true);let h=r.place_on_field(0,"BT27-044",Some(0));
    evolve(&mut r,h.index as usize);assert_eq!(r.game.memory,7);
}
#[test]
fn bt27_044_other_digimon_is_not_discounted() {
    let mut r=runner(CardColor::Green,true);r.place_on_field(0,"BT27-044",Some(0));let h=r.place_on_field(0,"OTHER",Some(0));
    evolve(&mut r,h.index as usize);assert_eq!(r.game.memory,7);assert_eq!(r.effective_dp(h),Some(6000));
}
#[test]
fn bt27_044_breeding_has_no_discount_q7710() {
    let mut r=runner(CardColor::Green,true);r.place_in_breeding(0,"BT27-044");
    assert!(r.game.digivolve_from_hand_onto_breeding(0,0,PlaySource::ByHand));assert_eq!(r.game.memory,7);
}
#[test]
fn bt27_044_inherited_dp_expires_on_opponent_turn() {
    let mut r=runner(CardColor::Green,true);let h=r.place_on_field(0,"BT27-044",Some(0));
    evolve(&mut r,h.index as usize);r.end_turn();assert_eq!(r.effective_dp(h),Some(6000));
}
#[test]
fn bt27_044_top_card_has_no_inherited_dp_bonus() {
    let mut r=runner(CardColor::Green,true);let h=r.place_on_field(0,"BT27-044",Some(0));assert_eq!(r.effective_dp(h),Some(2000));
}
#[test]
fn bt27_044_opponent_turn_has_no_discount() {
    let mut r=runner(CardColor::Green,true);let h=r.place_on_field(0,"BT27-044",Some(0));
    r.game.turn_player_idx=1;
    evolve(&mut r,h.index as usize);assert_eq!(r.game.memory,7);
}
