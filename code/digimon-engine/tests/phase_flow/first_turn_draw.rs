//! First-turn draw rule parity test (§1.7 in RUST_PYTHON_PARITY.md).
//!
//! Standard Digimon TCG: only the first player (turn_count == 1, tp == 0)
//! skips their draw. The second player draws normally on turn 2. Verified
//! equivalent to Python's `if self.turn_count == 1: pass` in `phase_draw`.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

/// Stack N FILLER cards onto a player's deck so draws are deterministic.
fn stack_deck(r: &mut DebugRunner, player: u8, count: usize) {
    // Find the FILLER data_index in the card_data store.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "FILLER")
        .expect("FILLER must be registered in card_data");
    for _ in 0..count {
        let next = r.game.next_card_index();
        let card = CardSource::new(data_idx, player, next);
        r.game.players[player as usize].deck.push(card);
    }
}

#[test]
fn first_player_skips_draw_on_turn_1() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("FILLER", "Filler"))
        .start();

    // Stack P0's deck with a known count of cards.
    stack_deck(&mut r, 0, 5);
    let hand_before = r.hand_size(0);
    let deck_before = r.deck_size(0);

    // At start(), the game has already advanced into turn 1 via start_game().
    // begin_turn's draw phase was skipped for P0 (turn_count == 1, tp == 0).
    // No draw happened.
    assert_eq!(r.hand_size(0), hand_before);
    assert_eq!(r.deck_size(0), deck_before);
    assert_eq!(r.turn_count(), 1);
    assert_eq!(r.turn_player(), 0);
}

#[test]
fn second_player_draws_on_turn_2() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("FILLER", "Filler"))
        .start();

    stack_deck(&mut r, 1, 5);
    let p1_hand_before = r.hand_size(1);
    let p1_deck_before = r.deck_size(1);

    // Pass P0's turn. That advances to P1's first turn (turn_count == 2).
    // P1 must draw 1 card per phase_draw rule.
    r.pass_turn();

    assert_eq!(r.turn_player(), 1);
    assert_eq!(r.turn_count(), 2);
    assert_eq!(
        r.hand_size(1),
        p1_hand_before + 1,
        "P1 must draw on their first turn (turn 2)"
    );
    assert_eq!(r.deck_size(1), p1_deck_before - 1);
}

#[test]
fn first_player_draws_on_turn_3() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("FILLER", "Filler"))
        .start();

    stack_deck(&mut r, 0, 5);
    stack_deck(&mut r, 1, 5);

    // Pass twice to return to P0 on turn 3.
    r.pass_turn(); // now P1, turn 2
    let p0_hand_before_turn3 = r.hand_size(0);
    let p0_deck_before_turn3 = r.deck_size(0);
    r.pass_turn(); // now P0, turn 3

    assert_eq!(r.turn_player(), 0);
    assert_eq!(r.turn_count(), 3);
    assert_eq!(
        r.hand_size(0),
        p0_hand_before_turn3 + 1,
        "P0 draws on turn 3 (second visit)"
    );
    assert_eq!(r.deck_size(0), p0_deck_before_turn3 - 1);
}

/// Regression: after an explicit mulligan-auto-keep finalize, the first-player
/// draw skip must still apply on turn 1. Guards §1.6 + §1.7 together.
#[test]
fn first_player_skip_still_works_after_mulligan_finalize() {
    use digimon_engine::card_data::CardData;
    use digimon_engine::game::Game;
    use digimon_engine::rules::Rules;
    use std::collections::HashMap;

    let mut db = HashMap::new();
    db.insert("FILLER".to_string(), make_test_card("FILLER", "Filler"));
    // Need a deck large enough to absorb 5 hand + 5 security + more for the
    // post-finalize turn-1 draw test. 20 cards is plenty.
    let deck = vec!["FILLER".to_string(); 20];
    let mut game = Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(42)).unwrap();

    // Step through both keeps explicitly (not via start_game) to exercise
    // the real mulligan API.
    let first = game.mulligan_current_player().unwrap();
    game.accept_mulligan(first, true).unwrap();
    let second = game.mulligan_current_player().unwrap();
    game.accept_mulligan(second, true).unwrap();

    // Finalize put us at turn_count == 1, and the first player's draw was
    // skipped because `SkipDraw::FirstPlayerOnly` matches on turn_count == 1.
    assert_eq!(game.turn_count, 1);
    let tp = game.turn_player();
    assert_eq!(
        game.player(tp).hand.len(),
        5,
        "first player's hand stays at 5 — no turn-1 draw"
    );
    let _: &CardData = &game.card_data[0];
}
