//! BT22-046 Gargomon — Digimon, Lv.4, Green/Yellow, DP 6000, Cost 5 (Beastkin,
//! CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_046.cs):
//! - Evo box: Lv.3 Green / cost 3 OR Lv.3 Yellow / cost 3; "Digivolve Lv.3
//!   w/[CS] trait: Cost 2".
//! - [When Digivolving] If you have 1 or fewer Tamers, you may play 1 Tamer
//!   card with the [CS] trait from your hand without paying the cost.
//! - Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! cost-2 route lived only in `xros_req` — a Green/Yellow Lv.3 [CS] base got no
//! cost-choice prompt (it silently paid 3) and an off-colour [CS] base could not
//! digivolve. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT22-046";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt22_046_green_or_yellow_lv3_circle_costs_3_without_prompt() {
    // The When-Digivolving clause has no Tamer to play on this board, so no
    // selection parks and the memory delta is exactly the printed cost.
    assert_single_route(ID, base("G3", "Green Rookie", 3, &[GREEN], &["Beast"]), 3);
    assert_single_route(ID, base("Y3", "Yellow Rookie", 3, &[YELLOW], &["Beast"]), 3);
}

#[test]
fn bt22_046_cs_lv3_of_any_colour_costs_2() {
    assert_single_route(ID, base("CS3", "Blue CS", 3, &[BLUE], &["CS"]), 2);
}

#[test]
fn bt22_046_green_cs_lv3_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("GCS3", "Green CS", 3, &[GREEN], &["CS"]), 2, 3);
    assert_cost_choice(ID, base("YCS3", "Yellow CS", 3, &[YELLOW], &["CS"]), 2, 3);
}

#[test]
fn bt22_046_other_bases_have_no_route() {
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("CS2", "CS In-Training", 2, &[GREEN], &["CS"]));
}

// ─── [When Digivolving]: <= 1 Tamer → may play a [CS] Tamer free ──────────────

fn tamer(id: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = 3;
    c.colors = vec![CardColor::Green];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn wd_runner(hand_extra: &[&str], own_tamers: usize) -> (DebugRunner, usize) {
    let mut cs3 = base("CS3", "Blue CS", 3, &[BLUE], &["CS"]);
    cs3.dp = Some(3000);
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut hand = vec![ID];
    hand.extend_from_slice(hand_extra);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-046 production YAML loads")
        .add_card(cs3)
        .add_card(filler)
        .add_card(tamer("CS-TAMER", &["CS"]))
        .add_card(tamer("CS-TAMER-B", &["CS"]))
        .add_card(tamer("PLAIN-TAMER", &["Hero"]))
        .add_card(tamer("FIELD-TAMER", &["Hero"]))
        .hand(0, &hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "CS3", Some(0)).index as usize;
    for _ in 0..own_tamers {
        r.place_on_field(0, "FIELD-TAMER", Some(0));
    }
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
}

#[test]
fn bt22_046_offers_only_cs_tamers_and_plays_the_pick_free() {
    let (mut r, slot) = wd_runner(&["CS-TAMER", "PLAIN-TAMER", "CS-TAMER-B"], 0);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("the optional Tamer pick parks");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "\"you may\"");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "only the two [CS] Tamers: {picks:?}");
    let before = r.memory();
    r.execute_action(0, picks[0]).unwrap();
    assert_eq!(r.memory(), before, "played without paying the cost");
    assert_eq!(
        r.game.players[0]
            .battle_area
            .iter()
            .filter(|p| p.top_card().card_id(&r.game.card_data).starts_with("CS-TAMER"))
            .count(),
        1
    );
}

#[test]
fn bt22_046_one_tamer_still_allows_it_two_do_not() {
    let (mut r, slot) = wd_runner(&["CS-TAMER"], 1);
    digivolve(&mut r, slot);
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::Hand),
        "1 Tamer: \"1 or fewer\""
    );
    let (mut r, slot) = wd_runner(&["CS-TAMER"], 2);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none(), "2 Tamers: the clause does not apply");
}

#[test]
fn bt22_046_no_cs_tamer_in_hand_or_declining_plays_nothing() {
    let (mut r, slot) = wd_runner(&["PLAIN-TAMER"], 0);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none(), "no [CS] Tamer: nothing to offer");

    let (mut r, slot) = wd_runner(&["CS-TAMER"], 0);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).expect("decline the optional play");
    assert!(r.pending_selection().is_none());
    assert!(r.game.players[0]
        .battle_area
        .iter()
        .all(|p| !p.top_card().card_id(&r.game.card_data).starts_with("CS-TAMER")));
}

// ─── Inherited +1000 DP ───────────────────────────────────────────────────────

#[test]
fn bt22_046_inherited_plus_1000_dp_on_the_host() {
    let (mut r, _) = wd_runner(&[], 0);
    let host = r.place_stack(0, &[ID, "CS3"]);
    assert_eq!(r.effective_dp(host), Some(3000 + 1000));
}
