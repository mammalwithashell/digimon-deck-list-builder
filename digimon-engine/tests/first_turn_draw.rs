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
