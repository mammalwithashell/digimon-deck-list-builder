//! P-187 Mastemon — Lv.6 Purple + Yellow, cost 13, 13000 DP.
//!
//! Printed text (data/card_bundles/P-187.md, official Bandai DB):
//!
//! ```text
//! [DNA Digivolve] Purple Lv.5 + yellow Lv.5: Cost 0. Stack the 2 specified
//!   Digimon and digivolve unsuspended.
//! [When Digivolving] <Recovery +1 (Deck)>. Then, if DNA digivolving, by
//!   placing 1 other Digimon or Tamer as the top or bottom security card, trash
//!   your opponent's top security card.
//! [When Digivolving][When Attacking][Once Per Turn] By trashing your top
//!   security card, you may play 1 purple or yellow Digimon card with 6000 DP or
//!   less from your hand or trash without paying the cost.
//! ```
//!
//! DCGO: DCGO/Assets/Scripts/CardEffect/P/Purple/P_187.cs. Synthetic boards.

#![allow(dead_code, unused_imports)]

use digimon_engine::action::space::PASS;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner, DebugRunnerBuilder};
use digimon_engine::effect_context::EffectContext;
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::selection::{SelectionKind, UnionZoneSet};

fn digimon(id: &str, level: u8, dp: i32, colors: Vec<CardColor>) -> CardData {
    let mut c = make_test_card(id, id);
    c.card_kind = CardKind::Digimon;
    c.level = Some(level);
    c.dp = Some(dp);
    c.play_cost = 4;
    c.colors = colors;
    c
}

fn base() -> DebugRunnerBuilder {
    DebugRunner::builder()
        .dsl_card("P-187")
        .expect("P-187 compiles into the embedded pack")
        .add_card(digimon("T-PY6000", 4, 6000, vec![CardColor::Purple]))
        .add_card(digimon("T-Y6000", 4, 6000, vec![CardColor::Yellow]))
        .add_card(digimon("T-PY7000", 4, 7000, vec![CardColor::Purple]))
        .add_card(digimon("T-RED3000", 4, 3000, vec![CardColor::Red]))
        .add_card(digimon("T-MAT-P", 5, 5000, vec![CardColor::Purple]))
        .add_card(digimon("T-MAT-Y", 5, 5000, vec![CardColor::Yellow]))
        .add_card(digimon("T-OTHER", 3, 3000, vec![CardColor::Blue]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
}

fn on_field(runner: &DebugRunner, p: usize, id: &str) -> bool {
    runner.game.players[p]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == id)
}

#[test]
fn p_187_has_dna_alt_path_and_two_trigger_clauses() {
    let runner = base().memory(10).start();
    let compiled = runner.compiled_card("P-187").expect("compiled");
    assert_eq!(compiled.alt_paths.len(), 1, "purple Lv.5 + yellow Lv.5, cost 0");
    assert_eq!(compiled.effects.len(), 2);
}

/// [When Attacking][OPT]: by trashing the top security card, play a purple or
/// yellow Digimon with 6000 DP or less from hand or trash for free. Only
/// matching cards are offered.
#[test]
fn p_187_when_attacking_trashes_top_security_to_play_a_small_purple_or_yellow_digimon() {
    let mut runner = base()
        .hand(0, &["T-PY6000", "T-PY7000", "T-RED3000"])
        .security(0, &["FILL", "FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    runner.inject_trash(0, "T-Y6000");
    let host = runner.place_on_field(0, "P-187", Some(0));
    runner.game.enter_main_phase();
    let memory_before = runner.game.memory;
    let _ = runner.attack_player(host, 1, false);

    // The optional "by trashing" gate, then the union pick (hand + trash).
    let mut view = runner.pending_selection_view().expect("optional prompt parked");
    if !matches!(view.kind, SelectionKind::UnionZone { .. }) {
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("accept the optional effect");
        runner.game.drain_effect_queue();
        view = runner.pending_selection_view().expect("union pick parked");
    }
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    let picks: Vec<u16> = view
        .valid_action_ids
        .iter()
        .copied()
        .filter(|&a| a != PASS)
        .collect();
    assert_eq!(
        picks.len(),
        2,
        "T-PY6000 (hand) and T-Y6000 (trash): not the 7000-DP or red card: {picks:?}"
    );
    assert_eq!(runner.security_count(0), 1, "the top security card was trashed as the cost");
    runner
        .execute_action(view.selecting_player, picks[0])
        .expect("play it");
    runner.auto_resolve().expect("finish");
    assert!(
        on_field(&runner, 0, "T-PY6000") || on_field(&runner, 0, "T-Y6000"),
        "a qualifying Digimon was played"
    );
    assert_eq!(runner.game.memory, memory_before, "played free");
}

/// Run the DNA digivolve into P-187 and answer every prompt with its first
/// offered action, except the Top/Bottom security-position choice, which takes
/// `position_branch` (0 = top, 1 = bottom).
fn dna_into_mastemon(position_branch: usize) -> DebugRunner {
    let mut runner = base()
        .hand(0, &["P-187"])
        .security(0, &["FILL", "FILL"])
        .security(1, &["FILL", "FILL", "FILL"])
        .memory(5)
        .start();
    let mat_a = runner.place_on_field(0, "T-MAT-P", None);
    let mat_b = runner.place_on_field(0, "T-MAT-Y", None);
    runner.place_on_field(0, "T-OTHER", None);
    let hand_card = runner.game.player(0).hand[0].handle();
    {
        let mut ctx = EffectContext::new(&mut runner.game, hand_card, None, 0);
        ctx.effect_initiated_dna_digivolve(mat_a, mat_b, hand_card, 0, true);
    }
    runner.game.drain_effect_queue();
    let mut guard = 0;
    let mut saw_position_choice = false;
    while let Some(view) = runner.pending_selection_view() {
        assert!(guard < 16, "selection loop did not terminate");
        if view.kind == SelectionKind::EffectChoice {
            saw_position_choice = true;
            runner.execute_branch(position_branch).expect("choose top/bottom");
        } else {
            runner
                .execute_action(view.selecting_player, view.valid_action_ids[0])
                .expect("drive selection");
        }
        runner.game.drain_effect_queue();
        guard += 1;
    }
    assert!(saw_position_choice, "the top-or-bottom security choice is offered");
    runner
}

fn security_ids(runner: &DebugRunner, p: usize) -> Vec<String> {
    runner.game.players[p]
        .security
        .iter()
        .map(|c| c.card_id(&runner.game.card_data).to_string())
        .collect()
}

/// DNA digivolving: Recovery +1, then (optionally) placing 1 other Digimon or
/// Tamer as the TOP security card trashes the opponent's top security card.
#[test]
fn p_187_dna_digivolve_recovers_then_places_a_permanent_on_top_to_trash_opposing_security() {
    let runner = dna_into_mastemon(0);
    assert!(on_field(&runner, 0, "P-187"), "Mastemon is on the field");
    assert!(!on_field(&runner, 0, "T-OTHER"), "the placed permanent left the field");
    assert_eq!(runner.security_count(1), 2, "the opponent's top security card was trashed");
    // Recovery +1 (deck -> security) and the placed permanent (+1): 2 -> 4. The
    // second clause is not offered (no purple/yellow Digimon in hand or trash).
    assert_eq!(runner.security_count(0), 4);
    assert_eq!(
        security_ids(&runner, 0).last().map(String::as_str),
        Some("T-OTHER"),
        "placed as the TOP security card"
    );
}

/// Q&A: the card may also go on the BOTTOM of security.
#[test]
fn p_187_dna_digivolve_may_place_the_permanent_on_the_bottom() {
    let runner = dna_into_mastemon(1);
    assert_eq!(runner.security_count(0), 4);
    assert_eq!(runner.security_count(1), 2, "the opponent's top security card was trashed");
    assert_eq!(
        security_ids(&runner, 0).first().map(String::as_str),
        Some("T-OTHER"),
        "placed as the BOTTOM security card"
    );
}

/// CR 4.3 15-7-4: "By trashing your top security card" is an optional
/// processing condition. With 0 own security it cannot be executed, so the
/// player cannot choose to execute it: the clause is not offered and the
/// Digimon is NOT played for free. (DCGO P_187.cs CanActivateCondition also
/// requires own security > 0.)
#[test]
fn p_187_when_attacking_is_not_offered_with_no_own_security() {
    let mut runner = base()
        .hand(0, &["T-PY6000"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    runner.inject_trash(0, "T-Y6000");
    assert_eq!(runner.security_count(0), 0);
    let host = runner.place_on_field(0, "P-187", Some(0));
    runner.game.enter_main_phase();
    let _ = runner.attack_player(host, 1, false);
    runner.game.drain_effect_queue();

    // The clause must not be offered at all: no optional confirm, no pick.
    assert!(
        runner.pending_selection_view().is_none(),
        "the [By trashing your top security card] clause must not be offered with 0 own security: {:?}",
        runner.pending_selection_view().map(|v| v.kind)
    );
    assert!(!on_field(&runner, 0, "T-PY6000"), "nothing played free from hand");
    assert!(!on_field(&runner, 0, "T-Y6000"), "nothing played free from trash");
}

/// 15-7-5 / DCGO: when the cost IS payable the security card is trashed first,
/// even if the player then declines the play.
#[test]
fn p_187_security_is_trashed_as_cost_even_when_the_play_is_declined() {
    let mut runner = base()
        .hand(0, &["T-PY6000"])
        .security(0, &["FILL", "FILL"])
        .security(1, &["FILL", "FILL"])
        .memory(5)
        .start();
    let host = runner.place_on_field(0, "P-187", Some(0));
    runner.game.enter_main_phase();
    let _ = runner.attack_player(host, 1, false);
    let mut view = runner.pending_selection_view().expect("optional prompt parked");
    if !matches!(view.kind, SelectionKind::UnionZone { .. }) {
        runner
            .execute_action(view.selecting_player, view.valid_action_ids[0])
            .expect("accept the optional effect");
        runner.game.drain_effect_queue();
        view = runner.pending_selection_view().expect("union pick parked");
    }
    assert!(matches!(view.kind, SelectionKind::UnionZone { .. }));
    assert_eq!(runner.security_count(0), 1, "cost paid before the pick");
    runner.execute_action(view.selecting_player, PASS).expect("decline the play");
    runner.game.drain_effect_queue();
    assert_eq!(runner.security_count(0), 1, "security stays trashed");
    assert!(!on_field(&runner, 0, "T-PY6000"), "declined: nothing played");
}
