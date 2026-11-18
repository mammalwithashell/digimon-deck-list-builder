//! P-193 The Wicked God Emerges! -- the printed `[End of All Turns] <Delay>`
//! window (hunt cluster `delay-trigger-substrate`).
//!
//! Printed: "[End of All Turns] <Delay> (By trashing this card after the
//! placing turn, activate the effect below.)  - By deleting 1 of your
//! [Millenniummon], you may play 1 [Wicked God] trait Digimon card from your
//! hand or trash without paying the cost."
//!
//! DCGO (`P_193.cs`) registers the clause on `EffectTiming.OnEndTurn`, optional
//! (`SetUpActivateClass(..., isOptional: true, ...)`), usable while
//! `CanDeclareOptionDelayEffect` -- the Option is on the battle area and its
//! enter-field turn is not the current turn. `StackSkillInfos(null, OnEndTurn)`
//! collects every battle-area permanent regardless of whose turn is ending, so
//! the prompt appears at the end of EVERY turn after the placing turn, to the
//! Option's controller. CR 4.3 16-17-1 / 16-17-2 (optional) / 15-16-12-3
//! ("[End of All Turns]").
//!
//! The pre-fix engine authored it as the player-initiated `[Main]` Delay, so no
//! end-of-turn prompt ever existed for DCGO's decline answer to land on.

#![allow(unused_imports, dead_code)]

use digimon_engine::action::space::{PASS, PLAY_HAND_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::permanent::OptionState;
use digimon_engine::selection::SelectionKind;

const P_193_YAML: &str = include_str!("../../../cards/p/P-193.yaml");

fn digimon(id: &str, name: &str, traits: &[&str]) -> CardData {
    let mut c = make_test_card(id, name);
    c.card_kind = CardKind::Digimon;
    c.colors = vec![CardColor::Purple];
    c.level = Some(6);
    c.dp = Some(12000);
    c.play_cost = 5;
    c.traits = traits.iter().map(|t| t.to_string()).collect();
    c
}

fn delay_on_field(runner: &DebugRunner, player: usize) -> bool {
    runner.game.players[player].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "P-193"
            && matches!(perm.option_state, OptionState::Delayed { .. })
    })
}

fn trash_has(runner: &DebugRunner, player: usize, id: &str) -> bool {
    runner.game.players[player]
        .trash
        .iter()
        .any(|c| c.card_id(&runner.game.card_data) == id)
}

/// P0 uses P-193's [Main] (trashing a [Composite] fodder), leaving it parked as
/// a Delay Option. P0 also controls a [Millenniummon]; a [Wicked God] Digimon
/// waits in P0's hand.
fn place_p193() -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(P_193_YAML)
        .expect("P-193 YAML parses")
        .add_card(digimon("P193-FODDER", "Fodder", &["Composite"]))
        .add_card(digimon("P193-MILL", "Millenniummon", &[]))
        .add_card(digimon("P193-WG", "SomeWickedGod", &["Wicked God"]))
        .add_card(make_test_card("FILL", "Filler"))
        .memory(10)
        .hand(0, &["P-193", "P193-FODDER", "P193-WG"])
        .hand(1, &["FILL"])
        .deck(0, &["FILL"; 8])
        .deck(1, &["FILL"; 8])
        .start();
    runner.place_on_field(0, "P193-MILL", Some(0));

    assert!(runner.game.activate_hand_main(0, 0), "P-193 [Main] activates");
    let view = runner.pending_selection_view().expect("trash-cost pick");
    // Hand: P-193 (0), FODDER (1), WG (2) -- trash the [Composite] fodder.
    runner
        .game
        .resolve_selection(view.selecting_player, PLAY_HAND_START + 1)
        .expect("trash the fodder");
    runner.game.drain_effect_queue();
    assert!(delay_on_field(&runner, 0), "P-193 is placed as a Delay Option");
    runner
}

/// The placing turn's own end offers no window (CanDeclareOptionDelayEffect:
/// the enter-field turn is the current turn), but the next turn end -- the
/// OPPONENT's -- does, to P-193's controller. Declining keeps the Option and it
/// is offered again at the following turn end.
#[test]
fn p193_end_of_all_turns_delay_is_offered_after_the_placing_turn_and_declinable() {
    let mut runner = place_p193();

    // End of the placing turn (P0's): no window yet.
    runner.end_turn();
    assert!(
        runner.pending_selection_view().is_none(),
        "no <Delay> window at the end of the placing turn"
    );
    assert_eq!(runner.game.turn_player(), 1);
    assert!(delay_on_field(&runner, 0));

    // End of the opponent's (P1's) turn: [End of All Turns] reaches it.
    runner.end_turn();
    let view = runner
        .pending_selection_view()
        .expect("DCGO OnEndTurn: the controller is offered the <Delay> at the opponent's turn end");
    assert_eq!(view.selecting_player, 0, "the Option's controller decides");
    assert!(runner.pending_is_optional(), "16-17-2: the <Delay> is optional");
    runner
        .execute_action(view.selecting_player, PASS)
        .expect("decline the <Delay>");
    runner.auto_resolve().expect("finish the turn boundary");

    assert!(
        delay_on_field(&runner, 0),
        "a declined <Delay> stays in the battle area (16-17-1)"
    );
    assert!(!trash_has(&runner, 0, "P-193"));

    // It is offered again at the next turn end (P0's own).
    runner.end_turn();
    let view = runner
        .pending_selection_view()
        .expect("still offered at the next turn end after a decline");
    assert_eq!(view.selecting_player, 0);
}

/// Accepting at the end of the turn trashes the Option as the cost, then runs
/// the body (delete a [Millenniummon], play a [Wicked God] free).
#[test]
fn p193_end_of_all_turns_delay_accept_trashes_option_and_runs_body() {
    let mut runner = place_p193();
    runner.end_turn();
    runner.end_turn();

    let outer = runner.pending_selection_view().expect("the <Delay> window");
    let accept = outer
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .expect("accept branch");
    runner
        .execute_action(outer.selecting_player, accept)
        .expect("accept the <Delay>");

    assert!(!delay_on_field(&runner, 0), "the Option is trashed as the cost");
    assert!(trash_has(&runner, 0, "P-193"));

    let view = runner
        .pending_selection_view()
        .expect("the Millenniummon delete pick");
    assert_eq!(view.kind, SelectionKind::OwnField);
    let action = view.valid_action_ids[0];
    runner
        .execute_action(view.selecting_player, action)
        .expect("delete the Millenniummon");
    let view = runner.pending_selection_view().expect("free-play pick");
    let action = view
        .valid_action_ids
        .iter()
        .copied()
        .find(|a| *a != PASS)
        .unwrap();
    runner
        .execute_action(view.selecting_player, action)
        .expect("play the Wicked God free");
    runner.auto_resolve().expect("settle");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "P193-WG"),
        "the [Wicked God] Digimon was played without paying the cost"
    );
}
