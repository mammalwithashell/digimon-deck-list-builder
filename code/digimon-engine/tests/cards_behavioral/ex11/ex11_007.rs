//! EX11-007 Agumon — Digimon, Lv.3, Red/Green, DP 1000, Cost 3 (Reptile,
//! LIBERATOR).
//!
//! Printed (official Bandai DB bundle + DCGO EX11_007.cs):
//! - Evo box: Lv.2 Red / cost 1 OR Lv.2 Green / cost 1; "Digivolve [Koromon]:
//!   Cost 0".
//! - [When Moving] [On Play] 1 of your Digimon with [Tyrannomon] in its name or
//!   the [Reptile] or [Dinosaur] trait gains <Raid> and <Piercing> for the turn.
//! - Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): the card had no YAML
//! spec, so the [Koromon] cost-0 route lived only in `xros_req` — a Red/Green
//! Koromon never got the "Which digivolution cost do you pay?" prompt and a
//! non-Red/Green Koromon had no legal digivolve. Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

const ID: &str = "EX11-007";

fn koromon(colors: &[digimon_engine::enums::CardColor]) -> digimon_engine::card_data::CardData {
    base("KOROMON", "Koromon", 2, colors, &["Lesser"])
}

// ─── Alt route: [Koromon] cost 0 vs the Red/Green Lv.2 circles (cost 1) ───────

#[test]
fn ex11_007_blue_koromon_reaches_only_the_special_route_for_0() {
    assert_single_route(ID, koromon(&[BLUE]), 0);
}

#[test]
fn ex11_007_red_koromon_prompts_cost_0_vs_1() {
    assert_cost_choice(ID, koromon(&[RED]), 0, 1);
}

#[test]
fn ex11_007_green_koromon_prompts_cost_0_vs_1() {
    assert_cost_choice(ID, koromon(&[GREEN]), 0, 1);
}

#[test]
fn ex11_007_red_and_green_lv2_circles_cost_1_without_prompt() {
    assert_single_route(ID, base("R2", "Red Rookie", 2, &[RED], &["Lesser"]), 1);
    assert_single_route(ID, base("G2", "Green Rookie", 2, &[GREEN], &["Lesser"]), 1);
}

#[test]
fn ex11_007_other_bases_have_no_route() {
    // Blue Lv.2 that is not [Koromon]: no circle, no special route.
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    // The printed gate is the exact name: a card merely containing it is out.
    assert_not_offered(ID, base("KX", "Koromon X", 2, &[BLUE], &["Lesser"]));
    // Right colours, wrong level.
    assert_not_offered(ID, base("R3", "Red Champion", 3, &[RED], &["Lesser"]));
}

// ─── [On Play] / [When Moving]: 1 Digimon gains <Raid> and <Piercing> ─────────

fn field_card(id: &str, name: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.level = Some(4);
    c.dp = Some(4000);
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn board_runner() -> DebugRunner {
    let mut filler = make_test_card("FILL", "Filler");
    filler.level = Some(3);
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX11-007 production YAML loads")
        .add_card(field_card("REPTILE", "Reptile Buddy", &["Reptile"]))
        .add_card(field_card("DINO", "Dino Buddy", &["Dinosaur"]))
        .add_card(field_card("NAMED", "MetalTyrannomon", &["Cyborg"]))
        .add_card(field_card("OTHER", "Other Buddy", &["Beast"]))
        .add_card(filler)
        .deck(0, &["FILL"; 10])
        .deck(1, &["FILL"; 10])
        .memory(10)
        .start()
}

#[test]
fn ex11_007_on_play_offers_only_matching_digimon_and_grants_raid_and_piercing() {
    let mut r = board_runner();
    let reptile = r.place_on_field(0, "REPTILE", Some(0));
    let other = r.place_on_field(0, "OTHER", Some(1));
    let dino = r.place_on_field(0, "DINO", Some(2));
    r.add_to_hand(0, ID);
    r.play(0, 0).expect("Agumon plays from hand");

    let view = r.pending_selection_view().expect("a target prompt must park");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(!view.is_optional, "the grant is mandatory (DCGO canNoSelect: false)");
    // REPTILE (trait), DINO (trait) and the played Agumon itself (Reptile); the
    // non-matching OTHER is out.
    assert_eq!(
        view.valid_action_ids.len(),
        3,
        "Reptile/Dinosaur/Tyrannomon-name Digimon only; got {:?}",
        view.valid_action_ids
    );
    // Ids ascend with the field slot: [REPTILE, DINO, Agumon].
    let mut ids = view.valid_action_ids.clone();
    ids.sort();
    r.execute_action(0, ids[1]).expect("pick the Dinosaur");

    assert!(r.game.has_keyword(dino, Keyword::Raid));
    assert!(r.game.has_keyword(dino, Keyword::Piercing));
    assert!(!r.game.has_keyword(other, Keyword::Raid));
    assert!(!r.game.has_keyword(other, Keyword::Piercing));
    assert!(!r.game.has_keyword(reptile, Keyword::Raid));

    // "for the turn": gone after the turn ends.
    r.end_turn();
    assert!(!r.game.has_keyword(dino, Keyword::Raid));
    assert!(!r.game.has_keyword(dino, Keyword::Piercing));
}

#[test]
fn ex11_007_tyrannomon_in_name_is_offered_without_the_trait() {
    let mut r = board_runner();
    let named = r.place_on_field(0, "NAMED", Some(0));
    r.add_to_hand(0, ID);
    r.play(0, 0).expect("Agumon plays from hand");
    let view = r.pending_selection_view().expect("target prompt");
    // MetalTyrannomon (name) + the played Agumon (Reptile).
    assert_eq!(view.valid_action_ids.len(), 2, "{:?}", view.valid_action_ids);
    let mut ids = view.valid_action_ids.clone();
    ids.sort();
    r.execute_action(0, ids[0]).unwrap();
    assert!(r.game.has_keyword(named, Keyword::Raid));
    assert!(r.game.has_keyword(named, Keyword::Piercing));
}

#[test]
fn ex11_007_when_moving_fires_for_this_digimon() {
    let mut r = board_runner();
    r.place_in_breeding(0, ID);
    assert!(r.move_from_breeding(0), "Agumon moves from breeding to battle");
    assert_eq!(
        r.pending_kind(),
        Some(SelectionKind::OwnField),
        "[When Moving] must install the Raid/Piercing target prompt"
    );
}

// ─── Inherited +1000 DP ───────────────────────────────────────────────────────

#[test]
fn ex11_007_inherited_plus_1000_dp_on_the_host() {
    let mut r = board_runner();
    let host = r.place_stack(0, &[ID, "OTHER"]);
    let dp = r.effective_dp(host).expect("host has DP");
    assert_eq!(dp, 4000 + 1000, "inherited [All Turns] +1000 DP");
}
