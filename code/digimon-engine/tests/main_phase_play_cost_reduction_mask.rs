//! Main-phase play-from-hand action mask affordability must credit declarable
//! `BeforePayCost` play-cost reductions (self and field reducers, CR 4.3
//! §1-3-11-2/3) and fulfillable `[DigiXros]` reductions when play-cost
//! reduction is not locked (`CannotReduceCost` / `CannotReducePlayCost`).

use digimon_engine::action::mask::build_action_mask;
use digimon_engine::action::space::PLAY_HAND_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Expiry, ModifierType};
use digimon_engine::modifiers::PlayerModifierEntry;

fn digimon(id: &str, level: u8, color: CardColor, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, level);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![color];
    card.play_cost = 4;
    card.dp = Some(4000);
    card.traits = traits.iter().map(|t| t.to_string()).collect();
    card
}

fn hand_slot(runner: &DebugRunner, player: u8, card_id: &str) -> usize {
    runner.game.players[player as usize]
        .hand
        .iter()
        .position(|c| c.card_id(&runner.game.card_data) == card_id)
        .unwrap_or_else(|| panic!("{card_id} must be in player {player}'s hand"))
}

#[test]
fn self_play_cost_reduction_is_credited_in_main_phase_mask() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-041")
        .expect("BT24-041 YAML loads")
        .add_card(digimon("ILIAD-ALLY", 4, CardColor::Yellow, &["Iliad"]))
        .add_card(digimon("OPP-MON", 4, CardColor::Red, &[]))
        .hand(0, &["BT24-041"])
        .memory(1)
        .start();

    let slot = hand_slot(&runner, 0, "BT24-041");
    let action = (PLAY_HAND_START as usize) + slot;

    // Without an [Iliad] Digimon/Tamer, printed cost 12 at memory 1 -> -11 < -10.
    let mask_without = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask_without[action], 0.0,
        "BT24-041 (cost 12) must not be playable at memory 1 without an Iliad permanent"
    );

    // With an [Iliad] Digimon, BT24-041 reduces its own play cost by 5 (12 - 5 = 7 -> 1 - 7 = -6 >= -10).
    runner.place_on_field(0, "ILIAD-ALLY", Some(0));
    runner.place_on_field(1, "OPP-MON", Some(0));
    let mask_with = build_action_mask(&runner.game, 0);
    assert!(
        mask_with[action] > 0.5,
        "BT24-041 (cost 12 - 5 = 7) must be playable in Main mask at memory 1 with an Iliad ally"
    );

    // Executing the play succeeds and pays the reduced cost of 7 (memory 1 -> -6).
    runner.game.decode_action(action as u16, 0);
    runner.auto_resolve().expect("finish BT24-041 On Play");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|p| p.top_card().card_id(&runner.game.card_data) == "BT24-041"),
        "BT24-041 entered the battle area"
    );
}

#[test]
fn cannot_reduce_play_cost_suppresses_main_phase_mask_credit() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT24-041")
        .expect("BT24-041 YAML loads")
        .add_card(digimon("ILIAD-ALLY", 4, CardColor::Yellow, &["Iliad"]))
        .hand(0, &["BT24-041"])
        .memory(1)
        .start();

    runner.place_on_field(0, "ILIAD-ALLY", Some(0));
    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotReducePlayCost, 1, Expiry::Permanent, None, 0),
    );

    let slot = hand_slot(&runner, 0, "BT24-041");
    let mask = build_action_mask(&runner.game, 0);
    assert_eq!(
        mask[(PLAY_HAND_START as usize) + slot],
        0.0,
        "CannotReducePlayCost must prevent crediting BT24-041's -5 play-cost reduction in Main mask"
    );
}

#[test]
fn digixros_play_cost_reduction_is_credited_in_main_phase_mask() {
    // EX12-015 Gokuumon: printed cost 7, DigiXros -2 (1 [SW] trait Digimon).
    // At memory -5, printed cost 7 -> -12 < -10 (unplayable), while DigiXros
    // -2 -> cost 5 -> -10 >= -10 (playable when an [SW] Digimon is available).
    let mut runner = DebugRunner::builder()
        .dsl_card("EX12-015")
        .expect("EX12-015 YAML loads")
        .add_card(digimon("SW-MAT", 3, CardColor::Red, &["SW"]))
        .hand(0, &["EX12-015"])
        .memory(-5)
        .start();

    let slot = hand_slot(&runner, 0, "EX12-015");
    let action = (PLAY_HAND_START as usize) + slot;

    assert_eq!(
        build_action_mask(&runner.game, 0)[action],
        0.0,
        "EX12-015 (cost 7) must not be playable at memory -5 without DigiXros material"
    );

    runner.place_on_field(0, "SW-MAT", Some(0));
    assert!(
        build_action_mask(&runner.game, 0)[action] > 0.5,
        "EX12-015 (cost 7 - 2 DigiXros = 5) must be playable in Main mask at memory -5 with an [SW] material"
    );

    runner.game.modifiers.add_player_modifier(
        0,
        PlayerModifierEntry::simple(ModifierType::CannotReducePlayCost, 1, Expiry::Permanent, None, 0),
    );
    assert_eq!(
        build_action_mask(&runner.game, 0)[action],
        0.0,
        "CannotReducePlayCost must suppress DigiXros play-cost credit in Main mask"
    );
}
