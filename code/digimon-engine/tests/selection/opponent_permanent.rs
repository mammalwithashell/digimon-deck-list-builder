//! Pilot selection subsystem tests — exercises `select_opponent_permanent`
//! end-to-end via TEST-010: "On Play: Delete 1 of your opponent's Digimon
//! (Optional)".
//!
//! Covers:
//! - OnPlay installs a PendingSelection (SelectTarget phase, OppField kind,
//!   valid_action_ids sized to the number of opponent Digimon).
//! - Action mask renders only those action IDs for the selecting player,
//!   plus PASS because the effect is optional.
//! - Non-selecting player sees an all-zero mask.
//! - Resolving with a valid action ID fires the callback and deletes the
//!   chosen target.
//! - PASS (decline) leaves the board unchanged.
//! - Wrong player / invalid action / no-pending-selection errors.
//! - Tensor writes valid_count + selecting_player slots.

use digimon_engine::action::build_action_mask;
use digimon_engine::action::space::{encode_attack, ACTION_SPACE_SIZE, CONCEDE_GAME, PASS};
use digimon_engine::card_registry::CardRegistry;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::GamePhase;
use digimon_engine::selection::{SelectionError, SelectionKind};
use digimon_engine::tensor_v1::build_tensor_standard_compact_v1;

fn runner_with_opponent_field(opp_count: usize) -> DebugRunner {
    let mut r = DebugRunner::builder()
        .add_card(make_test_card("TEST-010", "PilotDelete"))
        .add_card(make_test_card("ALLY", "Ally"))
        .hand(0, &["TEST-010"])
        .memory(3) // pre-fund play cost
        .start();
    for _ in 0..opp_count {
        r.place_on_field(1, "ALLY", Some(0));
    }
    r
}

#[test]
fn play_installs_pending_selection_with_target_bits() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    let sel = r
        .game
        .pending_selection
        .as_ref()
        .expect("TEST-010 must install a PendingSelection on play");
    assert_eq!(sel.kind, SelectionKind::OppField);
    assert_eq!(sel.selecting_player, 0);
    assert!(sel.is_optional, "TEST-010 is tagged optional");
    assert_eq!(
        sel.valid_action_ids,
        vec![encode_attack(0, 0), encode_attack(0, 1)],
        "both opponent Digimon must appear in valid_action_ids"
    );
    assert_eq!(r.current_phase(), GamePhase::SelectTarget);
}

#[test]
fn mask_emits_only_valid_targets_plus_pass() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    let mask = build_action_mask(&r.game, 0);
    let legal: Vec<usize> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    let expected_target_0 = encode_attack(0, 0) as usize;
    let expected_target_1 = encode_attack(0, 1) as usize;
    assert!(legal.contains(&expected_target_0));
    assert!(legal.contains(&expected_target_1));
    assert!(
        legal.contains(&(PASS as usize)),
        "optional selection must expose PASS"
    );
    assert!(
        !legal.contains(&(CONCEDE_GAME as usize)),
        "concede is disabled in the action mask in all formats (2026-06-19)"
    );
    // Exactly those three (2 targets + PASS); concede is no longer exposed.
    assert_eq!(legal.len(), 3, "got legal bits: {:?}", legal);
}

#[test]
fn mask_is_all_zero_for_non_selecting_player() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    let mask = build_action_mask(&r.game, 1);
    let legal: Vec<usize> = mask
        .iter()
        .enumerate()
        .filter_map(|(i, &v)| (v > 0.0).then_some(i))
        .collect();
    assert!(
        legal.is_empty(),
        "non-selecting player must see an empty mask, got {:?}",
        legal
    );
}

#[test]
fn resolving_deletes_chosen_permanent() {
    let mut r = runner_with_opponent_field(3);
    r.play(0, 0);

    // Pick opponent field slot 1.
    let action = encode_attack(0, 1);
    r.game
        .resolve_selection(0, action)
        .expect("valid selection resolves");

    assert!(
        r.game.pending_selection.is_none(),
        "selection cleared after resolution"
    );
    assert_eq!(r.current_phase(), GamePhase::Main, "phase restored");
    assert_eq!(
        r.battle_area_size(1),
        2,
        "exactly one opponent Digimon deleted"
    );
}

#[test]
fn pass_declines_and_leaves_field_intact() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    r.game
        .resolve_selection(0, PASS)
        .expect("PASS is legal on optional selections");

    assert!(r.game.pending_selection.is_none());
    assert_eq!(r.current_phase(), GamePhase::Main);
    assert_eq!(
        r.battle_area_size(1),
        2,
        "declining leaves both opponent Digimon alive"
    );
}

#[test]
fn invalid_action_rejected_without_clearing_selection() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    // An action ID outside the valid set — e.g. target slot 5 (empty).
    let bogus = encode_attack(0, 5);
    let err = r
        .game
        .resolve_selection(0, bogus)
        .expect_err("bogus action rejected");
    assert_eq!(err, SelectionError::InvalidAction);
    assert!(
        r.game.pending_selection.is_some(),
        "rejected action must leave the selection installed"
    );
}

#[test]
fn wrong_player_rejected() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    let err = r
        .game
        .resolve_selection(1, encode_attack(0, 0))
        .expect_err("opponent may not answer prompt on controller's behalf");
    assert_eq!(err, SelectionError::WrongPlayer);
}

#[test]
fn resolve_without_pending_returns_none_pending() {
    let mut r = runner_with_opponent_field(1);
    // Don't play — no selection installed.
    let err = r
        .game
        .resolve_selection(0, 0)
        .expect_err("no prompt means no resolution");
    assert_eq!(err, SelectionError::NoPendingSelection);
}

#[test]
fn empty_opponent_field_is_noop() {
    // Opponent has no Digimon → filter produces empty valid set →
    // select_opponent_permanent returns without installing anything.
    let mut r = runner_with_opponent_field(0);
    r.play(0, 0);

    assert!(
        r.game.pending_selection.is_none(),
        "empty valid set must not install a prompt"
    );
    assert_eq!(
        r.current_phase(),
        GamePhase::Main,
        "phase stays unchanged when the effect is a no-op"
    );
}

#[test]
fn tensor_reports_valid_count_and_selecting_player() {
    let mut r = runner_with_opponent_field(2);
    r.play(0, 0);

    let db: std::collections::HashMap<String, _> = r
        .game
        .card_data
        .iter()
        .map(|c| (c.card_id.clone(), c.clone()))
        .collect();
    let registry = CardRegistry::from_cards(&db);
    // The crate-root `build_tensor` follows the default profile
    // (standard_lite_deck_v2); this test asserts the compact-v1 layout's
    // selection slots, so build that profile explicitly.
    let tensor = build_tensor_standard_compact_v1(&r.game, 0, &registry);
    // Tensor offsets: OFF_SELECTION=1370 (phase), +1=valid_count/ACTION_SPACE_SIZE,
    //                 +2=selecting_player.
    use digimon_engine::tensor_profiles::standard::v1::OFF_SELECTION;
    let valid_count_slot = tensor[OFF_SELECTION + 1];
    let expected = 2.0f32 / ACTION_SPACE_SIZE as f32;
    assert!(
        (valid_count_slot - expected).abs() < 1e-6,
        "valid_count slot {} doesn't match {}",
        valid_count_slot,
        expected
    );
    assert_eq!(tensor[OFF_SELECTION + 2], 0.0, "selecting_player = P0");
}
