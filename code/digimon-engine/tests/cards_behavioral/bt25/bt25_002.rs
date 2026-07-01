//! BT25-002 Wanyamon.
//!
//! Printed text (per data/cards.json + image):
//!   Inherited Effect:
//!   [Your Turn] [Once Per Turn] When any of your [DATA SQUAD] trait Tamers are
//!   played, both players <Draw 1>.
//!
//! DCGO ref: DCGO/Assets/Scripts/CardEffect/BT25/Blue/BT25_002.cs
//!   - EffectTiming.OnEnterFieldAnyone, inherited, hash BT25_002_Inherited,
//!     maxCountPerTurn 1, OPT, owner-turn gated, played permanent must be an
//!     owner battle-area Tamer with the DATA SQUAD trait. Draw loops over
//!     Players_ForTurnPlayer (turn player first).
//!
//! Pattern rows (RUST_DSL_TEST_API §4.3): inherited triggered, on_ally_played
//!   trigger w/ event predicate, OPT lockout, both-players draw.

use digimon_dsl::compiled::{CompiledClause, CompiledScope, CompiledTiming};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

fn ds_tamer(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.traits = vec!["DATA SQUAD".to_string()];
    card
}

fn plain_tamer(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![CardColor::Blue];
    card.play_cost = 3;
    card.traits = vec!["Olympos XII".to_string()];
    card
}

fn carrier(card_id: &str, name: &str) -> CardData {
    let mut card = make_test_card(card_id, name);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Blue];
    card.level = Some(3);
    card.dp = Some(2000);
    card.play_cost = 3;
    card.traits = vec!["DATA SQUAD".to_string()];
    card
}

/// Wanyamon (BT25-002) as the inherited bottom source under a Digimon on
/// player 0's field, both players have a stocked deck, player 0 has the named
/// Tamer in hand. Memory high so plays don't stall.
fn runner_with_tamer_in_hand(tamer_id: &str) -> DebugRunner {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-002")
        .expect("BT25-002 YAML loads")
        .add_card(carrier("CARRIER", "Gaomon"))
        .add_card(ds_tamer("DS-TAMER", "Thomas H. Norstein"))
        .add_card(plain_tamer("PLAIN-TAMER", "Sora Takenouchi"))
        .add_card(make_test_card("MY-DRAW-1", "My Draw 1"))
        .add_card(make_test_card("MY-DRAW-2", "My Draw 2"))
        .add_card(make_test_card("OPP-DRAW-1", "Opp Draw 1"))
        .add_card(make_test_card("OPP-DRAW-2", "Opp Draw 2"))
        .hand(0, &[tamer_id])
        .deck(0, &["MY-DRAW-1", "MY-DRAW-2"])
        .deck(1, &["OPP-DRAW-1", "OPP-DRAW-2"])
        .memory(10)
        .start();
    runner.place_stack(0, &["BT25-002", "CARRIER"]);
    runner
}

// ---------------------------------------------------------------------------
// Section 1 — structural
// ---------------------------------------------------------------------------

#[test]
fn bt25_002_structural_one_inherited_opt_played_trigger() {
    let runner = runner_with_tamer_in_hand("DS-TAMER");
    let card = runner.compiled_card("BT25-002").expect("BT25-002 present");

    assert_eq!(card.name, "Wanyamon");
    assert_eq!(card.level, Some(2));

    let triggered: Vec<&_> = card
        .effects
        .iter()
        .filter_map(|clause| match clause {
            CompiledClause::Triggered(t) => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(triggered.len(), 1, "single inherited played-Tamer clause");
    let clause = triggered[0];
    assert_eq!(clause.scope, CompiledScope::Inherited);
    assert!(clause.when.contains(&CompiledTiming::OnAllyPlayed));
    assert!(clause.once_per_turn, "[Once Per Turn]");
}

// ---------------------------------------------------------------------------
// Section 2 — condition gating (positive + negative)
// ---------------------------------------------------------------------------

#[test]
fn bt25_002_positive_data_squad_tamer_played_draws_both_players() {
    let mut runner = runner_with_tamer_in_hand("DS-TAMER");
    let my_deck_before = runner.deck_size(0);
    let opp_deck_before = runner.deck_size(1);
    let my_hand_before = runner.hand_size(0); // includes the Tamer about to leave hand

    runner.play(0, 0).expect("play DATA SQUAD Tamer");
    runner.auto_resolve().expect("resolve both-players draw");

    assert_eq!(runner.deck_size(0), my_deck_before - 1, "you draw 1");
    assert_eq!(runner.deck_size(1), opp_deck_before - 1, "opponent draws 1");
    // Net hand for player 0: -1 (played Tamer) +1 (draw) = unchanged.
    assert_eq!(runner.hand_size(0), my_hand_before);
}

#[test]
fn bt25_002_negative_non_data_squad_tamer_played_no_draw() {
    let mut runner = runner_with_tamer_in_hand("PLAIN-TAMER");
    let my_deck_before = runner.deck_size(0);
    let opp_deck_before = runner.deck_size(1);

    runner.play(0, 0).expect("play non-DATA-SQUAD Tamer");
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        my_deck_before,
        "no draw for a non-DATA-SQUAD Tamer"
    );
    assert_eq!(runner.deck_size(1), opp_deck_before);
}

#[test]
fn bt25_002_negative_opponent_turn_no_draw() {
    let mut runner = runner_with_tamer_in_hand("DS-TAMER");
    runner.end_turn(); // now player 1's turn
    assert_eq!(runner.turn_player(), 1);

    let my_deck_before = runner.deck_size(0);
    // Player 1 plays a DATA SQUAD Tamer of their own; player 0's inherited
    // [Your Turn] effect must NOT fire on the opponent's turn.
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DS-TAMER")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[1]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, 1, next,
        ));
    let p1_idx = runner.game.players[1].hand.len() - 1;

    let _ = runner.play(1, p1_idx);
    let _ = runner.auto_resolve();

    assert_eq!(
        runner.deck_size(0),
        my_deck_before,
        "player 0 inherited [Your Turn] effect must not fire on opponent's turn"
    );
}

// ---------------------------------------------------------------------------
// Section 5 — OPT lockout
// ---------------------------------------------------------------------------

#[test]
fn bt25_002_opt_second_tamer_same_turn_no_second_draw() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT25-002")
        .expect("BT25-002 YAML loads")
        .add_card(carrier("CARRIER", "Gaomon"))
        .add_card(ds_tamer("DS-TAMER-A", "Thomas H. Norstein"))
        .add_card(ds_tamer("DS-TAMER-B", "Marcus Damon"))
        .add_card(make_test_card("MY-DRAW-1", "My Draw 1"))
        .add_card(make_test_card("MY-DRAW-2", "My Draw 2"))
        .add_card(make_test_card("MY-DRAW-3", "My Draw 3"))
        .add_card(make_test_card("OPP-DRAW-1", "Opp Draw 1"))
        .add_card(make_test_card("OPP-DRAW-2", "Opp Draw 2"))
        .add_card(make_test_card("OPP-DRAW-3", "Opp Draw 3"))
        .hand(0, &["DS-TAMER-A", "DS-TAMER-B"])
        .deck(0, &["MY-DRAW-1", "MY-DRAW-2", "MY-DRAW-3"])
        .deck(1, &["OPP-DRAW-1", "OPP-DRAW-2", "OPP-DRAW-3"])
        .memory(10)
        .start();
    runner.place_stack(0, &["BT25-002", "CARRIER"]);

    let my_deck_before = runner.deck_size(0);

    runner.play(0, 0).expect("play first DATA SQUAD Tamer");
    runner.auto_resolve().expect("first draw resolves");
    assert_eq!(runner.deck_size(0), my_deck_before - 1, "first draw fires");

    // Second DATA SQUAD Tamer this turn — OPT lockout, no further draw.
    runner.play(0, 0).expect("play second DATA SQUAD Tamer");
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.deck_size(0),
        my_deck_before - 1,
        "OPT: second Tamer this turn does not draw again"
    );

    // Lockout clears after end-of-turn round-trip.
    runner.end_turn(); // player 1
    runner.end_turn(); // back to player 0
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "DS-TAMER-A")
        .unwrap();
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .hand
        .push(digimon_engine::card_source::CardSource::new(
            data_idx, 0, next,
        ));
    let idx = runner.game.players[0].hand.len() - 1;
    let deck_now = runner.deck_size(0);
    let _ = runner.play(0, idx);
    let _ = runner.auto_resolve();
    assert_eq!(
        runner.deck_size(0),
        deck_now - 1,
        "OPT lockout cleared after end_turn"
    );
}
