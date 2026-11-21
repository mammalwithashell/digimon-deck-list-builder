//! EX11-025 FunBeemon — Digimon, Lv.3, Green/Black, DP 1000, Cost 3 (Insectoid,
//! X Antibody, Royal Base, LIBERATOR).
//!
//! Printed (official Bandai DB bundle + DCGO EX11_025.cs):
//! - Evo box: Lv.2 Green / cost 1 OR Lv.2 Black / cost 1; "Digivolve Lv.2
//!   w/[Royal Base] trait: Cost 0".
//! - {Security} [Opponent's Turn] All of your [Royal Base] trait Digimon gain
//!   <Reboot>.
//! - [Start of Your Main Phase] Add your top face-down security card to the
//!   hand. Then, you may place 1 [Royal Base] trait Digimon card from your hand
//!   face up as the bottom security card.
//! - Inherited: [All Turns] This Digimon gets +1000 DP.
//!
//! Regression (hunt cluster alt-digivolve-xros-req-cards): no YAML spec, so the
//! [Royal Base] cost-0 route lived only in `xros_req` — a Green/Black Lv.2
//! [Royal Base] base got no cost-choice prompt and an off-colour one could not
//! digivolve. New substrate: `add_top_face_down_security_to_hand`
//! (G-DSL-ADD-TOP-FACE-DOWN-SECURITY-TO-HAND). Synthetic boards only.

use crate::alt_digivolve_support::*;
use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, TriggerSource};

const ID: &str = "EX11-025";

// ─── Digivolution routes ──────────────────────────────────────────────────────

#[test]
fn ex11_025_green_or_black_lv2_circle_costs_1() {
    assert_single_route(ID, base("G2", "Green Rookie", 2, &[GREEN], &["Lesser"]), 1);
    assert_single_route(ID, base("K2", "Black Rookie", 2, &[BLACK], &["Lesser"]), 1);
}

#[test]
fn ex11_025_royal_base_lv2_of_any_colour_costs_0() {
    assert_single_route(ID, base("RB2", "Blue RB", 2, &[BLUE], &["Royal Base"]), 0);
}

#[test]
fn ex11_025_green_royal_base_lv2_prompts_cost_0_vs_1() {
    assert_cost_choice(ID, base("GRB2", "Green RB", 2, &[GREEN], &["Royal Base"]), 0, 1);
    assert_cost_choice(ID, base("KRB2", "Black RB", 2, &[BLACK], &["Royal Base"]), 0, 1);
}

#[test]
fn ex11_025_other_bases_have_no_route() {
    assert_not_offered(ID, base("B2", "Blue Rookie", 2, &[BLUE], &["Lesser"]));
    assert_not_offered(ID, base("RB3", "RB Champion", 3, &[GREEN], &["Royal Base"]));
}

// ─── Fixtures ─────────────────────────────────────────────────────────────────

fn digimon(id: &str, level: u8, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Green];
    c.level = Some(level);
    c.dp = Some(3000);
    c.play_cost = 3;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn builder() -> DebugRunnerBuilder {
    let mut tamer = make_test_card("RB-TAMER", "RB Tamer");
    tamer.card_kind = CardKind::Tamer;
    tamer.level = None;
    tamer.dp = None;
    tamer.traits = vec!["Royal Base".into()];
    DebugRunner::builder()
        .dsl_card(ID)
        .expect("EX11-025 production YAML loads")
        .add_card(digimon("RB-HAND", 3, &["Royal Base"]))
        .add_card(digimon("RB-OTHER", 4, &["Royal Base"]))
        .add_card(digimon("PLAIN-HAND", 3, &["Beast"]))
        .add_card(tamer)
        .add_card(digimon("SEC-A", 3, &["Beast"]))
        .add_card(digimon("SEC-B", 3, &["Beast"]))
        .add_card(digimon("SEC-C", 3, &["Beast"]))
        .add_card(digimon("F", 3, &["Beast"]))
        .deck(0, &["F"; 12])
        .deck(1, &["F"; 12])
        .memory(5)
}

fn fire_somp(r: &mut DebugRunner, host: PermanentHandle) {
    r.game
        .enqueue_triggered(EffectTiming::StartOfYourMainPhase, TriggerSource::Permanent(host));
    r.game.drain_effect_queue();
}

fn sec_ids(r: &DebugRunner) -> Vec<String> {
    // Bottom -> top, as the engine stores it.
    r.game.players[0]
        .security
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn hand_ids(r: &DebugRunner) -> Vec<String> {
    r.game.players[0]
        .hand
        .iter()
        .map(|c| c.card_id(&r.game.card_data).to_string())
        .collect()
}

fn setup(hand: &[&str]) -> (DebugRunner, PermanentHandle) {
    let mut r = builder().security(0, &["SEC-A", "SEC-B", "SEC-C"]).hand(0, hand).start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    let host = r.place_on_field(0, ID, Some(0));
    (r, host)
}

// ─── [Start of Your Main Phase] ───────────────────────────────────────────────

#[test]
fn ex11_025_adds_the_top_face_down_security_card_to_the_hand() {
    let (mut r, host) = setup(&[]);
    let top = sec_ids(&r).last().unwrap().clone();
    fire_somp(&mut r, host);
    assert_eq!(r.security_count(0), 2);
    assert!(hand_ids(&r).contains(&top), "the top security card is now in hand");
}

#[test]
fn ex11_025_skips_face_up_security_cards_and_takes_the_first_face_down_one() {
    let (mut r, host) = setup(&[]);
    let ids = sec_ids(&r);
    let (top, second) = (ids[2].clone(), ids[1].clone());
    // The top card is FACE UP: the "top face-down" card is the one beneath it.
    let top_index = r.game.players[0].security[2].card_index;
    r.game.players[0].face_up_security.insert(top_index);
    fire_somp(&mut r, host);
    assert_eq!(r.security_count(0), 2);
    assert!(hand_ids(&r).contains(&second), "the first face-down card was added");
    assert!(!hand_ids(&r).contains(&top), "the face-up top card stays in the stack");
    assert_eq!(sec_ids(&r).last().unwrap(), &top);
}

#[test]
fn ex11_025_adds_nothing_when_every_security_card_is_face_up() {
    let (mut r, host) = setup(&[]);
    let indexes: Vec<_> = r.game.players[0].security.iter().map(|c| c.card_index).collect();
    for i in indexes {
        r.game.players[0].face_up_security.insert(i);
    }
    fire_somp(&mut r, host);
    assert_eq!(r.security_count(0), 3);
    assert!(hand_ids(&r).is_empty());
}

#[test]
fn ex11_025_may_place_a_royal_base_digimon_from_hand_face_up_as_the_bottom_security() {
    let (mut r, host) = setup(&["RB-HAND", "PLAIN-HAND", "RB-TAMER", "RB-OTHER"]);
    fire_somp(&mut r, host);
    let view = r.pending_selection_view().expect("the optional placement parks");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(view.is_optional, "\"you may place\"");
    let picks: Vec<u16> = view.valid_action_ids.iter().copied().filter(|&a| a != PASS).collect();
    assert_eq!(picks.len(), 2, "only [Royal Base] Digimon cards (not the Tamer / plain): {picks:?}");
    r.execute_action(0, picks[0]).expect("place a Royal Base Digimon");

    assert_eq!(r.security_count(0), 3, "2 left after the add + 1 placed");
    let bottom = &r.game.players[0].security[0];
    assert!(bottom.card_id(&r.game.card_data).starts_with("RB-"), "placed as the BOTTOM card");
    assert!(
        r.game.players[0].face_up_security.contains(&bottom.card_index),
        "placed face up"
    );
}

#[test]
fn ex11_025_the_placement_is_optional_and_needs_a_royal_base_digimon() {
    let (mut r, host) = setup(&["RB-HAND"]);
    fire_somp(&mut r, host);
    r.execute_action(0, PASS).expect("decline the placement");
    assert!(r.pending_selection().is_none());
    assert_eq!(r.security_count(0), 2, "nothing placed");

    let (mut r, host) = setup(&["PLAIN-HAND", "RB-TAMER"]);
    fire_somp(&mut r, host);
    assert!(r.pending_selection().is_none(), "no [Royal Base] Digimon card to offer");
}

// ─── {Security} [Opponent's Turn] Reboot, inherited +1000 DP ──────────────────

fn reboot_runner() -> (DebugRunner, PermanentHandle, PermanentHandle) {
    let mut r = builder().security(0, &[ID, "SEC-A"]).start();
    r.skip_mulligan();
    r.game.turn_count = 1;
    let royal = r.place_on_field(0, "RB-HAND", Some(0));
    let plain = r.place_on_field(0, "PLAIN-HAND", Some(0));
    // FunBeemon sits at the bottom of the stack (index 0); mark it face up.
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.insert(idx);
    (r, royal, plain)
}

#[test]
fn ex11_025_face_up_in_security_grants_reboot_to_royal_base_digimon_on_the_opponents_turn() {
    let (mut r, royal, plain) = reboot_runner();
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(royal, Keyword::Reboot));
    assert!(!r.game.has_keyword(plain, Keyword::Reboot), "[Royal Base] trait only");

    r.game.turn_player_idx = 0;
    r.game.tick_declarative_effects();
    assert!(
        !r.game.has_keyword(royal, Keyword::Reboot),
        "[Opponent's Turn] only"
    );
}

#[test]
fn ex11_025_face_down_in_security_grants_nothing() {
    let (mut r, royal, _plain) = reboot_runner();
    let idx = r.game.players[0].security[0].card_index;
    r.game.players[0].face_up_security.remove(&idx);
    r.game.turn_player_idx = 1;
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(royal, Keyword::Reboot), "{{Security}} needs it face up");
}

#[test]
fn ex11_025_inherited_plus_1000_dp_on_the_host() {
    let (mut r, _royal, _plain) = reboot_runner();
    let host = r.place_stack(0, &[ID, "PLAIN-HAND"]);
    assert_eq!(r.effective_dp(host), Some(3000 + 1000));
}
