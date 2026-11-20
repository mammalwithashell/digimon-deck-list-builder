//! BT21-069 GulusGammamon — Digimon, Lv.4, Purple+Red, DP 6000, Cost 6
//! (Dragonkin).
//!
//! Printed (official Bandai DB bundle + DCGO BT21_069.cs):
//! - Evo box: Purple Lv.3 / cost 3 OR Red Lv.3 / cost 3; "[Digivolve]
//!   [Gammamon]: Cost 2" (exact name, any level).
//! - [Security] At the end of the battle, play this card without paying the cost.
//! - [On Play] [When Digivolving] By placing 1 Digimon card with [Gammamon] in its
//!   name from your hand or trash as this Digimon's bottom digivolution card,
//!   delete 1 of your opponent's level 4 or lower Digimon.
//! - Inherited <Retaliation>.
//!
//! Regression (hunt2 cluster alt-digivolve-cost-cards): no YAML spec, so the
//! cost-2 route lived only in `xros_req` and the WD effect did not exist.
//! Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT21-069";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt21_069_purple_or_red_lv3_circle_costs_3_without_prompt() {
    assert_single_route(ID, base("P3", "Purple Rookie", 3, &[PURPLE], &["Dragonkin"]), 3);
    assert_single_route(ID, base("R3", "Red Rookie", 3, &[RED], &["Dragonkin"]), 3);
}

#[test]
fn bt21_069_gammamon_named_base_of_any_level_costs_2() {
    assert_single_route(ID, base("GM2", "Gammamon", 2, &[BLUE], &["Dragonkin"]), 2);
    assert_single_route(ID, base("GM3", "Gammamon", 3, &[YELLOW], &["Dragonkin"]), 2);
}

#[test]
fn bt21_069_purple_or_red_lv3_gammamon_prompts_cost_2_vs_3() {
    assert_cost_choice(ID, base("PGM3", "Gammamon", 3, &[PURPLE], &["Dragonkin"]), 2, 3);
    assert_cost_choice(ID, base("RGM3", "Gammamon", 3, &[RED], &["Dragonkin"]), 2, 3);
}

#[test]
fn bt21_069_other_bases_have_no_route() {
    assert_not_offered(ID, base("SGM", "Super Gammamon", 3, &[BLUE], &["Dragonkin"]));
    assert_not_offered(ID, base("B3", "Blue Rookie", 3, &[BLUE], &["Dragonkin"]));
}

// ─── [On Play][When Digivolving] tuck a [Gammamon] -> delete a Lv.4- Digimon ──

fn digimon(id: &str, name: &str, level: u8) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(3000);
    c
}

/// BT21-069 + `hand_extra` in P0's hand, a plain Purple Lv.3 base on P0's
/// field, `trash_ids` in P0's trash, and a Lv.4 + Lv.5 Digimon on P1's field.
fn runner_with(trash_ids: &[&str], hand_extra: &[&str], opp_ids: &[&str]) -> (DebugRunner, usize) {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    filler.dp = Some(3000);
    let mut hand = vec![ID];
    hand.extend_from_slice(hand_extra);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT21-069 production YAML loads")
        .add_card(base("P3", "Purple Rookie", 3, &[PURPLE], &["Dragonkin"]))
        .add_card(filler)
        .add_card(digimon("GM-H", "Gammamon", 2))
        .add_card(digimon("GM-T", "Super Gammamon", 3))
        .add_card(digimon("OPP4", "Opp Champion", 4))
        .add_card(digimon("OPP5", "Opp Ultimate", 5))
        .hand(0, &hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(15)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    for id in trash_ids {
        r.inject_trash(0, id);
    }
    let slot = r.place_on_field(0, "P3", Some(0)).index as usize;
    for id in opp_ids {
        r.place_on_field(1, id, None);
    }
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0, "digivolve offered");
    r.game.decode_action(a, 0);
}

fn field_ids(r: &DebugRunner, p: usize) -> Vec<String> {
    r.game.players[p]
        .battle_area
        .iter()
        .map(|perm| perm.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

fn pick_first(r: &mut DebugRunner) {
    let view = r.pending_selection_view().expect("a selection is pending");
    let pick = view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap();
    r.execute_action(0, pick).unwrap();
}

#[test]
fn bt21_069_tucks_from_hand_then_deletes_a_level_4_or_lower_digimon() {
    let (mut r, slot) = runner_with(&[], &["GM-H"], &["OPP4", "OPP5"]);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("hand/trash pick parks");
    assert!(view.is_optional, "\"By placing\" cost is declinable");
    pick_first(&mut r);
    // The delete pick offers ONLY the level-4-or-lower Digimon (auto-resolves
    // or parks with one legal target).
    if r.pending_selection().is_some() {
        let view = r.pending_selection_view().unwrap();
        let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
        assert_eq!(picks.len(), 1, "only the Lv.4 is a legal target: {picks:?}");
        pick_first(&mut r);
    }
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()], "the Lv.4 was deleted, Lv.5 stays");
    let perm = &r.game.players[0].battle_area[slot];
    assert_eq!(
        perm.card_sources[0].card_id(&r.game.card_data),
        "GM-H",
        "the hand card is the bottom digivolution card"
    );
}

#[test]
fn bt21_069_can_tuck_from_the_trash_too() {
    let (mut r, slot) = runner_with(&["GM-T"], &[], &["OPP4"]);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_some(), "trash candidate offered");
    pick_first(&mut r);
    if r.pending_selection().is_some() {
        pick_first(&mut r);
    }
    assert!(field_ids(&r, 1).is_empty(), "the Lv.4 was deleted");
    assert_eq!(
        r.game.players[0].battle_area[slot].card_sources[0].card_id(&r.game.card_data),
        "GM-T"
    );
}

#[test]
fn bt21_069_declining_the_tuck_deletes_nothing() {
    let (mut r, slot) = runner_with(&["GM-T"], &["GM-H"], &["OPP4"]);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).expect("decline the placement");
    assert!(r.pending_selection().is_none());
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()], "no placement -> no delete");
}

#[test]
fn bt21_069_tuck_without_a_level_4_or_lower_target_still_places_but_deletes_nothing() {
    let (mut r, slot) = runner_with(&[], &["GM-H"], &["OPP5"]);
    digivolve(&mut r, slot);
    pick_first(&mut r);
    assert!(r.pending_selection().is_none(), "no legal delete target");
    assert_eq!(field_ids(&r, 1), vec!["OPP5".to_string()]);
    assert_eq!(
        r.game.players[0].battle_area[slot].card_sources[0].card_id(&r.game.card_data),
        "GM-H"
    );
}

#[test]
fn bt21_069_no_gammamon_in_hand_or_trash_offers_nothing() {
    let (mut r, slot) = runner_with(&["FILL"], &[], &["OPP4"]);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none(), "no [Gammamon]-name Digimon anywhere");
    assert_eq!(field_ids(&r, 1), vec!["OPP4".to_string()]);
}

// ─── [Security] and inherited <Retaliation> ──────────────────────────────────

#[test]
fn bt21_069_security_plays_card_onto_field_after_the_battle() {
    let mut attacker_card = digimon("ATTACKER", "Attacker", 4);
    attacker_card.colors = vec![CardColor::Red];
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT21-069 production YAML loads")
        .add_card(attacker_card)
        .add_card(make_test_card("PAD", "PAD"))
        .deck(0, &["PAD"; 5])
        .deck(1, &["PAD"; 5])
        .memory(10)
        .start();
    let attacker = r.place_on_field(0, "ATTACKER", Some(0));
    let idx = r.game.card_data.iter().position(|c| c.card_id == ID).unwrap();
    let iid = r.game.next_card_index();
    r.game.players[1].security.push(CardSource::new(idx, 1, iid));
    r.attack_player(attacker, 1, false);
    let _ = r.auto_resolve();
    assert!(
        field_ids(&r, 1).iter().any(|id| id == ID),
        "BT21-069 is played from security at the end of the battle"
    );
}

#[test]
fn bt21_069_inherited_retaliation_on_the_host() {
    let (mut r, _) = runner_with(&[], &[], &[]);
    let host = r.place_stack(0, &[ID, "P3"]);
    assert!(r.game.has_keyword(host, Keyword::Retaliation));
}

#[allow(dead_code)]
fn _kinds() -> SelectionKind {
    SelectionKind::Hand
}
