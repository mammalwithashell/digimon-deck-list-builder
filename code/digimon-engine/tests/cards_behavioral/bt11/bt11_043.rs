//! BT11-043 KingSukamon — Digimon, Lv.5, Yellow/Black, DP 6000, Cost 7 (Mutant).
//!
//! Printed (official Bandai DB bundle + DCGO BT11_043.cs):
//! - Evo box: Lv.4 Yellow / cost 4 OR Lv.4 Black / cost 4; "Digivolve 3 from
//!   Lv.4 w/[Sukamon] in name".
//! - [On Play] [When Digivolving] If your opponent has 16 or more cards in their
//!   trash, or you have 3 or more cards with [Sukamon] in their names in your
//!   trash, change 1 of your opponent's Digimon into a white Digimon with 3000
//!   DP and an original name of [Sukamon] until the end of your opponent's turn.
//! - [When Attacking] For each other Digimon with [Sukamon] in its name in play,
//!   this Digimon gains <Security Attack +1> for the turn.
//! - Inherited: [All Turns] When this Digimon would be deleted, by deleting 1
//!   other Digimon with [Sukamon] in its name, prevent that deletion.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! cost-3 [Sukamon] route lived only in `xros_req` — DCGO's cost-choice prompt
//! never fired and the engine charged a wrong memory cost (the printed 4 vs the
//! alt 3) for a [Sukamon]-name Lv.4 base. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::space::{ATTACK_START, PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT11-043";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn bt11_043_yellow_or_black_lv4_circle_costs_4() {
    assert_single_route(ID, base("Y4", "Yellow Champion", 4, &[YELLOW], &["Beast"]), 4);
    assert_single_route(ID, base("K4", "Black Champion", 4, &[BLACK], &["Beast"]), 4);
}

#[test]
fn bt11_043_sukamon_name_lv4_of_any_colour_costs_3() {
    assert_single_route(ID, base("S4", "Sukamon", 4, &[BLUE], &["Abnormal"]), 3);
    assert_single_route(ID, base("SX", "Gold Sukamon", 4, &[RED], &["Abnormal"]), 3);
}

#[test]
fn bt11_043_yellow_sukamon_lv4_prompts_cost_3_vs_4() {
    assert_cost_choice(ID, base("YS4", "Sukamon", 4, &[YELLOW], &["Abnormal"]), 3, 4);
    assert_cost_choice(ID, base("KS4", "Sukamon", 4, &[BLACK], &["Abnormal"]), 3, 4);
}

#[test]
fn bt11_043_other_bases_have_no_route() {
    assert_not_offered(ID, base("B4", "Blue Champion", 4, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("S3", "Sukamon", 3, &[YELLOW], &["Abnormal"]));
    assert_not_offered(ID, base("S5", "Sukamon", 5, &[BLUE], &["Abnormal"]));
}

// ─── Fixtures for the effects ─────────────────────────────────────────────────

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Yellow];
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 0;
    c
}

fn builder() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT11-043 production YAML loads")
        .add_card(digimon("SUKA", "Sukamon", 4, 2000))
        .add_card(digimon("OPP", "Greymon", 4, 5000))
        .add_card(digimon("OPP2", "Garurumon", 4, 5000))
        .add_card(digimon("CARRIER", "Plain Carrier", 5, 5000))
        .add_card(digimon("SECDIGI", "Security Digimon", 3, 1000))
        .add_card(digimon("F", "Filler", 3, 1000))
        .deck(0, &["F"; 12])
        .deck(1, &["F"; 12])
        .memory(10)
}

fn started(b: DebugRunnerBuilder) -> DebugRunner {
    let mut r = b.start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r
}

fn identity(r: &DebugRunner, h: PermanentHandle) -> digimon_engine::permanent::SynthIdentity {
    r.game.players[h.player as usize].battle_area[h.index as usize].synth_identity(
        &r.game.card_data,
        &r.game.modifiers,
        h,
    )
}

// ─── [On Play][When Digivolving] — gated by trash counts ──────────────────────

fn on_play_runner(opp_trash: usize, own_sukamon_trash: usize) -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = started(builder());
    let me = r.place_on_field(0, ID, Some(0));
    let opp = r.place_on_field(1, "OPP", Some(0));
    for _ in 0..opp_trash {
        r.inject_trash(1, "F");
    }
    for _ in 0..own_sukamon_trash {
        r.inject_trash(0, "SUKA");
    }
    r.fire_on_play(0, me.index as usize);
    (r, me, opp)
}

#[test]
fn bt11_043_sixteen_opponent_trash_cards_turn_an_opponent_digimon_into_a_white_3000_sukamon() {
    let (mut r, _me, opp) = on_play_runner(16, 0);
    let view = r.pending_selection_view().expect("mandatory target pick parks");
    assert!(!view.is_optional, "the change is mandatory (DCGO canNoSelect: false)");
    r.execute_action(0, ATTACK_START + opp.index as u16).expect("pick the opponent Digimon");
    let id = identity(&r, opp);
    assert_eq!(id.card_name, "Sukamon", "original name becomes [Sukamon]");
    assert_eq!(id.colors, vec![CardColor::White], "original colour becomes white");
    assert_eq!(r.effective_dp(opp), Some(3000), "original DP becomes 3000");
}

#[test]
fn bt11_043_three_sukamon_name_cards_in_your_trash_also_enable_it() {
    let (r, _me, _opp) = on_play_runner(0, 3);
    assert!(r.pending_selection().is_some(), "3 [Sukamon]-name cards in your trash");
}

#[test]
fn bt11_043_fifteen_opponent_trash_or_two_sukamon_do_not_enable_it() {
    let (r, _me, _opp) = on_play_runner(15, 2);
    assert!(r.pending_selection().is_none(), "neither threshold is met");
}

// ─── [When Attacking]: Security A. + for each other Sukamon-name Digimon ──────

fn attack_runner(allies: usize, rivals: usize) -> (DebugRunner, PermanentHandle) {
    let mut b = builder();
    b = b.security(1, &["SECDIGI"; 8]);
    let mut r = started(b);
    let me = r.place_on_field(0, ID, Some(0));
    for _ in 0..allies {
        r.place_on_field(0, "SUKA", Some(0));
    }
    for _ in 0..rivals {
        r.place_on_field(1, "SUKA", Some(0));
    }
    // Opposing non-Sukamon Digimon never counts; keep one so the board is mixed.
    r.place_on_field(1, "OPP2", Some(0));
    (r, me)
}

fn checks_made(allies: usize, rivals: usize) -> usize {
    let (mut r, me) = attack_runner(allies, rivals);
    let before = r.security_count(1);
    r.attack_player(me, 1, false);
    let _ = r.auto_resolve();
    before - r.security_count(1)
}

#[test]
fn bt11_043_security_attack_counts_other_sukamon_name_digimon_of_both_players() {
    assert_eq!(checks_made(0, 0), 1, "no other Sukamon: the clause does not apply");
    assert_eq!(checks_made(1, 0), 2, "one own Sukamon: Security A. +1");
    assert_eq!(checks_made(1, 2), 4, "one own + two opposing Sukamon: Security A. +3");
}

// ─── Inherited: by deleting 1 other Sukamon-name Digimon, prevent the deletion ─

fn carrier_runner() -> (DebugRunner, PermanentHandle) {
    let mut r = started(builder());
    let carrier = r.place_stack(0, &[ID, "CARRIER"]);
    (r, carrier)
}

fn field_ids(r: &DebugRunner, player: u8) -> Vec<String> {
    r.game.players[player as usize]
        .battle_area
        .iter()
        .map(|p| p.top_card().card_id(&r.game.card_data).to_string())
        .collect()
}

#[test]
fn bt11_043_inherited_deletes_another_sukamon_and_the_carrier_stays() {
    let (mut r, carrier) = carrier_runner();
    r.place_on_field(0, "SUKA", Some(0));
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    assert_eq!(accept.kind, SelectionKind::Replacement);
    assert!(accept.is_optional);
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    let picks: Vec<u16> = cost.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 1);
    r.execute_action(0, picks[0]).expect("delete the other Sukamon");
    let _ = r.auto_resolve();
    let field = field_ids(&r, 0);
    assert!(field.contains(&"CARRIER".to_string()), "the deletion was prevented");
    assert!(!field.contains(&"SUKA".to_string()), "the cost Digimon was deleted");
}

#[test]
fn bt11_043_inherited_may_delete_an_opponents_sukamon_too() {
    // Official Q&A Q2078: "Can this card's inherited effect delete an
    // opponent's [Sukamon] to prevent itself from being deleted?" — Yes.
    let (mut r, carrier) = carrier_runner();
    let rival = r.place_on_field(1, "SUKA", Some(0));
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    r.execute_action(0, accept.valid_action_ids[0]).expect("accept");
    let cost = r.pending_selection_view().expect("cost prompt");
    let picks: Vec<u16> = cost.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 1, "the opposing [Sukamon] is selectable");
    r.execute_action(0, picks[0]).unwrap();
    let _ = r.auto_resolve();
    assert!(field_ids(&r, 0).contains(&"CARRIER".to_string()));
    assert!(!field_ids(&r, 1).contains(&"SUKA".to_string()));
    let _ = rival;
}

#[test]
fn bt11_043_inherited_is_not_offered_without_another_sukamon_and_can_be_declined() {
    let (mut r, carrier) = carrier_runner();
    r.place_on_field(0, "OPP", Some(0));
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    assert!(r.pending_selection_view().is_none(), "no other [Sukamon]-name Digimon");
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()));

    let (mut r, carrier) = carrier_runner();
    r.place_on_field(0, "SUKA", Some(0));
    r.game.delete_permanent_with_cause(carrier, ReplacementCause::OwnEffect);
    let accept = r.pending_selection_view().expect("accept prompt");
    r.execute_action(0, PASS).expect("decline the prevention");
    let _ = (accept, EffectTiming::OnPlay);
    let _ = r.auto_resolve();
    assert!(!field_ids(&r, 0).contains(&"CARRIER".to_string()), "declined: the carrier is deleted");
    assert!(field_ids(&r, 0).contains(&"SUKA".to_string()));
}
