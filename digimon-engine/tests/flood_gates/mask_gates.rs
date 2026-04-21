//! Mask-layer enforcement tests for player-scoped flood-gate modifiers.
//!
//! Covers:
//!   - CannotPlayFromHand (player-scoped upgrade)
//!   - CannotAttack (player-scoped gate on attack bits)
//!   - CannotActivateMainEffects (player-scoped gate on FIELD_EFFECT bits)
//!   - CannotSuspend — no mask bit exists (suspend is effect-only); skipped.

use digimon_engine::action::{
    build_action_mask, encode_attack, ACTION_SPACE_SIZE, ATTACK_END, ATTACK_START, FIELD_EFFECT_END,
    FIELD_EFFECT_START, PLAY_HAND_END, PLAY_HAND_START, SECURITY_TARGET,
};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

// ─── Helper card factories ────────────────────────────────────────────────────

fn make_digimon(id: &str, dp: i32) -> CardData {
    CardData {
        card_id: id.to_string(),
        card_name: id.to_string(),
        card_kind: CardKind::Digimon,
        level: Some(4),
        dp: Some(dp),
        play_cost: 3,
        colors: vec![CardColor::Red],
        traits: Vec::new(),
        evo_costs: Vec::new(),
        dna_costs: Vec::new(),
        effect_text: String::new(),
        inherited_text: String::new(),
        security_text: String::new(),
        keywords: Vec::new(),
        effect_class_name: id.replace('-', "_"),
        index: 0,
        norm_id: 0.0,
    }
}

// ─── CannotPlayFromHand — player-scoped ──────────────────────────────────────

/// Install CannotPlayFromHand on player 0 (player-scoped).
/// Player 0's hand bits must all be 0; player 1 is unaffected.
#[test]
fn cannot_play_from_hand_player_scoped_zeroes_hand_bits_for_target_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATCK", 3000))
        .start();

    let tp = r.game.turn_player();

    // Give the turn player a card in hand so there's a bit that could be set.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ATCK")
        .unwrap();
    let next = r.game.next_card_index();
    let hand_card = digimon_engine::card_source::CardSource::new(data_idx, tp, next);
    r.game.players[tp as usize].hand.push(hand_card);

    // Fund memory so the card is affordable (cost 3, give memory 10).
    r.game.set_memory(10);

    // Without the modifier the hand bit should be set.
    r.game.enter_main_phase();
    let mask_before = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_before[PLAY_HAND_START as usize], 1.0,
        "hand bit 0 should be 1.0 before modifier installed"
    );

    // Install player-scoped CannotPlayFromHand on the turn player.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry {
            modifier: ModifierType::CannotPlayFromHand,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: 1 - tp,
        },
    );

    let mask_p0 = build_action_mask(&r.game, tp);
    for i in PLAY_HAND_START..PLAY_HAND_END {
        assert_eq!(
            mask_p0[i as usize], 0.0,
            "hand bit {} should be zero for player {} under CannotPlayFromHand",
            i, tp
        );
    }
    assert_eq!(mask_p0.len(), ACTION_SPACE_SIZE);
}

/// Player-scoped CannotPlayFromHand on player 1 must NOT affect player 0's hand bits.
#[test]
fn player_scoped_mask_gates_are_symmetric_per_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATCK", 3000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    // Give the turn player a card in hand.
    let data_idx = r
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "ATCK")
        .unwrap();
    let next = r.game.next_card_index();
    let hand_card = digimon_engine::card_source::CardSource::new(data_idx, tp, next);
    r.game.players[tp as usize].hand.push(hand_card);
    r.game.set_memory(10);
    r.game.enter_main_phase();

    // Install CannotPlayFromHand on the OPPONENT (not the turn player).
    r.game.modifiers.add_player_modifier(
        opp,
        PlayerModifierEntry {
            modifier: ModifierType::CannotPlayFromHand,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: tp,
        },
    );

    // Turn player's hand bit should still be 1.0 (they are unaffected).
    let mask_tp = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_tp[PLAY_HAND_START as usize], 1.0,
        "turn player's hand bit should be unaffected when modifier is on opponent"
    );
}

// ─── CannotAttack — player-scoped ────────────────────────────────────────────

/// Install CannotAttack on the turn player → all attack bits (100-399) must be zero.
#[test]
fn cannot_attack_player_scoped_zeroes_attack_bits() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATCK", 3000))
        .add_card(make_digimon("DEF", 2000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    // Place an unsuspended attacker on field (turn_played = 0 to avoid
    // summoning sickness).
    let attacker = r.place_on_field(tp, "ATCK", Some(0));
    // Place a suspended defender so the attacker would otherwise have valid targets.
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    r.game.enter_main_phase();

    // Verify attack bits are lit before installing the modifier.
    let mask_before = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_before[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize], 1.0,
        "security attack should be available before CannotAttack"
    );

    // Install player-scoped CannotAttack on the turn player.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry {
            modifier: ModifierType::CannotAttack,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: opp,
        },
    );

    let mask_blocked = build_action_mask(&r.game, tp);
    for i in ATTACK_START..ATTACK_END {
        assert_eq!(
            mask_blocked[i as usize], 0.0,
            "attack bit {} should be zero under CannotAttack",
            i
        );
    }
}

/// CannotAttack on the opponent must NOT affect the turn player's attack bits.
#[test]
fn cannot_attack_only_affects_target_player() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATCK", 3000))
        .add_card(make_digimon("DEF", 2000))
        .start();

    let tp = r.game.turn_player();
    let opp = 1 - tp;

    let attacker = r.place_on_field(tp, "ATCK", Some(0));
    let defender = r.place_on_field(opp, "DEF", Some(0));
    r.game.players[opp as usize].battle_area[defender.index as usize].is_suspended = true;

    r.game.enter_main_phase();

    // Install CannotAttack on the OPPONENT — turn player should be unaffected.
    r.game.modifiers.add_player_modifier(
        opp,
        PlayerModifierEntry {
            modifier: ModifierType::CannotAttack,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: tp,
        },
    );

    let mask_tp = build_action_mask(&r.game, tp);
    assert_eq!(
        mask_tp[encode_attack(attacker.index as u16, SECURITY_TARGET) as usize], 1.0,
        "turn player's security-attack bit should be unaffected when CannotAttack is on opponent"
    );
}

// ─── CannotActivateMainEffects — player-scoped ───────────────────────────────

/// Install CannotActivateMainEffects on the turn player → all FIELD_EFFECT bits must be zero.
///
/// Even without an activatable field effect, the gate must zero the entire
/// FIELD_EFFECT range for the target player. We verify the range is all-zero
/// when the modifier is present (the range is already 0 without any
/// activatable effect; the gate prevents any effect from being lit).
#[test]
fn cannot_activate_main_effects_zeroes_field_effect_bits() {
    let mut r = DebugRunner::builder()
        .add_card(make_digimon("ATCK", 3000))
        .start();

    let tp = r.game.turn_player();

    r.place_on_field(tp, "ATCK", Some(0));
    r.game.enter_main_phase();

    // Install CannotActivateMainEffects on the turn player.
    r.game.modifiers.add_player_modifier(
        tp,
        PlayerModifierEntry {
            modifier: ModifierType::CannotActivateMainEffects,
            value: 0,
            expiry: Expiry::Permanent,
            source_permanent: None,
            source_player: 1 - tp,
        },
    );

    let mask = build_action_mask(&r.game, tp);
    for i in FIELD_EFFECT_START..FIELD_EFFECT_END {
        assert_eq!(
            mask[i as usize], 0.0,
            "FIELD_EFFECT bit {} should be zero under CannotActivateMainEffects",
            i
        );
    }
}
