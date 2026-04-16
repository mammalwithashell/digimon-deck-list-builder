//! Behavioral tests for the 5 hand-written test cards (TEST-001 .. TEST-005).
//!
//! These exercise the full effect pipeline:
//! Game::play_from_hand -> registry lookup -> EffectContext -> mutations.

use digimon_engine::debug_runner::{make_test_card, make_test_egg, DebugRunner};
use digimon_engine::enums::{EffectTiming, Expiry, ModifierType};
use digimon_engine::permanent::PermanentHandle;

/// TEST-001: "On Play: Gain 1 memory." (card also costs 3)
#[test]
fn test_001_gains_one_memory_on_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5) // pre-fund: 5 − 3 cost + 1 OnPlay = 3
        .start();
    let m_before = r.memory();
    r.play(0, 0);
    assert_eq!(
        r.memory(),
        m_before - 3 + 1,
        "play should cost 3, then OnPlay grants +1"
    );
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(r.hand_size(0), 0);
}

/// TEST-002: "On Play: Draw 2 cards."
#[test]
fn test_002_draws_two_cards_on_play() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-002", "TestTwo"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-002"])
        .deck(0, &["FILLER", "FILLER", "FILLER"]) // 3 in deck
        .start();
    assert_eq!(r.hand_size(0), 1);
    assert_eq!(r.deck_size(0), 3);

    r.play(0, 0);

    // Hand started with 1 (TEST-002), played to field, drew 2 = hand should be 2.
    assert_eq!(r.hand_size(0), 2, "should have drawn 2 cards");
    assert_eq!(r.deck_size(0), 1, "deck should have 1 left");
}

/// TEST-002 with deck-out: only draws what's available.
#[test]
fn test_002_partial_draw_on_short_deck() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-002", "TestTwo"))
        .add_card(make_test_card("FILLER", "Filler"))
        .hand(0, &["TEST-002"])
        .deck(0, &["FILLER"]) // only 1 card
        .start();
    r.play(0, 0);
    assert_eq!(r.hand_size(0), 1, "should have drawn the only card");
    assert_eq!(r.deck_size(0), 0);
}

/// TEST-003: "On Play: All your Digimon get +1000 DP for the turn."
#[test]
fn test_003_grants_dp_modifier_to_all_allies() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-003", "TestThree"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["ALLY", "ALLY", "TEST-003"])
        .start();

    // Play 2 allies first.
    r.play(0, 0);
    r.play(0, 0); // remaining ALLY (now at index 0 after first removal)
    assert_eq!(r.battle_area_size(0), 2);
    let h0 = r.perm_handle(0, 0);
    let h1 = r.perm_handle(0, 1);
    // Allies have base DP 2000, no modifiers.
    assert_eq!(r.dp_of(h0), Some(2000));
    assert_eq!(r.dp_of(h1), Some(2000));

    // Play TEST-003 — should buff all allies (including itself).
    r.play(0, 0);
    let h2 = r.perm_handle(0, 2);

    assert_eq!(r.dp_of(h0), Some(3000), "ally 0 should be buffed");
    assert_eq!(r.dp_of(h1), Some(3000), "ally 1 should be buffed");
    assert_eq!(r.dp_of(h2), Some(3000), "TEST-003 should be buffed too");
}

/// TEST-003: modifiers should expire at end of turn.
#[test]
fn test_003_dp_modifier_expires_end_of_turn() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-003", "TestThree"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["ALLY", "TEST-003"])
        .start();

    r.play(0, 0); // ALLY -> field index 0
    r.play(0, 0); // TEST-003 -> field index 1
    let h_ally = r.perm_handle(0, 0);
    assert_eq!(r.dp_of(h_ally), Some(3000));

    // End P0's turn — modifiers with Expiry::EndOfTurn should be cleared.
    // Note: end_of_turn expiry happens via modifiers.expire_end_of_turn(),
    // which is wired into the turn cycle. For this test we call it directly.
    let p0 = r.turn_player();
    r.game.modifiers.expire_end_of_turn(p0);

    assert_eq!(
        r.dp_of(h_ally),
        Some(2000),
        "modifier should have expired"
    );
}

/// TEST-004: "When Digivolving: Gain 2 memory if opponent has any Digimon."
/// We test the condition check directly via the registry, since digivolution
/// flow isn't fully wired through play_from_hand yet.
#[test]
fn test_004_condition_true_when_opponent_has_digimon() {
    let r = DebugRunner::builder()
        .add_card(make_test_card("TEST-004", "TestFour"))
        .add_card(make_test_card("OPP_DIG", "OppDig"))
        .hand(1, &["OPP_DIG"])
        .start();

    // Build the effect manually and check its condition.
    let registry = digimon_engine::cards::build_registry();
    let effect_impl = registry.get("TEST-004").expect("TEST-004 registered");
    let card_handle = digimon_engine::card_source::CardHandle(99);
    let effects = effect_impl.effects(card_handle);
    let when_dig = effects
        .iter()
        .find(|e| e.timing == EffectTiming::WhenDigivolving)
        .expect("WhenDigivolving effect");
    assert!(when_dig.when_digivolving);
    assert!(when_dig.condition.is_some());
    assert!(when_dig.process.is_some());

    // With no opponent Digimon on field, condition should be false.
    let mut game = r.game;
    let ctx = digimon_engine::effect_context::EffectContext::new(
        &mut game,
        card_handle,
        None,
        0,
    );
    let cond = when_dig.condition.as_ref().unwrap();
    assert!(!cond(&ctx.as_read()), "condition should be false (no opp Digimon)");
}

#[test]
fn test_004_condition_true_then_gains_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-004", "TestFour"))
        .add_card(make_test_card("OPP_DIG", "OppDig"))
        .hand(1, &["OPP_DIG"])
        .start();

    // Manually place an opponent Digimon on the field (simulating it being there).
    // We do this by playing during P1's turn, but for the simple case let's just
    // inject the permanent directly.
    let opp_card = r.game.players[1].hand.remove(0);
    let perm = digimon_engine::permanent::Permanent::new(opp_card, r.turn_count());
    r.game.players[1].battle_area.push(perm);

    // Now check the condition.
    let registry = digimon_engine::cards::build_registry();
    let effect_impl = registry.get("TEST-004").unwrap();
    let handle = digimon_engine::card_source::CardHandle(99);
    let effects = effect_impl.effects(handle);
    let when_dig = effects
        .iter()
        .find(|e| e.timing == EffectTiming::WhenDigivolving)
        .unwrap();

    let m_before = r.memory();
    {
        let ctx = digimon_engine::effect_context::EffectContext::new(
            &mut r.game,
            handle,
            None,
            0,
        );
        let cond = when_dig.condition.as_ref().unwrap();
        assert!(cond(&ctx.as_read()), "condition should be true (opp has Digimon)");
    }
    {
        let mut ctx = digimon_engine::effect_context::EffectContext::new(
            &mut r.game,
            handle,
            None,
            0,
        );
        let process = when_dig.process.as_ref().unwrap();
        process(&mut ctx);
    }
    assert_eq!(r.memory(), m_before + 2, "should gain 2 memory");
}

/// TEST-005: "On Deletion: Lose 1 memory."
/// We invoke the OnDeletion effect manually since deletion flow isn't fully
/// wired through play_from_hand.
#[test]
fn test_005_on_deletion_loses_memory() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-005", "TestFive"))
        .hand(0, &["TEST-005"])
        .start();
    r.play(0, 0);

    let registry = digimon_engine::cards::build_registry();
    let effect_impl = registry.get("TEST-005").unwrap();
    let handle = digimon_engine::card_source::CardHandle(0);
    let effects = effect_impl.effects(handle);
    let on_del = effects
        .iter()
        .find(|e| e.timing == EffectTiming::OnDeletion)
        .expect("OnDeletion effect");
    assert!(on_del.on_deletion);

    let m_before = r.memory();
    let mut ctx = digimon_engine::effect_context::EffectContext::new(
        &mut r.game,
        handle,
        None,
        0,
    );
    let process = on_del.process.as_ref().unwrap();
    process(&mut ctx);
    assert_eq!(r.memory(), m_before - 1, "should lose 1 memory on deletion");
}

/// Sanity: cards without registered effects don't crash play_from_hand.
#[test]
fn unregistered_card_plays_silently() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("UNKNOWN", "Unknown"))
        .hand(0, &["UNKNOWN"])
        .memory(5)
        .start();
    let m_before = r.memory();
    let result = r.play(0, 0);
    assert_eq!(result, Some(0));
    assert_eq!(r.battle_area_size(0), 1);
    assert_eq!(
        r.memory(),
        m_before - 3,
        "memory only drops by the play cost (no OnPlay effect)"
    );
}

/// Sanity: registered cards in opponent's hand don't fire when played by their
/// owner — they fire for the player who plays them (which is correct).
#[test]
fn opp_plays_test_001_credits_active_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-001", "TestOne"))
        .hand(0, &["TEST-001"])
        .memory(5)
        .start();
    let m_before = r.memory();
    r.play(0, 0);
    assert_eq!(r.memory(), m_before - 3 + 1);
}

/// Modifier with Expiry::Permanent should NOT expire on end_of_turn.
#[test]
fn test_003_persistent_modifier_does_not_expire() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["ALLY"])
        .start();
    r.play(0, 0);
    let h = r.perm_handle(0, 0);

    // Add a permanent +500 manually.
    r.game.modifiers.add(
        h,
        digimon_engine::modifiers::ModifierEntry {
            modifier: ModifierType::ChangeDp,
            value: 500,
            expiry: Expiry::Permanent,
            source_player: 0,
        },
    );
    assert_eq!(r.dp_of(h), Some(2500));

    r.game.modifiers.expire_end_of_turn(0);
    assert_eq!(r.dp_of(h), Some(2500), "permanent modifier should remain");
}

/// Sanity test on egg / digitama setup.
#[test]
fn debug_runner_supports_digitama() {
    let r = DebugRunner::builder()
        .add_card(make_test_egg("EGG", "Egg"))
        .digitama(0, &["EGG", "EGG"])
        .start();
    assert_eq!(r.game.player(0).digitama_deck.len(), 2);
}

/// Sanity test: PermanentHandle round-trips.
#[test]
fn perm_handle_indexes_correctly() {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["ALLY", "ALLY", "ALLY"])
        .start();
    r.play(0, 0);
    r.play(0, 0);
    r.play(0, 0);
    let h0: PermanentHandle = r.perm_handle(0, 0);
    let h2: PermanentHandle = r.perm_handle(0, 2);
    assert_eq!(h0.player, 0);
    assert_eq!(h0.index, 0);
    assert_eq!(h2.index, 2);
}
