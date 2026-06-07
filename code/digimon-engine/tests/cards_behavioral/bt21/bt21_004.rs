//! BT21-004 Koromon — Digi-Egg (In-Training), Lv.2, Yellow, Cost 0, no DP.
//! Traits: Lesser, Hero.
//!
//! # Card text (BT21-004.json) — INHERITED ONLY
//! `Inherited: [Your Turn] [Once Per Turn] When any of your yellow or red`
//! `Tamers suspend, ＜Draw 1＞ (Draw 1 card from your deck.)`
//!
//! # DCGO C# reference
//! DCGO/Assets/Scripts/CardEffect/BT21/Yellow/BT21_004.cs
//!   - OnTappedAnyone, IsInheritedEffect, IsOwnerTurn, maxUsableCount: 1,
//!     own Tamer w/ Yellow or Red color → DrawClass(1), isOptional: false.
//!
//! # Patterns (test API §4.3): G4 (inherited DigiEgg), B3-adjacent (event
//! observer of own-Tamer suspend), E2 (OPT). Body is MANDATORY (no "may").
//!
//! Inherited dispatch is exercised with BT21-004 as a digivolution SOURCE under
//! a host Digimon (an inherited clause only fires from a source, never as the
//! top card).

#![allow(dead_code, unused_imports)]

#[path = "../../support/dsl_card_data.rs"]
mod dsl_card_data;

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, PlayerId};
use digimon_engine::permanent::PermanentHandle;

fn egg() -> digimon_dsl::compiled::CompiledCard {
    dsl_card_data::compiled("BT21-004")
}

fn make_tamer(card_id: &str, name: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.dp = None;
    card.level = None;
    card
}

fn make_digimon(card_id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.dp = Some(3000);
    card.level = Some(4);
    card
}

/// Runner with BT21-004 as a digivolution source under a yellow host Digimon,
/// plus Tamers to suspend and stocked decks (so turn rotation does not deck-out
/// during OPT-clear tests).
fn egg_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT21-004")
        .expect("BT21-004 in pack")
        .add_card(make_digimon("HOST", CardColor::Yellow))
        .add_card(make_tamer("RED-TAMER", "Red Tamer", CardColor::Red))
        .add_card(make_tamer("YEL-TAMER", "Yellow Tamer", CardColor::Yellow))
        .add_card(make_tamer("GRN-TAMER", "Green Tamer", CardColor::Green))
        .add_card(make_digimon("FILLER", CardColor::Blue))
        .deck(0, &["FILLER"; 20])
        .deck(1, &["FILLER"; 20])
        .memory(10)
        .start()
}

/// Place a host Digimon for `player` with BT21-004 underneath as a source.
fn host_with_egg(runner: &mut DebugRunner, player: PlayerId) -> PermanentHandle {
    runner.place_stack(player, &["BT21-004", "HOST"])
}

// ─── SECTION 1 — Structural ─────────────────────────────────────────────────

#[test]
fn bt21_004_metadata_is_yellow_lv2_egg_no_dp() {
    let card = egg();
    assert_eq!(card.card, "BT21-004");
    assert_eq!(card.name, "Koromon");
    assert_eq!(card.level, Some(2));
    assert_eq!(card.dp, None, "an In-Training egg has no DP");
    assert!(card
        .color
        .contains(&digimon_dsl::compiled::CompiledColor::Yellow));
}

#[test]
fn bt21_004_has_single_inherited_opt_suspend_clause_no_main_effect() {
    let card = egg();
    let triggered: Vec<_> = card
        .effects
        .iter()
        .filter_map(|c| match c {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "exactly one clause (the inherited draw)");
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited, "must be inherited");
    assert!(clause.when.contains(&CompiledTiming::OnSuspend));
    assert!(clause.once_per_turn, "[Once Per Turn]");
    assert!(!clause.optional, "mandatory draw — no 'may'");
}

// ─── SECTION 2 — Behavioral (inherited dispatch) ────────────────────────────

#[test]
fn bt21_004_draws_when_own_yellow_tamer_suspends() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    let deck_before = runner.game.players[0].deck.len();

    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "own yellow Tamer suspend → Draw 1"
    );
    assert_eq!(runner.game.players[0].deck.len(), deck_before - 1);
}

#[test]
fn bt21_004_draws_when_own_red_tamer_suspends() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);
}

// ─── SECTION 2b — Negatives ─────────────────────────────────────────────────

#[test]
fn bt21_004_does_not_draw_on_green_tamer_suspend() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let tamer = runner.place_on_field(0, "GRN-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "a green Tamer is not yellow/red → no draw"
    );
}

#[test]
fn bt21_004_does_not_draw_on_own_digimon_suspend() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let ally = runner.place_on_field(0, "FILLER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(ally);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "a Digimon (not a Tamer) suspending → no draw"
    );
}

#[test]
fn bt21_004_does_not_draw_on_opponent_tamer_suspend() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let opp_tamer = runner.place_on_field(1, "YEL-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(opp_tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "the OPPONENT's Tamer suspending does not satisfy 'your' Tamers"
    );
}

#[test]
fn bt21_004_does_not_draw_on_opponents_turn() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));
    runner.game.turn_player_idx = 1; // opponent's turn

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before,
        "[Your Turn] gate blocks the draw on the opponent's turn"
    );
}

// ─── SECTION 3 — OPT lockout + clear ────────────────────────────────────────

#[test]
fn bt21_004_opt_blocks_second_suspend_same_turn() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let t1 = runner.place_on_field(0, "YEL-TAMER", Some(0));
    let t2 = runner.place_on_field(0, "RED-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(t1);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);

    runner.game.suspend(t2);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_before + 1,
        "[Once Per Turn] blocks a second suspend-draw the same turn"
    );
}

#[test]
fn bt21_004_opt_clears_next_turn() {
    let mut runner = egg_runner();
    host_with_egg(&mut runner, 0);
    let tamer = runner.place_on_field(0, "YEL-TAMER", Some(0));

    let hand_before = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(runner.game.players[0].hand.len(), hand_before + 1);

    for _ in 0..6 {
        runner.end_turn();
        let _ = runner.auto_resolve();
        if runner.game.turn_player() == 0 {
            break;
        }
    }
    assert_eq!(runner.game.turn_player(), 0, "back to P0's turn");

    let tamer = runner.game.players[0]
        .battle_area
        .iter()
        .position(|p| p.top_card().card_id(&runner.game.card_data) == "YEL-TAMER")
        .map(|i| PermanentHandle {
            player: 0,
            index: i as u8,
        })
        .expect("yellow Tamer still on field");
    let hand_mid = runner.game.players[0].hand.len();
    runner.game.suspend(tamer);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.game.players[0].hand.len(),
        hand_mid + 1,
        "OPT lockout clears on the new turn"
    );
}
