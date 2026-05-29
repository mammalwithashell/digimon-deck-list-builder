//! BT4-084 NeoDevimon.
//!
//! [Opponent's Turn] When an opponent plays a Tamer, gain 3 memory.
//! Inherited: [Opponent's Turn] When an opponent's Tamer becomes suspended,
//! gain 1 memory.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};

#[test]
fn bt4_084_gains_three_when_opponent_plays_tamer_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-084")
        .expect("BT4-084 YAML loads")
        .add_card(tamer("OPP-TAMER", CardColor::Yellow, 0))
        .hand(1, &["OPP-TAMER"])
        .memory(0)
        .start();

    runner.place_on_field(0, "BT4-084", Some(0));
    runner.game.turn_player_idx = 1;
    let memory_before = runner.memory();

    runner.play(1, 0).expect("opponent plays tamer");
    runner
        .auto_resolve()
        .expect("memory trigger has no choices");

    assert_eq!(
        runner.memory(),
        memory_before - 3,
        "P0's NeoDevimon gains 3 during P1's turn, so raw memory moves negative"
    );
}

#[test]
fn bt4_084_inherited_gains_one_when_opponent_tamer_suspends_on_opponents_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-084")
        .expect("BT4-084 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(tamer("OPP-TAMER", CardColor::Purple, 0))
        .start();

    runner.place_stack(0, &["BT4-084", "CARRIER"]);
    let tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));
    runner.game.turn_player_idx = 1;
    let memory_before = runner.memory();

    runner.game.suspend(tamer);
    runner
        .auto_resolve()
        .expect("memory trigger has no choices");

    assert_eq!(runner.memory(), memory_before - 1);
}

#[test]
fn bt4_084_does_not_fire_on_own_turn_or_for_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-084")
        .expect("BT4-084 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(digimon("OPP-DIGIMON"))
        .start();

    runner.place_on_field(0, "BT4-084", Some(0));
    runner.place_stack(0, &["BT4-084", "CARRIER"]);
    let opponent_digimon = runner.place_on_field(1, "OPP-DIGIMON", Some(0));
    let memory_before = runner.memory();

    runner.game.suspend(opponent_digimon);
    runner.auto_resolve().expect("no memory");

    assert_eq!(runner.memory(), memory_before);
}

fn digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.level = Some(5);
    card.dp = Some(6000);
    card.play_cost = 0;
    card
}

fn tamer(id: &str, color: CardColor, cost: u16) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card.play_cost = cost;
    card
}
