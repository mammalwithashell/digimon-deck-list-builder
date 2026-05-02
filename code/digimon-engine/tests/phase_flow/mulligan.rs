//! Mulligan phase state machine tests (§1.6 in RUST_PYTHON_PARITY.md).
//!
//! These exercise Game::accept_mulligan directly — not via DebugRunner —
//! because DebugRunner's build_inner clears mulligan_pending when it
//! populates explicit zones. Mulligan tests want the full draw/redraw
//! flow, so they construct Game::new with a real deck and step through
//! keep/mulligan actions manually.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::enums::GamePhase;
use digimon_engine::game::Game;
use digimon_engine::rules::Rules;

fn test_card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "EGG": {
            "card_id": "EGG", "card_name_eng": "Egg",
            "card_effect_class_name": "", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "DIG": {
            "card_id": "DIG", "card_name_eng": "Agumon",
            "card_effect_class_name": "", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "CHA": {
            "card_id": "CHA", "card_name_eng": "Greymon",
            "card_effect_class_name": "", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": [], "form_eng": [], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

/// A deck large enough that redraws still have cards to pull from.
/// 4 eggs + 30 main (20 Agumon + 10 Greymon) = 34 cards per player.
fn test_deck() -> Vec<String> {
    let mut deck = Vec::with_capacity(34);
    for _ in 0..4 {
        deck.push("EGG".to_string());
    }
    for _ in 0..20 {
        deck.push("DIG".to_string());
    }
    for _ in 0..10 {
        deck.push("CHA".to_string());
    }
    deck
}

fn fresh_game(seed: u64) -> Game {
    let db = test_card_db();
    let deck = test_deck();
    Game::new(&[deck.clone(), deck], &db, Rules::standard(), Some(seed)).unwrap()
}

#[test]
fn game_starts_in_mulligan_phase_with_first_player_pending() {
    let game = fresh_game(42);
    assert_eq!(game.current_phase, GamePhase::Mulligan);
    assert_eq!(game.turn_count, 0);
    let pending = game
        .mulligan_current_player()
        .expect("mulligan should be pending");
    assert_eq!(
        pending, game.turn_order[0],
        "first decider is turn_order[0]"
    );
    assert_eq!(game.mulligan_pending.len(), 2);
    assert_eq!(game.mulligan_used, vec![false, false]);
    // Opening hands drawn; security not yet laid.
    assert_eq!(game.player(0).hand.len(), 5);
    assert_eq!(game.player(1).hand.len(), 5);
    assert_eq!(game.player(0).security.len(), 0);
    assert_eq!(game.player(1).security.len(), 0);
}

#[test]
fn keep_decision_advances_to_next_player() {
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    game.accept_mulligan(first, true).unwrap();

    let second = game.mulligan_current_player().expect("one still pending");
    assert_ne!(second, first);
    assert_eq!(
        game.mulligan_used[first as usize], false,
        "keep does not mark used"
    );
}

#[test]
fn mulligan_redraws_hand_to_starting_size() {
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    let hand_before = game.player(first).hand.len();
    let deck_before = game.player(first).deck.len();

    game.accept_mulligan(first, false).unwrap();

    assert_eq!(
        game.player(first).hand.len(),
        hand_before,
        "hand stays at starting size"
    );
    assert_eq!(
        game.player(first).deck.len(),
        deck_before,
        "deck size unchanged (hand went back, equal count came out)"
    );
    assert!(game.mulligan_used[first as usize], "used flag set");
    assert_ne!(
        game.mulligan_current_player(),
        Some(first),
        "decider advanced"
    );
}

#[test]
fn mulligan_redraw_can_produce_different_hand_than_kept() {
    // Capture the hand. Mulligan. Compare ordering of card data indices —
    // with a 34-card deck and seed 42, the reshuffle should reorder things.
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    let before: Vec<usize> = game
        .player(first)
        .hand
        .iter()
        .map(|c| c.data_index)
        .collect();

    game.accept_mulligan(first, false).unwrap();

    let after: Vec<usize> = game
        .player(first)
        .hand
        .iter()
        .map(|c| c.data_index)
        .collect();

    assert_eq!(before.len(), after.len(), "same hand size");
    // With enough cards in the deck and this seed, the shuffle should
    // produce at least one difference in order. (Not bulletproof against
    // pathological seeds, but fine in practice.)
    assert_ne!(before, after, "redraw should produce a different ordering");
}

#[test]
fn all_players_keep_finalizes_and_starts_turn_1() {
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    game.accept_mulligan(first, true).unwrap();
    let second = game.mulligan_current_player().unwrap();
    game.accept_mulligan(second, true).unwrap();

    assert!(game.mulligan_current_player().is_none());
    assert_eq!(game.turn_count, 1);
    assert_eq!(game.memory, 0);
    // Security laid to full count for every player.
    for pid in 0..game.rules.player_count {
        assert_eq!(
            game.player(pid).security.len(),
            game.rules.security_count as usize,
            "player {} should have full security",
            pid
        );
    }
    // begin_turn has advanced us into Breeding (the first player's turn-1
    // draw is skipped, so we pass through Draw immediately).
    assert_eq!(game.current_phase, GamePhase::Breeding);
}

#[test]
fn security_is_laid_only_after_mulligan_completes() {
    let mut game = fresh_game(42);
    // Before any decision: no security.
    assert_eq!(game.player(0).security.len(), 0);
    assert_eq!(game.player(1).security.len(), 0);

    let first = game.mulligan_current_player().unwrap();
    game.accept_mulligan(first, true).unwrap();

    // After first decision but before second: still no security.
    assert_eq!(game.player(0).security.len(), 0);
    assert_eq!(game.player(1).security.len(), 0);

    let second = game.mulligan_current_player().unwrap();
    game.accept_mulligan(second, true).unwrap();

    // After finalize: full security for both.
    assert_eq!(game.player(0).security.len(), 5);
    assert_eq!(game.player(1).security.len(), 5);
}

#[test]
fn accept_mulligan_rejects_out_of_order_player() {
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    let other = if first == 0 { 1 } else { 0 };
    let err = game
        .accept_mulligan(other, true)
        .expect_err("out-of-order player should fail");
    assert!(err.contains("different player"));
    // State unchanged.
    assert_eq!(game.mulligan_current_player(), Some(first));
}

#[test]
fn accept_mulligan_rejects_after_completion() {
    let mut game = fresh_game(42);
    let first = game.mulligan_current_player().unwrap();
    game.accept_mulligan(first, true).unwrap();
    let second = game.mulligan_current_player().unwrap();
    game.accept_mulligan(second, true).unwrap();

    // A third decision must be rejected.
    let err = game
        .accept_mulligan(0, true)
        .expect_err("should reject after done");
    assert!(err.contains("already complete"));
}

#[test]
fn start_game_auto_keeps_all_pending() {
    // Regression: `.start_game()` must skip the mulligan for any caller that
    // doesn't care, matching pre-mulligan Rust behavior.
    let mut game = fresh_game(42);
    assert!(game.mulligan_current_player().is_some());

    game.start_game();

    assert!(game.mulligan_current_player().is_none());
    assert_eq!(game.turn_count, 1);
    assert_eq!(game.current_phase, GamePhase::Breeding);
    assert_eq!(game.player(0).security.len(), 5);
    assert_eq!(game.player(1).security.len(), 5);
}

#[test]
fn coin_flip_respects_seed() {
    let game_a = fresh_game(42);
    let game_b = fresh_game(42);
    assert_eq!(
        game_a.turn_order, game_b.turn_order,
        "same seed → same turn_order"
    );

    // Different seeds should — with high probability — differ somewhere
    // across a handful of tries. We loop until we see the other order,
    // bounded to keep the test deterministic.
    let mut seen_different = false;
    for seed in 1..200u64 {
        let g = fresh_game(seed);
        if g.turn_order != game_a.turn_order {
            seen_different = true;
            break;
        }
    }
    assert!(
        seen_different,
        "expected some other seed to produce a different coin flip"
    );
}

#[test]
fn multiplayer_all_players_get_mulligan_chance() {
    let db = test_card_db();
    let deck = test_deck();
    let rules = Rules::edh(); // 4-player variant

    let mut game = Game::new(
        &[deck.clone(), deck.clone(), deck.clone(), deck],
        &db,
        rules,
        Some(42),
    )
    .unwrap();

    assert_eq!(game.mulligan_pending.len(), 4);

    let mut deciders = Vec::new();
    while let Some(p) = game.mulligan_current_player() {
        deciders.push(p);
        game.accept_mulligan(p, true).unwrap();
    }

    assert_eq!(deciders.len(), 4, "every player gets a chance");
    let mut sorted = deciders.clone();
    sorted.sort();
    assert_eq!(sorted, vec![0, 1, 2, 3], "all four distinct players");
    // Order matches turn_order (FIFO).
    assert_eq!(deciders, game.turn_order);
}
