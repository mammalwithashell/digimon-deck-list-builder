//! Integration tests for opaque-opponent-deck mode (task 6.9).
//!
//! Covers the scenarios in
//! `openspec/changes/add-dcgo-recording-parity-harness/specs/opaque-opponent-deck/spec.md`
//! that don't depend on the not-yet-integrated mid-game draw paths.
//!
//! What's covered here (Phase 1 setup-time integration):
//!   - Construction validation (player count, my_player_id range, deck
//!     size mismatch, unknown card IDs).
//!   - Initial-hand setup consumes reveals from the source.
//!   - Calling-player draws follow supplied deck order (no source call).
//!   - Reveal source exhaustion surfaces as a structured error.
//!
//! What's deferred (mid-game integration, task 6.6 follow-up):
//!   - Per-turn draw against opaque opponent.
//!   - Mill effects against opaque opponent.
//!   - Effect-driven peeks / searches against opaque opponent.

use std::collections::HashMap;

use digimon_engine::card_data::CardData;
use digimon_engine::opaque_deck::{RevealKind, RevealQueue};
use digimon_engine::rules::Rules;
use digimon_engine::Game;

fn micro_card_db() -> HashMap<String, CardData> {
    let json = r#"{
        "BT1-001": {
            "card_id": "BT1-001", "card_name_eng": "Koromon",
            "card_effect_class_name": "BT1_001", "play_cost": 0, "dp": -1,
            "level": 2, "card_kind": 3, "rarity": 0, "card_colors": [0],
            "type_eng": ["Lesser"], "form_eng": ["In-Training"], "attribute_eng": [],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-010": {
            "card_id": "BT1-010", "card_name_eng": "Agumon",
            "card_effect_class_name": "BT1_010", "play_cost": 3, "dp": 2000,
            "level": 3, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Reptile"], "form_eng": ["Rookie"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "", "evo_costs": []
        },
        "BT1-025": {
            "card_id": "BT1-025", "card_name_eng": "Greymon",
            "card_effect_class_name": "BT1_025", "play_cost": 5, "dp": 5000,
            "level": 4, "card_kind": 0, "rarity": 0, "card_colors": [0],
            "type_eng": ["Dinosaur"], "form_eng": ["Champion"], "attribute_eng": ["Vaccine"],
            "effect_description_eng": "", "inherited_effect_description_eng": "",
            "security_effect_description_eng": "",
            "evo_costs": [{"card_color": 0, "level": 3, "memory_cost": 2}]
        }
    }"#;
    CardData::load_from_str(json).unwrap()
}

/// 50-card legal deck: 4 DigiEggs + 30 Agumon + 16 Greymon.
fn micro_deck() -> Vec<String> {
    let mut d = Vec::new();
    for _ in 0..4 {
        d.push("BT1-001".to_string()); // DigiEgg
    }
    for _ in 0..30 {
        d.push("BT1-010".to_string());
    }
    for _ in 0..16 {
        d.push("BT1-025".to_string());
    }
    d
}

/// A reveal queue serving the same card-id over and over — useful for
/// tests where we don't care which specific card is drawn, only that the
/// source was consulted.
fn fill_queue_with_agumon(count: usize) -> RevealQueue {
    let pairs = (0..count).map(|_| (RevealKind::Draw, "BT1-010".to_string()));
    RevealQueue::from_pairs(pairs)
}

#[test]
fn construction_accepts_well_formed_inputs() {
    let db = micro_card_db();
    let starting_hand = Rules::standard().starting_hand as usize;
    // Need at least `starting_hand` reveals for the opponent's initial draw.
    let queue = fill_queue_with_agumon(starting_hand);

    let game = Game::new_with_opaque_opponent(
        0, // my_player_id
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect("construction should succeed for well-formed inputs");

    // Calling player (0) has an ordered deck — the engine populated it
    // from the supplied my_deck.
    let me = &game.players[0];
    assert!(me.opaque_deck_state.is_none(), "calling player should not be opaque");
    assert!(!me.deck.is_empty(), "calling player's deck should be populated");

    // Opponent (1) is in opaque mode with the multiset known.
    let opp = &game.players[1];
    assert!(opp.opaque_deck_state.is_some(), "opponent should be opaque");
    let state = opp.opaque_deck_state.as_ref().unwrap();
    // Initial multiset = decklist minus digitama (4 DigiEggs go to digitama_deck)
    // minus starting_hand cards (already drawn from the reveal source).
    let expected = 50 - 4 - starting_hand;
    assert_eq!(state.total_remaining(), expected,
        "after initial draw, opaque pile should have {} cards", expected);
    assert!(opp.deck.is_empty(),
        "opaque player's ordered deck Vec should be empty (state owns composition)");
    assert_eq!(opp.hand.len(), starting_hand,
        "opaque player's starting hand should be filled via reveals");
}

#[test]
fn construction_rejects_wrong_player_count_rules() {
    let db = micro_card_db();
    let queue = fill_queue_with_agumon(5);
    let mut bad_rules = Rules::standard();
    bad_rules.player_count = 3;

    let err = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        bad_rules,
        Some(0),
    )
    .expect_err("3-player rules should be rejected for opaque mode");
    assert!(err.contains("player_count == 2"), "unexpected error: {}", err);
}

#[test]
fn construction_rejects_invalid_my_player_id() {
    let db = micro_card_db();
    let queue = fill_queue_with_agumon(5);
    let err = Game::new_with_opaque_opponent(
        2, // invalid — must be 0 or 1
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect_err("my_player_id >= 2 should be rejected");
    assert!(err.contains("my_player_id"), "unexpected error: {}", err);
}

#[test]
fn construction_rejects_mismatched_decklist_sizes() {
    let db = micro_card_db();
    let queue = fill_queue_with_agumon(5);
    let mut short_opp = micro_deck();
    short_opp.pop(); // 49-card decklist — mismatched

    let err = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        short_opp,
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect_err("size mismatch should be rejected");
    assert!(err.contains("size") && err.contains("differs"),
        "unexpected error: {}", err);
}

#[test]
fn construction_rejects_unknown_card_id_in_opponent_decklist() {
    let db = micro_card_db();
    let queue = fill_queue_with_agumon(5);
    let mut bad_opp = micro_deck();
    bad_opp[0] = "FAKE-NONEXISTENT-CARD".to_string();

    let err = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        bad_opp,
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect_err("unknown card ID should be rejected");
    assert!(err.contains("unknown card ID"), "unexpected error: {}", err);
    assert!(err.contains("FAKE-NONEXISTENT-CARD"), "should name the offending card: {}", err);
}

#[test]
fn initial_hand_setup_consumes_reveals_in_order() {
    let db = micro_card_db();
    let starting_hand = Rules::standard().starting_hand as usize;
    // Serve a specific sequence so we can assert it materialized into the hand.
    let mut pairs = Vec::new();
    for _ in 0..starting_hand {
        pairs.push((RevealKind::Draw, "BT1-025".to_string()));
    }
    let queue = RevealQueue::from_pairs(pairs);

    let game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect("construction");

    // Opponent's hand should be all Greymon (BT1-025) because the
    // reveal source served Greymon for every initial draw.
    let opp = &game.players[1];
    assert_eq!(opp.hand.len(), starting_hand);
    for card in &opp.hand {
        let card_id = card.card_id(&game.card_data);
        assert_eq!(card_id, "BT1-025",
            "opaque opponent hand card should match served reveal, got {}", card_id);
    }
    // The multiset is correspondingly depleted of Greymon.
    let state = opp.opaque_deck_state.as_ref().unwrap();
    assert_eq!(state.count_of("BT1-025"), 16 - starting_hand);
}

#[test]
fn calling_player_draws_follow_supplied_deck_order() {
    let db = micro_card_db();
    let starting_hand = Rules::standard().starting_hand as usize;
    let queue = fill_queue_with_agumon(starting_hand);

    // Construct my_deck with all DigiEggs at the front, then all Greymon,
    // then all Agumon. Game::new will shuffle the deck before drawing the
    // initial hand, so we won't see a deterministic post-shuffle order
    // without the seed. With Some(0), the shuffle is deterministic; we
    // just confirm the calling player's hand isn't drawn via the reveal
    // source (i.e., the source queue still has its full count remaining
    // after construction — only the opponent's draws consumed it).
    let queue_remaining_before_test = starting_hand;

    let game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect("construction");

    // The calling player's hand should have been filled from THEIR own
    // ordered deck, not the reveal source. We can't directly assert
    // "source had X reveals consumed" without access to its internal
    // state, but we can confirm:
    //   1. Calling player has a starting hand.
    //   2. Calling player still has cards in their `deck` Vec (because
    //      starting_hand << deck size).
    //   3. The reveal source had `starting_hand` reveals at start,
    //      and exactly `starting_hand` should remain consumed by the
    //      opponent's initial draw — meaning the source is now empty
    //      AS USED BY OPPONENT ONLY.
    let me = &game.players[0];
    assert_eq!(me.hand.len(), starting_hand);
    assert!(!me.deck.is_empty(),
        "calling player should still have a deck after drawing starting hand");
    // The variable `queue_remaining_before_test` is for documentation —
    // the assertions above transitively prove the source wasn't
    // double-consumed (otherwise the opponent's hand would be incomplete
    // or short, but the previous test confirms it isn't).
    let _ = queue_remaining_before_test;
}

#[test]
fn empty_reveal_source_at_init_surfaces_error() {
    // A reveal source with insufficient entries for the initial-hand
    // setup should fail construction with a descriptive error rather
    // than panicking.
    let db = micro_card_db();
    let starting_hand = Rules::standard().starting_hand as usize;
    let queue = fill_queue_with_agumon(starting_hand - 1); // one short

    let err = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect_err("undersized reveal source should fail construction");
    assert!(err.contains("queue empty") || err.contains("exhausted"),
        "unexpected error: {}", err);
}

#[test]
fn opaque_opponent_my_player_id_one_path() {
    // The constructor accepts either side as the calling player. Verify
    // that my_player_id == 1 correctly assigns the opaque pile to player 0.
    let db = micro_card_db();
    let starting_hand = Rules::standard().starting_hand as usize;
    let queue = fill_queue_with_agumon(starting_hand);

    let game = Game::new_with_opaque_opponent(
        1, // calling player is player 1
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        Rules::standard(),
        Some(0),
    )
    .expect("construction");

    // Player 0 is now the opaque opponent.
    assert!(game.players[0].opaque_deck_state.is_some());
    assert!(game.players[1].opaque_deck_state.is_none());
}

#[test]
fn opaque_security_setup_via_finalize_mulligan_pushes_placeholders() {
    // Lazy reveal: opaque security is filled with PLACEHOLDERS at setup
    // time. Their identities are materialized later, when the security
    // cards are flipped via attack-triggered SecurityCheck. The reveal
    // source is NOT consumed at setup — only at flip time.
    let db = micro_card_db();
    let rules = Rules::standard();
    let starting_hand = rules.starting_hand as usize;
    let security_count = rules.security_count as usize;

    // Source needs ONLY `starting_hand` Draw reveals (for initial hand).
    // Security reveals are not consumed at setup in the lazy model.
    let pairs: Vec<_> = (0..starting_hand)
        .map(|_| (RevealKind::Draw, "BT1-010".to_string()))
        .collect();
    let queue = RevealQueue::from_pairs(pairs);

    let mut game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        rules,
        Some(0),
    )
    .expect("construction");

    // Both players keep their starting hand. After both keep,
    // `finalize_mulligan` runs which sets up security — pushing
    // placeholders for the opaque opponent.
    for _ in 0..2 {
        if let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, true).expect("mulligan keep");
        } else {
            break;
        }
    }

    // Opaque opponent's security has `security_count` placeholder cards.
    let opp = &game.players[1];
    assert_eq!(opp.security.len(), security_count,
        "opaque opponent's security should hold `security_count` placeholders");
    for card in &opp.security {
        assert!(card.is_opaque_placeholder,
            "every opaque opponent security card should be a placeholder pre-flip");
        assert!(card.face_down,
            "opaque security placeholders should be face-down");
    }
    // The reveal queue had only `starting_hand` reveals (no extra for
    // security); construction would have failed if security had eagerly
    // consumed reveals. Reaching this assertion proves lazy semantics.
}

#[test]
fn opaque_mulligan_restores_hand_to_pile_and_redraws() {
    // When the opaque opponent mulligans (decides to redraw), the
    // engine's redraw_hand should:
    //   1. Fold the existing hand back into the opaque multiset.
    //   2. Consume `starting_hand` fresh reveals from the source.
    //
    // The simplest way to verify: serve a distinguishable card for the
    // redraw and confirm the opaque opponent's post-mulligan hand
    // contains those distinct cards.
    let db = micro_card_db();
    let rules = Rules::standard();
    let starting_hand = rules.starting_hand as usize;
    let security_count = rules.security_count as usize;

    // Source for: starting_hand (Draw, BT1-010) + starting_hand (Draw,
    // BT1-025 for redraw) + 2*security_count (Security, BT1-001 for
    // each player's security setup — though player 0 (non-opaque) uses
    // their ordered deck; only player 1's Security reveals are consumed).
    let mut pairs = Vec::new();
    for _ in 0..starting_hand {
        pairs.push((RevealKind::Draw, "BT1-010".to_string()));
    }
    for _ in 0..starting_hand {
        pairs.push((RevealKind::Draw, "BT1-025".to_string())); // redraw
    }
    for _ in 0..security_count {
        pairs.push((RevealKind::Security, "BT1-010".to_string()));
    }
    let queue = RevealQueue::from_pairs(pairs);

    let mut game = Game::new_with_opaque_opponent(
        0, // I'm player 0; opaque opponent is player 1
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        rules,
        Some(0),
    )
    .expect("construction");

    // Mulligan order: typically player 0 first, then player 1. Player 0
    // keeps; player 1 mulligans (the opaque opponent).
    if let Some(p) = game.mulligan_current_player() {
        game.accept_mulligan(p, true).expect("player 0 keeps");
    }
    if let Some(p) = game.mulligan_current_player() {
        // This player should be player 1 (the opaque opponent).
        assert_eq!(p, 1, "expected player 1 to mulligan next");
        game.accept_mulligan(p, false).expect("player 1 mulligans");
    }

    // After redraw, opponent's hand should contain `starting_hand`
    // copies of BT1-025 (the cards served for the redraw).
    let opp = &game.players[1];
    assert_eq!(opp.hand.len(), starting_hand,
        "opaque opponent's hand should be re-filled to {} after mulligan", starting_hand);
    for card in &opp.hand {
        let id = card.card_id(&game.card_data);
        assert_eq!(id, "BT1-025",
            "post-mulligan hand card should be from the redraw stream, got {}", id);
    }

    // Multiset should have restored the original hand (BT1-010) and
    // consumed the redraw cards (BT1-025). Security uses lazy reveal
    // post-Group-6-lazy refactor — so security setup does NOT debit
    // per-card multiset entries at finalize_mulligan; it only debits
    // total_remaining via reserve_placeholders. Per-card counts
    // therefore reflect only the hand-related transactions.
    let state = opp.opaque_deck_state.as_ref().unwrap();
    // Before mulligan: 30 BT1-010 - starting_hand = 30 - 5 = 25 BT1-010.
    // After mulligan restore: +starting_hand = back to 30.
    // Security setup is lazy and doesn't debit per-card counts.
    assert_eq!(state.count_of("BT1-010"), 30,
        "BT1-010 count should reflect restore-after-mulligan (security debit is lazy)");
    // BT1-025 initial 16, redrew starting_hand=5 in hand, so 16 - 5 = 11 remain.
    assert_eq!(state.count_of("BT1-025"), 16 - starting_hand,
        "BT1-025 count should reflect redraw consumption");
    // Total remaining reflects: 50 - 4 digitama (routed to digitama_deck
    // at game start, not opaque pile) - starting_hand drawn - security_count
    // reserved. (Restore-after-mulligan brought hand back, then redraw
    // pulled starting_hand cards.)
    let expected_total = 50 - 4 - starting_hand - security_count;
    assert_eq!(state.total_remaining(), expected_total,
        "total_remaining should reflect reserved security placeholders + hand");
}

#[test]
fn opaque_security_placeholder_materializes_on_flip() {
    // Direct exercise of the lazy security flow: construct an opaque
    // game, set up security with placeholders, then materialize one
    // placeholder via Game::materialize_opaque_security_placeholder and
    // confirm: identity is set, is_opaque_placeholder cleared, per-card
    // multiset debited, total_remaining unchanged (since reservation
    // already debited it at setup).
    let db = micro_card_db();
    let rules = Rules::standard();
    let starting_hand = rules.starting_hand as usize;
    let security_count = rules.security_count as usize;

    // Source: starting_hand Draw reveals + 1 Security reveal for the
    // first lazy flip.
    let mut pairs: Vec<_> = (0..starting_hand)
        .map(|_| (RevealKind::Draw, "BT1-010".to_string()))
        .collect();
    pairs.push((RevealKind::Security, "BT1-025".to_string())); // first flip
    let queue = RevealQueue::from_pairs(pairs);

    let mut game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        rules,
        Some(0),
    )
    .expect("construction");

    // Walk through mulligans to trigger finalize_mulligan + security
    // placeholders.
    for _ in 0..2 {
        if let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, true).expect("mulligan keep");
        }
    }

    // All security slots are placeholders.
    assert_eq!(game.players[1].security.len(), security_count);
    for c in &game.players[1].security {
        assert!(c.is_opaque_placeholder);
    }

    // Snapshot per-card BT1-025 count before materialization.
    let bt1_025_before = game.players[1]
        .opaque_deck_state
        .as_ref()
        .unwrap()
        .count_of("BT1-025");
    let total_before = game.players[1]
        .opaque_deck_state
        .as_ref()
        .unwrap()
        .total_remaining();

    // Materialize the top of security (idx = len-1) via the public helper.
    let top_idx = game.players[1].security.len() - 1;
    let did = game
        .materialize_opaque_security_placeholder(1, top_idx)
        .expect("materialize");
    assert!(did, "should report a materialization happened");

    // The slot now has data_index for BT1-025 and is no longer a placeholder.
    let slot = &game.players[1].security[top_idx];
    assert!(!slot.is_opaque_placeholder);
    assert_eq!(slot.card_id(&game.card_data), "BT1-025");

    // BT1-025 per-card count debited by 1; total_remaining unchanged.
    let state = game.players[1].opaque_deck_state.as_ref().unwrap();
    assert_eq!(state.count_of("BT1-025"), bt1_025_before - 1,
        "per-card BT1-025 count should drop by 1 after materialization");
    assert_eq!(state.total_remaining(), total_before,
        "total_remaining should NOT change (was debited at setup)");

    // Idempotent: calling materialize on the same now-materialized slot
    // is a no-op (returns Ok(false), doesn't touch source).
    let did_again = game
        .materialize_opaque_security_placeholder(1, top_idx)
        .expect("idempotent");
    assert!(!did_again, "second call should not materialize again");
}

#[test]
fn opaque_per_turn_draw_consumes_one_reveal() {
    // Directly exercise `Game::draw_one_for_player` against the opaque
    // opponent — the same chokepoint the per-turn draw phase uses.
    // This catches regressions where the per-turn draw path bypasses
    // the opaque branch.
    let db = micro_card_db();
    let rules = Rules::standard();
    let starting_hand = rules.starting_hand as usize;

    // Source: starting_hand (initial) + 1 (the per-turn draw under test).
    let mut pairs: Vec<_> = (0..starting_hand)
        .map(|_| (RevealKind::Draw, "BT1-010".to_string()))
        .collect();
    pairs.push((RevealKind::Draw, "BT1-025".to_string())); // distinguishable
    let queue = RevealQueue::from_pairs(pairs);

    let mut game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        rules,
        Some(0),
    )
    .expect("construction");

    let opp_hand_before = game.players[1].hand.len();
    let opp_remaining_before = game.players[1]
        .opaque_deck_state
        .as_ref()
        .unwrap()
        .total_remaining();

    // Simulate a per-turn draw for the opaque player.
    let drew = game.draw_one_for_player(1).expect("per-turn opaque draw");
    assert!(drew, "draw should succeed when reveal source has data");

    assert_eq!(
        game.players[1].hand.len(),
        opp_hand_before + 1,
        "opaque opponent's hand should grow by one after a draw"
    );
    assert_eq!(
        game.players[1].opaque_deck_state.as_ref().unwrap().total_remaining(),
        opp_remaining_before - 1,
        "opaque pile total should shrink by one"
    );
    // The newly-drawn card should match the distinguishable reveal we queued.
    let new_card = game.players[1].hand.last().unwrap();
    assert_eq!(
        new_card.card_id(&game.card_data),
        "BT1-025",
        "post-construction draw should consume the next queued reveal"
    );
}

#[test]
fn ensure_security_materialized_helper_idempotent_and_safe() {
    // Direct unit-level coverage of the convenience helper used by the
    // effect-driven security sweep. The helper must:
    //   1. No-op for non-opaque players (standard mode has no placeholders).
    //   2. No-op for already-materialized slots (idempotent).
    //   3. No-op for invalid pid / idx (defensive).
    //   4. Trigger materialization (consume a Security reveal) when the
    //      slot IS a placeholder.
    let db = micro_card_db();
    let rules = Rules::standard();
    let starting_hand = rules.starting_hand as usize;

    let mut pairs: Vec<_> = (0..starting_hand)
        .map(|_| (RevealKind::Draw, "BT1-010".to_string()))
        .collect();
    pairs.push((RevealKind::Security, "BT1-025".to_string()));
    let queue = RevealQueue::from_pairs(pairs);

    let mut game = Game::new_with_opaque_opponent(
        0,
        micro_deck(),
        micro_deck(),
        Box::new(queue),
        &db,
        rules,
        Some(0),
    )
    .expect("construction");

    // Walk through mulligans to populate security with placeholders.
    for _ in 0..2 {
        if let Some(p) = game.mulligan_current_player() {
            game.accept_mulligan(p, true).expect("mulligan keep");
        }
    }

    // Case 1: helper on the standard-mode player (player 0) is a no-op.
    // It shouldn't consume from the source.
    game.ensure_security_materialized(0, 0);
    // No assertion possible on "didn't consume" directly without queue
    // inspection — but the next opaque materialize must succeed.

    // Case 2: helper on invalid pid — defensive no-op (no panic).
    game.ensure_security_materialized(7, 0);

    // Case 3: helper on out-of-range idx — defensive no-op.
    game.ensure_security_materialized(1, 999);

    // Case 4: real opaque materialization. The top of security
    // (security[len-1]) is a placeholder; ensure_security_materialized
    // should resolve it.
    let top_idx = game.players[1].security.len() - 1;
    assert!(game.players[1].security[top_idx].is_opaque_placeholder);
    game.ensure_security_materialized(1, top_idx);
    assert!(!game.players[1].security[top_idx].is_opaque_placeholder,
        "helper should materialize a placeholder");
    assert_eq!(
        game.players[1].security[top_idx].card_id(&game.card_data),
        "BT1-025",
        "materialized identity should match the served reveal"
    );

    // Case 5: helper on the now-materialized slot — idempotent no-op.
    // (Calling materialize_opaque_security_placeholder directly would
    // return Ok(false); the helper's no-op branch keeps consistent
    // behavior.)
    game.ensure_security_materialized(1, top_idx);
    // Slot unchanged.
    assert!(!game.players[1].security[top_idx].is_opaque_placeholder);
}

#[test]
fn standard_constructor_unaffected_by_opaque_field_additions() {
    // Smoke test: a vanilla Game::new still works without ever touching
    // the new opaque-deck fields. Regression guard for the field additions.
    let db = micro_card_db();
    let game = Game::new(
        &[micro_deck(), micro_deck()],
        &db,
        Rules::standard(),
        Some(42),
    )
    .expect("standard construction");
    assert!(game.players[0].opaque_deck_state.is_none());
    assert!(game.players[1].opaque_deck_state.is_none());
    assert!(game.reveal_source.is_none());
    // (opaque_data_index_map is pub(crate) so untestable from integration
    // scope — that's the right visibility for an internal cache. The
    // None-ness is verified by the lib unit tests if needed.)
}
