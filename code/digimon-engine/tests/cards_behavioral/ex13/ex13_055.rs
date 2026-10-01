//! EX13-055 Raptordramon — Digimon, Lv.4, Black/Yellow, DP 5000, Cost 5.
//! Traits: Cyborg / X Antibody / Chronicle. Form: Champion. Attribute: Vaccine.
//!
//! # Card text (per-card JSON `cards/ex13/EX13-055.json`; official Bandai DB
//! bundle `data/card_bundles/EX13-055.md` agrees)
//!
//! ```text
//! Digivolve: Black Lv.3 / cost 3, Yellow Lv.3 / cost 3
//! [Digivolve] [Dorumon]/Lv.3 w/[Chronicle] trait: Cost 2
//!
//! Effect:
//! [On Play] [When Digivolving] 1 of your opponent's Digimon gets -3000 DP for
//! the turn.
//! [When Attacking] This Digimon may digivolve into a Digimon card with the
//! [Chronicle] trait in the hand or trash.
//!
//! Inherited Effect:
//! ＜Barrier＞ (When this Digimon would be deleted in battle, by trashing your
//! top security card, it isn't deleted.)
//! ```
//!
//! # DCGO C# reference
//! None at b9a0638cd (no `EX13_055.cs`). Closest DCGO-backed sibling:
//! BT20-053 Grademon (alt route "[Raptordramon] || Lv.4 w/[Chronicle]").
//! The When-Attacking digivolve follows BT25-092's hand∪trash union pick
//! filtered by `can_digivolve_onto`, paying the printed digivolution cost.
//!
//! # Patterns (RUST_DSL_TEST_API §4.3)
//! - [On Play]+[When Digivolving] single-target -DP for the turn.
//! - [When Attacking] optional effect-digivolve from hand ∪ trash (paid cost).
//! - Inherited static keyword (＜Barrier＞).
//! - Printed circles + named / trait-gated alt digivolve.

use digimon_dsl::compiled::{
    CompiledAltPathKind, CompiledClause, CompiledScope, CompiledTiming, CompiledTriggeredClause,
};
use digimon_engine::action::space::PASS;
use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

use super::chronicle_support::*;

const CARD_ID: &str = "EX13-055";

fn builder() -> digimon_engine::debug_runner::DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX13-055 YAML parses, compiles and is in the embedded pack")
        .dsl_card("EX13-049")
        .expect("EX13-049 Dorumon YAML loads")
        .add_card(digimon("FILL", CardColor::Black, 3, 1000, &[]))
        .add_card(digimon("BLACK-L3", CardColor::Black, 3, 3000, &[]))
        .add_card(digimon("RED-CHRON-L3", CardColor::Red, 3, 3000, &["Chronicle"]))
        .add_card(digimon("RED-L3", CardColor::Red, 3, 3000, &[]))
        .add_card(with_evo(
            digimon("CHRON-L5", CardColor::Black, 5, 8000, &["Chronicle"]),
            CardColor::Black,
            4,
            4,
        ))
        .add_card(with_evo(
            digimon("CHRON-L5-T", CardColor::Black, 5, 8000, &["Chronicle"]),
            CardColor::Black,
            4,
            4,
        ))
        .add_card(with_evo(
            digimon("PLAIN-L5", CardColor::Black, 5, 8000, &["Beast"]),
            CardColor::Black,
            4,
            4,
        ))
        .add_card(with_evo(
            digimon("CHRON-RED-L5", CardColor::Red, 5, 8000, &["Chronicle"]),
            CardColor::Red,
            4,
            4,
        ))
        .add_card(digimon("CARRIER", CardColor::Black, 5, 4000, &[]))
        .add_card(digimon("OPP-A", CardColor::Red, 4, 5000, &[]))
        .add_card(digimon("OPP-B", CardColor::Red, 4, 6000, &[]))
        .add_card(digimon("OPP-BIG", CardColor::Red, 6, 12000, &[]))
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
fn ex13_055_structure_matches_printed_text() {
    let runner = builder().start();
    let card = runner.compiled_card(CARD_ID).expect("compiled");
    assert_eq!(card.traits, vec!["Cyborg", "X Antibody", "Chronicle"]);
    let digivolve_paths = card
        .alt_paths
        .iter()
        .filter(|p| matches!(p.kind, CompiledAltPathKind::Digivolve))
        .count();
    assert_eq!(digivolve_paths, 4, "2 circles + [Dorumon] + Lv.3 w/[Chronicle]");

    let face = triggered(&runner, CompiledScope::FaceUp);
    assert_eq!(face.len(), 2, "[OP][WD] -3000 and [When Attacking] digivolve");
    let dp = face
        .iter()
        .find(|c| c.when.contains(&CompiledTiming::OnPlay))
        .expect("[On Play] clause");
    assert!(dp.when.contains(&CompiledTiming::WhenDigivolving));
    assert!(!dp.optional && !dp.once_per_turn);
    let wa = face
        .iter()
        .find(|c| c.when == vec![CompiledTiming::WhenAttacking])
        .expect("[When Attacking] clause");
    assert!(!wa.once_per_turn, "no [Once Per Turn] printed");
    assert!(triggered(&runner, CompiledScope::Inherited).is_empty(), "inherited is a static keyword");
}

#[test]
fn ex13_055_inherited_barrier_is_granted_to_the_carrier_only_from_the_stack() {
    let mut runner = builder().start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let plain = runner.place_on_field(0, "CARRIER", Some(0));
    assert!(runner.game.has_keyword(carrier, Keyword::Barrier), "inherited ＜Barrier＞");
    assert!(!runner.game.has_keyword(plain, Keyword::Barrier));
}

#[test]
fn ex13_055_barrier_trashes_top_security_instead_of_battle_deletion() {
    let mut runner = builder()
        .deck(0, &["FILL", "FILL", "FILL"])
        .security(0, &["FILL", "FILL"])
        .memory(3)
        .start();
    runner.skip_mulligan();
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);
    let big = runner.place_on_field(1, "OPP-BIG", Some(0));
    set_suspended(&mut runner, big, true);
    runner.attack_digimon(carrier, big, false);
    // Accept every prompt of ours (the Barrier window); PASS the opponent's.
    for _ in 0..16 {
        let Some(view) = runner.pending_selection_view() else { break };
        let a = if view.selecting_player == 0 {
            view.valid_action_ids.iter().copied().find(|&a| a != PASS).unwrap_or(PASS)
        } else {
            PASS
        };
        runner.execute_action(view.selecting_player, a).expect("resolve");
    }
    assert!(field_has(&runner, 0, "CARRIER"), "＜Barrier＞ prevented the battle deletion");
    assert_eq!(runner.security_count(0), 1, "top security card trashed");
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
fn ex13_055_digivolves_from_dorumon_for_2() {
    let (runner, prompted) = digivolve_onto("EX13-049", 5, 2);
    assert!(prompted, "circle (3) vs [Dorumon] route (2): the player picks");
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3, "[Dorumon]: Cost 2");
}

#[test]
fn ex13_055_digivolves_from_any_colour_lv3_chronicle_for_2() {
    let (runner, _) = digivolve_onto("RED-CHRON-L3", 5, 2);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 3, "Lv.3 w/[Chronicle]: Cost 2 (no colour gate)");
}

#[test]
fn ex13_055_black_lv3_uses_the_cost_3_circle() {
    let (runner, _) = digivolve_onto("BLACK-L3", 5, 3);
    assert!(field_has(&runner, 0, CARD_ID));
    assert_eq!(runner.memory(), 2, "Black Lv.3 circle: cost 3");
}

#[test]
fn ex13_055_red_lv3_without_chronicle_has_no_route() {
    let (runner, _) = digivolve_onto("RED-L3", 5, 2);
    assert!(!field_has(&runner, 0, CARD_ID));
}

// ─── [On Play] / [When Digivolving] -3000 ────────────────────────────────────

#[test]
fn ex13_055_on_play_gives_chosen_opponent_minus_3000_for_the_turn() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .deck(1, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(6)
        .start();
    runner.skip_mulligan();
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    runner.play(0, 0);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField));
    assert!(!runner.pending_is_optional(), "the -3000 is mandatory");
    pick_field(&mut runner, opp_b);
    assert_eq!(runner.effective_dp(opp_b), Some(3000));
    assert_eq!(runner.effective_dp(opp_a), Some(5000));
    next_turn(&mut runner);
    assert_eq!(runner.effective_dp(opp_b), Some(6000), "\"for the turn\" expired");
}

#[test]
fn ex13_055_when_digivolving_gives_minus_3000() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(6)
        .start();
    runner.skip_mulligan();
    let base = runner.place_on_field(0, "EX13-049", Some(0));
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    digivolve_choosing_cost(&mut runner, 0, base, 2);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OppField), "[When Digivolving] fired");
    pick_field(&mut runner, opp_a);
    assert_eq!(runner.effective_dp(opp_a), Some(2000));
}

#[test]
fn ex13_055_on_play_without_opponent_digimon_fizzles() {
    let mut runner = builder()
        .hand(0, &[CARD_ID])
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .memory(6)
        .start();
    runner.skip_mulligan();
    runner.play(0, 0);
    assert!(runner.pending_selection().is_none(), "no target, nothing to choose");
    assert!(field_has(&runner, 0, CARD_ID));
}

// ─── [When Attacking] may digivolve into a [Chronicle] card ──────────────────

fn attack_setup(hand: &[&str], trash: &[&str], memory: i16) -> (DebugRunner, PermanentHandle) {
    let mut runner = builder()
        .hand(0, hand)
        .deck(0, &["FILL", "FILL", "FILL", "FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(memory)
        .start();
    runner.skip_mulligan();
    for id in trash {
        runner.inject_trash(0, id);
    }
    let rap = runner.place_on_field(0, CARD_ID, Some(0));
    (runner, rap)
}

#[test]
fn ex13_055_when_attacking_offers_chronicle_cards_in_hand_and_trash() {
    let (mut runner, rap) = attack_setup(&["CHRON-L5", "PLAIN-L5", "CHRON-RED-L5"], &["CHRON-L5-T"], 6);
    runner.attack_player(rap, 1, false);
    let view = runner.pending_selection_view().expect("digivolve pick");
    assert!(
        matches!(view.kind, SelectionKind::UnionZone { zones } if zones == (UnionZoneSet::HAND | UnionZoneSet::TRASH)),
        "one pick over hand ∪ trash: {view:?}"
    );
    assert!(view.is_optional, "\"may digivolve\"");
    assert!(hand_offered(&runner, 0, "CHRON-L5"));
    assert!(trash_offered(&runner, 0, "CHRON-L5-T"));
    assert!(!hand_offered(&runner, 0, "PLAIN-L5"), "non-[Chronicle] card not offered");
    assert!(
        !hand_offered(&runner, 0, "CHRON-RED-L5"),
        "a [Chronicle] card with no legal route onto Raptordramon is not offered"
    );
    let _ = rap;
}

#[test]
fn ex13_055_when_attacking_digivolves_from_hand_paying_the_cost() {
    let (mut runner, rap) = attack_setup(&["CHRON-L5"], &[], 6);
    runner.attack_player(rap, 1, false);
    pick_hand(&mut runner, 0, "CHRON-L5");
    assert_eq!(top_id(&runner, rap), "CHRON-L5");
    assert_eq!(runner.memory(), 2, "printed Black Lv.4 circle cost 4 paid");
}

#[test]
fn ex13_055_when_attacking_digivolves_from_trash() {
    let (mut runner, rap) = attack_setup(&[], &["CHRON-L5-T"], 6);
    runner.attack_player(rap, 1, false);
    pick_trash(&mut runner, 0, "CHRON-L5-T");
    assert_eq!(top_id(&runner, rap), "CHRON-L5-T");
    assert!(!trash_ids(&runner, 0).contains(&"CHRON-L5-T".to_string()));
    assert_eq!(runner.memory(), 2);
}

#[test]
fn ex13_055_when_attacking_digivolve_can_be_declined() {
    let (mut runner, rap) = attack_setup(&["CHRON-L5"], &[], 6);
    runner.attack_player(rap, 1, false);
    decline(&mut runner);
    assert_eq!(top_id(&runner, rap), CARD_ID);
    assert_eq!(runner.memory(), 6);
    assert!(hand_ids(&runner, 0).contains(&"CHRON-L5".to_string()));
}

#[test]
fn ex13_055_when_attacking_with_no_chronicle_card_offers_nothing() {
    let (mut runner, rap) = attack_setup(&["PLAIN-L5"], &[], 6);
    runner.attack_player(rap, 1, false);
    assert!(
        !matches!(runner.pending_kind(), Some(SelectionKind::UnionZone { .. })),
        "no legal [Chronicle] result → no digivolve prompt"
    );
    decline_all(&mut runner);
    assert_eq!(top_id(&runner, rap), CARD_ID);
}
