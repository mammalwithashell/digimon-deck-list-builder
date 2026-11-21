//! BT20-031 Liamon — Digimon, Lv.4, Yellow/Black, DP 5000, Cost 4 (Holy Beast,
//! ACCEL).
//!
//! Printed (official Bandai DB bundle + DCGO BT20_031.cs):
//! - Evo box: Lv.3 Yellow / cost 3 OR Lv.3 Black / cost 3; "Digivolve Lv.3
//!   w/[ACCEL] trait: Cost 2".
//! - [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP
//!   for the turn.
//! - Inherited: <Barrier>.
//!
//! The card had only a JSON stub (alt route only in `xros_req`): a Yellow/Black
//! Lv.3 [ACCEL] base never got the cost-choice prompt and an off-colour one
//! could not digivolve. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, ATTACK_START}};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-031";

#[test]
fn bt20_031_yellow_or_black_lv3_circle_costs_3() {
    assert_single_route(ID, base("Y3", "Yellow Rookie", 3, &[YELLOW], &["Beast"]), 3);
    assert_single_route(ID, base("K3", "Black Rookie", 3, &[BLACK], &["Beast"]), 3);
}

#[test]
fn bt20_031_accel_lv3_of_any_colour_costs_2() {
    assert_single_route(ID, base("A3", "Blue Accel", 3, &[BLUE], &["ACCEL"]), 2);
}

#[test]
fn bt20_031_yellow_or_black_accel_lv3_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("YA3", "Yellow Accel", 3, &[YELLOW], &["ACCEL"]), 2, 3);
    assert_cost_choice(ID, base("KA3", "Black Accel", 3, &[BLACK], &["ACCEL"]), 2, 3);
}

#[test]
fn bt20_031_other_bases_have_no_route() {
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("A4", "Accel Champion", 4, &[YELLOW], &["ACCEL"]));
    assert_not_offered(ID, base("A2", "Accel In-Training", 2, &[YELLOW], &["ACCEL"]));
}

fn wd_runner() -> (DebugRunner, usize, digimon_engine::permanent::PermanentHandle) {
    let mut opp = base("OPP", "Opposing Digimon", 4, &[BLUE], &["Beast"]);
    opp.dp = Some(5000);
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-031 production YAML loads")
        .add_card(base("A3", "Blue Accel", 3, &[BLUE], &["ACCEL"]))
        .add_card(opp)
        .add_card(filler)
        .hand(0, &[ID])
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "A3", Some(0)).index as usize;
    let opp_h = r.place_on_field(1, "OPP", Some(0));
    (r, slot, opp_h)
}

#[test]
fn bt20_031_when_digivolving_gives_an_opponent_digimon_minus_3000_dp_for_the_turn() {
    let (mut r, slot, opp) = wd_runner();
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
    let view = r.pending_selection_view().expect("mandatory target pick parks");
    assert!(!view.is_optional, "DCGO canNoSelect: false");
    assert_eq!(view.kind, SelectionKind::OppField);
    r.execute_action(0, ATTACK_START + opp.index as u16).expect("pick the opposing Digimon");
    assert_eq!(r.effective_dp(opp), Some(5000 - 3000));
    r.end_turn();
    assert_eq!(r.effective_dp(opp), Some(5000), "for the turn only");
}

#[test]
fn bt20_031_inherited_barrier_reaches_the_host() {
    let (mut r, _slot, _opp) = wd_runner();
    let host = r.place_stack(0, &[ID, "A3"]);
    assert!(r.game.has_keyword(host, Keyword::Barrier));
    let top_only = r.place_on_field(0, ID, Some(1));
    assert!(!r.game.has_keyword(top_only, Keyword::Barrier), "inherited text only from under another card");
}
