//! BT22-012 RizeGreymon — Digimon, Lv.5, Red/Black, DP 8000, Cost 8 (Cyborg,
//! CS).
//!
//! Printed (official Bandai DB bundle + DCGO BT22_012.cs):
//! - Evo box: Lv.4 Red / cost 4 OR Lv.4 Black / cost 4; "Digivolve Lv.4
//!   w/[Greymon] in name or w/[CS] trait: Cost 3".
//! - <Raid>; [When Digivolving] If you have 1 or fewer Tamers, from your hand
//!   and without paying the cost, you may play 1 red or black Tamer card with a
//!   play cost of 4 or less or 1 Tamer card with the [CS] trait.
//! - Inherited: <Security A. +1>.
//!
//! The card had only a JSON stub (alt route only in `xros_req`): a Greymon-name
//! or [CS] Lv.4 base of another colour had no legal digivolve, and a Red/Black
//! one never got the cost-choice prompt. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::{build_action_mask, space::{encode_digivolve, PASS}};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, GamePhase, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "BT22-012";

#[test]
fn bt22_012_red_or_black_lv4_circle_costs_4() {
    assert_single_route(ID, base("R4", "Red Champion", 4, &[RED], &["Beast"]), 4);
    assert_single_route(ID, base("K4", "Black Champion", 4, &[BLACK], &["Beast"]), 4);
}

#[test]
fn bt22_012_greymon_name_or_cs_lv4_of_any_colour_costs_3() {
    assert_single_route(ID, base("G4", "SkullGreymon", 4, &[BLUE], &["Undead"]), 3);
    assert_single_route(ID, base("C4", "Blue CS", 4, &[BLUE], &["CS"]), 3);
}

#[test]
fn bt22_012_red_greymon_or_cs_lv4_prompts_cost_3_vs_4() {
    assert_cost_choice(ID, base("RG4", "Greymon", 4, &[RED], &["Dinosaur"]), 3, 4);
    assert_cost_choice(ID, base("KC4", "Black CS", 4, &[BLACK], &["CS"]), 3, 4);
}

#[test]
fn bt22_012_other_bases_have_no_route() {
    assert_not_offered(ID, base("B4", "Blue Champion", 4, &[BLUE], &["Beast"]));
    assert_not_offered(ID, base("G3", "Greymon Rookie", 3, &[RED], &["Dinosaur"]));
    assert_not_offered(ID, base("G5", "Greymon Ultimate", 5, &[RED], &["Dinosaur"]));
}

fn tamer(id: &str, color: CardColor, cost: u16, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Tamer;
    c.level = None;
    c.dp = None;
    c.play_cost = cost;
    c.colors = vec![color];
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn wd_runner(hand_extra: &[&str], own_tamers: usize) -> (DebugRunner, usize) {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    let mut hand = vec![ID];
    hand.extend_from_slice(hand_extra);
    let mut r = DebugRunner::builder()
        .dsl_card(ID)
        .expect("BT22-012 production YAML loads")
        .add_card(base("C4", "Blue CS", 4, &[BLUE], &["CS"]))
        .add_card(filler)
        .add_card(tamer("RED-LOW", CardColor::Red, 4, &["Hero"]))
        .add_card(tamer("BLACK-LOW", CardColor::Black, 3, &["Hero"]))
        .add_card(tamer("RED-HIGH", CardColor::Red, 5, &["Hero"]))
        .add_card(tamer("BLUE-LOW", CardColor::Blue, 2, &["Hero"]))
        .add_card(tamer("CS-HIGH", CardColor::Blue, 6, &["CS"]))
        .add_card(tamer("FIELD", CardColor::Blue, 1, &["Hero"]))
        .hand(0, &hand)
        .deck(0, &["FILL"; 12])
        .deck(1, &["FILL"; 12])
        .memory(10)
        .start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    r.game.current_phase = GamePhase::Main;
    let slot = r.place_on_field(0, "C4", Some(0)).index as usize;
    for _ in 0..own_tamers {
        r.place_on_field(0, "FIELD", Some(0));
    }
    (r, slot)
}

fn digivolve(r: &mut DebugRunner, slot: usize) {
    let a = encode_digivolve(0, slot as u16);
    assert!(build_action_mask(&r.game, 0)[a as usize] > 0.0);
    r.game.decode_action(a, 0);
}

#[test]
fn bt22_012_offers_cheap_red_black_tamers_and_cs_tamers_only() {
    let (mut r, slot) = wd_runner(&["RED-LOW", "BLACK-LOW", "RED-HIGH", "BLUE-LOW", "CS-HIGH"], 0);
    digivolve(&mut r, slot);
    let view = r.pending_selection_view().expect("the optional Tamer pick parks");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "\"you may\"");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    // RED-LOW (red, 4), BLACK-LOW (black, 3), CS-HIGH ([CS], any cost).
    // RED-HIGH (cost 5) and BLUE-LOW (blue, no CS) are out.
    assert_eq!(picks.len(), 3, "{picks:?}");
    let before = r.memory();
    r.execute_action(0, picks[0]).unwrap();
    assert_eq!(r.memory(), before, "played without paying the cost");
}

#[test]
fn bt22_012_one_tamer_still_allows_it_two_do_not() {
    let (mut r, slot) = wd_runner(&["RED-LOW"], 1);
    digivolve(&mut r, slot);
    assert_eq!(r.pending_kind(), Some(SelectionKind::Hand));
    let (mut r, slot) = wd_runner(&["RED-LOW"], 2);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt22_012_nothing_legal_or_declining_plays_nothing() {
    let (mut r, slot) = wd_runner(&["RED-HIGH", "BLUE-LOW"], 0);
    digivolve(&mut r, slot);
    assert!(r.pending_selection().is_none());
    let (mut r, slot) = wd_runner(&["RED-LOW"], 0);
    digivolve(&mut r, slot);
    r.execute_action(0, PASS).expect("decline");
    assert!(r.pending_selection().is_none());
}

#[test]
fn bt22_012_has_raid_and_declares_the_inherited_security_attack_plus_1() {
    // <Raid> is a face-up keyword of the top card.
    let (mut r, _slot) = wd_runner(&[], 0);
    let face = r.place_on_field(0, ID, Some(1));
    assert!(r.game.has_keyword(face, Keyword::Raid));
    // The inherited <Security A. +1> is declared as an inherited keyword
    // clause (BT20-025 idiom); the engine evaluates the Security Attack total
    // from the card's printed inherited text (cards.json, absent from this
    // synthetic YAML-only fixture), so the clause shape is what we pin here.
    let compiled = r.compiled_card(ID).expect("compiled");
    let has_inherited_sa = compiled.effects.iter().any(|c| {
        matches!(
            c,
            digimon_dsl::compiled::CompiledClause::Declarative(
                digimon_dsl::compiled::CompiledDeclarativeClause::GrantKeyword {
                    scope: digimon_dsl::compiled::CompiledScope::Inherited,
                    ..
                }
            )
        )
    });
    assert!(has_inherited_sa, "inherited <Security A. +1> keyword clause");
}
