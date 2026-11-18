//! BT20-039 Diatrymon — Digimon, Lv.4, Green/Yellow, DP 5000, Cost 4 (Ancient
//! Bird, ACCEL).
//!
//! Printed (official Bandai DB bundle + DCGO BT20_039.cs):
//! - Evo box: Lv.3 Green / cost 3 OR Lv.3 Yellow / cost 3; "Digivolve Lv.3
//!   w/[ACCEL] trait: Cost 2".
//! - [On Play] [When Digivolving] Suspend 1 of your opponent's Digimon.
//! - Inherited: <Piercing>.
//!
//! The card had only a JSON stub (alt route only in `xros_req`). Synthetic
//! boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, ATTACK_START}};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT20-039";

#[test]
fn bt20_039_green_or_yellow_lv3_circle_costs_3() {
    assert_single_route(ID, base("G3", "Green Rookie", 3, &[GREEN], &["Beast"]), 3);
    assert_single_route(ID, base("Y3", "Yellow Rookie", 3, &[YELLOW], &["Beast"]), 3);
}

#[test]
fn bt20_039_accel_lv3_of_any_colour_costs_2() {
    assert_single_route(ID, base("A3", "Red Accel", 3, &[RED], &["ACCEL"]), 2);
}

#[test]
fn bt20_039_green_or_yellow_accel_lv3_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("GA3", "Green Accel", 3, &[GREEN], &["ACCEL"]), 2, 3);
    assert_cost_choice(ID, base("YA3", "Yellow Accel", 3, &[YELLOW], &["ACCEL"]), 2, 3);
}

#[test]
fn bt20_039_other_bases_have_no_route() {
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("A4", "Accel Champion", 4, &[GREEN], &["ACCEL"]));
}

fn wd_runner() -> (DebugRunner, usize, digimon_engine::permanent::PermanentHandle) {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT20-039 production YAML loads")
        .add_card(base("A3", "Red Accel", 3, &[RED], &["ACCEL"]))
        .add_card(base("OPP", "Opposing Digimon", 4, &[BLUE], &["Beast"]))
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
    let opp = r.place_on_field(1, "OPP", Some(0));
    (r, slot, opp)
}

#[test]
fn bt20_039_when_digivolving_suspends_an_opponent_digimon() {
    let (mut r, slot, opp) = wd_runner();
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
    let view = r.pending_selection_view().expect("mandatory target pick parks");
    assert!(!view.is_optional, "DCGO canNoSelect: false");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(!r.game.players[1].battle_area[opp.index as usize].is_suspended);
    r.execute_action(0, ATTACK_START + opp.index as u16).expect("pick the opposing Digimon");
    assert!(r.game.players[1].battle_area[opp.index as usize].is_suspended);
}

#[test]
fn bt20_039_inherited_piercing_reaches_the_host() {
    let (mut r, _slot, _opp) = wd_runner();
    let host = r.place_stack(0, &[ID, "A3"]);
    assert!(r.game.has_keyword(host, Keyword::Piercing));
    let top_only = r.place_on_field(0, ID, Some(1));
    assert!(!r.game.has_keyword(top_only, Keyword::Piercing), "inherited text only from under another card");
}
