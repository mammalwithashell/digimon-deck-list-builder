//! EX13-049 Dorumon — Digimon, Lv.3, Black/Yellow, DP 1000, Cost 3.
//! Traits: Beast / X Antibody / Chronicle. Form: Rookie. Attribute: Data.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-049.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-049.md` agrees)
//!
//! ```text
//! Digivolve: Black Lv.2 / cost 1, Yellow Lv.2 / cost 1
//! [Digivolve] [Dorimon]/Black Lv.2 w/[X Antibody] trait: Cost 0
//!
//! Effect:
//! [When Moving] [On Play] Reveal the top 3 cards of your deck. Add 1 card with
//! the [X Antibody] or [Chronicle] trait among them to the hand. Return the rest
//! to the top or bottom of the deck.
//!
//! Inherited Effect:
//! [When Attacking] [Once Per Turn] 1 of your opponent's Digimon gets -2000 DP
//! for the turn.
//! ```
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_049.cs`). Printed text + general_rule.pdf are the
//! references; the reveal / DP idioms follow the closest DCGO-backed siblings
//! (EX13-008 reveal, BT24-058 top-or-bottom remainder).
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - A1 reveal-3 single-pick to hand (trait filter), remainder top-OR-bottom
//!   (EffectChoice + player-ordered permutation).
//! - Shared [When Moving] + [On Play] (on_move self-gated).
//! - Inherited [When Attacking][OPT] -DP for the turn.
//! - Printed circles + named / trait-gated alt digivolve.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::SelectionKind;

use super::chronicle_support::*;

const CARD_ID: &str = "EX13-049";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-049 YAML parses, compiles and is in the embedded pack")
        .dsl_card("EX13-006")
        .expect("EX13-006 Dorimon YAML loads")
        .add_card(digimon("XA-CARD", CardColor::Red, 4, 5000, &["X Antibody"]))
        .add_card(digimon("CHRON-CARD", CardColor::Blue, 5, 7000, &["Chronicle"]))
        .add_card(digimon("PLAIN-A", CardColor::Black, 3, 3000, &["Beast"]))
        .add_card(digimon("PLAIN-B", CardColor::Black, 3, 3000, &["Beast"]))
        .add_card(digimon("PLAIN-C", CardColor::Black, 3, 3000, &["Beast"]))
        .add_card(digimon("FILL", CardColor::Black, 3, 1000, &[]))
        .add_card(digimon("ATTACKER", CardColor::Black, 4, 6000, &[]))
        .add_card(digimon("OPP-A", CardColor::Red, 4, 5000, &[]))
        .add_card(digimon("OPP-B", CardColor::Red, 4, 6000, &[]))
        .add_card(digimon("BLACK-L2", CardColor::Black, 2, 0, &[]))
        .add_card(digimon("BLACK-XA-L2", CardColor::Black, 2, 0, &["X Antibody"]))
        .add_card(digimon("YELLOW-L2", CardColor::Yellow, 2, 0, &[]))
        .add_card(digimon("RED-L2", CardColor::Red, 2, 0, &["X Antibody"]))
}

fn triggered(runner: &DebugRunner, scope: CompiledScope) -> Vec<CompiledTriggeredClause> {
    runner
        .compiled_card(CARD_ID)
        .expect("compiled")
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) if t.scope == scope => Some(t.clone()),
            _ => None,
        })
        .collect()
}

// ─── Section 1 — Structural ──────────────────────────────────────────────────

#[test]
fn ex13_049_structure_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["Beast", "X Antibody", "Chronicle"]);
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(digivolve_paths, 4, "2 circles + [Dorimon] + Black Lv.2 w/[X Antibody]");

    let face = triggered(&runner, CompiledScope::FaceUp);
    assert_eq!(face.len(), 1);
    assert!(face[0].when.contains(&CompiledTiming::OnPlay));
    assert!(face[0].when.contains(&CompiledTiming::OnMove));
    assert!(!face[0].optional, "the reveal is mandatory");
    assert!(!face[0].once_per_turn);

    let inh = triggered(&runner, CompiledScope::Inherited);
    assert_eq!(inh.len(), 1);
    assert_eq!(inh[0].when, vec![CompiledTiming::WhenAttacking]);
    assert!(inh[0].once_per_turn, "[Once Per Turn]");
    assert!(!inh[0].optional, "the -2000 is mandatory");
}

// ─── Digivolution routes ─────────────────────────────────────────────────────

fn digivolve_onto(base_id: &str, memory: i16, cost: u16) -> (DebugRunner, bool) {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, base_id, Some(0));
    let prompted = digivolve_choosing_cost(&mut runner, 0, base, cost);
    (runner, prompted)
}

#[test]
fn ex13_049_digivolves_from_dorimon_for_0() {
    let (runner, prompted) = digivolve_onto("EX13-006", 3, 0);
    assert!(prompted, "Black Lv.2 circle (1) vs [Dorimon] route (0): the player picks");
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3, "[Dorimon]: Cost 0");
}

#[test]
fn ex13_049_digivolves_from_black_x_antibody_lv2_for_0() {
    let (runner, prompted) = digivolve_onto("BLACK-XA-L2", 3, 0);
    assert!(prompted, "circle (1) vs X-Antibody route (0): the player picks");
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3, "Black Lv.2 w/[X Antibody]: Cost 0");
}

#[test]
fn ex13_049_plain_black_lv2_uses_the_cost_1_circle() {
    let (runner, _) = digivolve_onto("BLACK-L2", 3, 1);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 2, "Black Lv.2 circle: cost 1");
}

#[test]
fn ex13_049_yellow_lv2_uses_the_cost_1_circle() {
    let (runner, _) = digivolve_onto("YELLOW-L2", 3, 1);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 2, "Yellow Lv.2 circle: cost 1");
}

#[test]
fn ex13_049_red_x_antibody_lv2_has_no_route() {
    let (runner, _) = digivolve_onto("RED-L2", 3, 0);
    assert!(!field_has(&runner, 0, CARD_ID), "red is neither a circle nor the black X-Antibody route");
}

// ─── Section 2/3 — [On Play] / [When Moving] reveal ─────────────────────────

fn play_dorumon(deck_top: &[&str]) -> DebugRunner {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    runner.skip_mulligan();
    stack_deck_top(&mut runner, 0, deck_top);
    runner.play(0, 0);
    runner
}

#[test]
fn ex13_049_on_play_offers_only_x_antibody_or_chronicle_cards() {
    let runner = play_dorumon(&["PLAIN-A", "CHRON-CARD", "XA-CARD"]);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::Reveal));
    assert!(reveal_offers(&runner, "XA-CARD"), "[X Antibody] trait offered");
    assert!(reveal_offers(&runner, "CHRON-CARD"), "[Chronicle] trait offered");
    assert!(!reveal_offers(&runner, "PLAIN-A"), "trait-less card not offered");
}

#[test]
fn ex13_049_on_play_adds_pick_and_returns_rest_to_bottom() {
    let mut runner = play_dorumon(&["PLAIN-A", "PLAIN-B", "XA-CARD"]);
    pick_revealed(&mut runner, "XA-CARD");
    assert_eq!(hand_ids(&runner, 0), vec!["XA-CARD"]);

    let view = runner.pending_selection_view().expect("top-or-bottom choice");
    assert_eq!(view.kind, SelectionKind::EffectChoice, "\"top or bottom\" is a player choice");
    runner.execute_branch(1).expect("bottom");
    finish_ordering(&mut runner);
    assert!(runner.pending_selection().is_none());
    let deck = deck_ids(&runner, 0);
    let mut bottom: Vec<_> = deck[..2].to_vec();
    bottom.sort();
    assert_eq!(bottom, vec!["PLAIN-A", "PLAIN-B"], "rest went to the bottom");
    assert_eq!(deck.len(), 5);
}

#[test]
fn ex13_049_on_play_can_return_rest_to_top() {
    let mut runner = play_dorumon(&["PLAIN-A", "PLAIN-B", "CHRON-CARD"]);
    pick_revealed(&mut runner, "CHRON-CARD");
    runner.execute_branch(0).expect("top");
    finish_ordering(&mut runner);
    let deck = deck_ids(&runner, 0);
    let mut top: Vec<_> = deck[deck.len() - 2..].to_vec();
    top.sort();
    assert_eq!(top, vec!["PLAIN-A", "PLAIN-B"], "rest went back on top");
}

#[test]
fn ex13_049_on_play_with_no_match_returns_all_three() {
    let mut runner = play_dorumon(&["PLAIN-A", "PLAIN-B", "PLAIN-C"]);
    decline_all(&mut runner);
    assert!(hand_ids(&runner, 0).is_empty(), "nothing eligible to add");
    assert_eq!(deck_ids(&runner, 0).len(), 6, "all three returned to the deck");
    assert!(runner.game.revealed_cards.is_empty());
}

#[test]
fn ex13_049_when_moving_out_of_breeding_fires_the_reveal() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    stack_deck_top(&mut runner, 0, &["PLAIN-A", "PLAIN-B", "XA-CARD"]);
    runner.place_in_breeding(0, CARD_ID);
    assert!(runner.move_from_breeding(0));
    assert!(reveal_offers(&runner, "XA-CARD"), "[When Moving] reveal fired");
}

#[test]
fn ex13_049_another_digimon_moving_does_not_fire_the_reveal() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_in_breeding(0, "PLAIN-A");
    assert!(runner.move_from_breeding(0));
    assert!(runner.pending_selection().is_none(), "[When Moving] is self-only");
    assert!(runner.game.revealed_cards.is_empty());
}

// ─── Inherited [When Attacking][OPT] -2000 ───────────────────────────────────

fn attack_setup() -> (DebugRunner, PermanentHandle, PermanentHandle, PermanentHandle) {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let attacker = runner.place_stack(0, &[CARD_ID, "ATTACKER"]);
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    (runner, attacker, opp_a, opp_b)
}

#[test]
fn ex13_049_inherited_when_attacking_gives_chosen_opponent_minus_2000() {
    let (mut runner, attacker, opp_a, opp_b) = attack_setup();
    runner.attack_player(attacker, 1, false);
    let view = runner.pending_selection_view().expect("-2000 target prompt");
    assert_eq!(view.kind, SelectionKind::OppField);
    assert!(field_offered(&runner, opp_a) && field_offered(&runner, opp_b));
    pick_field(&mut runner, opp_b);
    assert_eq!(runner.effective_dp(opp_b), Some(4000), "6000 - 2000");
    assert_eq!(runner.effective_dp(opp_a), Some(5000), "unchosen Digimon untouched");
}

#[test]
fn ex13_049_minus_2000_lasts_only_for_the_turn() {
    let (mut runner, attacker, _opp_a, opp_b) = attack_setup();
    runner.attack_player(attacker, 1, false);
    pick_field(&mut runner, opp_b);
    decline_all(&mut runner);
    assert_eq!(runner.effective_dp(opp_b), Some(4000));
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 1);
    assert_eq!(runner.effective_dp(opp_b), Some(6000), "\"for the turn\" expired");
}

#[test]
fn ex13_049_face_up_dorumon_does_not_grant_its_inherited_effect_to_itself() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let dorumon = runner.place_on_field(0, CARD_ID, Some(0));
    let opp = runner.place_on_field(1, "OPP-A", Some(0));
    runner.attack_player(dorumon, 1, false);
    decline_all(&mut runner);
    assert_eq!(runner.effective_dp(opp), Some(5000), "inherited only works from the stack");
}

#[test]
fn ex13_049_inherited_once_per_turn_lockout_and_reset() {
    let (mut runner, attacker, opp_a, _opp_b) = attack_setup();
    runner.attack_player(attacker, 1, false);
    pick_field(&mut runner, opp_a);
    decline_all(&mut runner);
    assert_eq!(runner.effective_dp(opp_a), Some(3000));

    // Second attack this turn: locked out.
    set_suspended(&mut runner, attacker, false);
    runner.attack_player(attacker, 1, false);
    assert_ne!(
        runner.pending_kind(),
        Some(SelectionKind::OppField),
        "second [When Attacking] this turn is locked out"
    );
    decline_all(&mut runner);
    assert_eq!(runner.effective_dp(opp_a), Some(3000), "no second -2000");

    // Next own turn: available again.
    next_turn(&mut runner);
    next_turn(&mut runner);
    assert_eq!(runner.turn_player(), 0);
    set_suspended(&mut runner, attacker, false);
    runner.attack_player(attacker, 1, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField), "OPT reset next turn");
}
