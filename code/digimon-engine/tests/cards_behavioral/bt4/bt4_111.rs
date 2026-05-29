//! BT4-111 Jack Raid.
//!
//! [Main] For every 10 cards in your trash, gain 1 memory.
//! [Security] Gain 2 memory.

use digimon_engine::card_source::CardSource;
use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn bt4_111_main_gains_one_memory_per_ten_cards_in_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-111")
        .expect("BT4-111 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT4-111"])
        .memory(0)
        .start();
    push_trash(&mut runner, 0, "FILL", 20);

    let memory_before = runner.memory();
    let fired = runner.game.activate_hand_main(0, 0);
    assert!(fired, "Jack Raid must have a MainFromHand clause");
    runner.auto_resolve().expect("formula gain has no choices");

    assert_eq!(runner.memory(), memory_before + 2);
}

#[test]
fn bt4_111_main_rounds_down_trash_count_below_next_ten() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-111")
        .expect("BT4-111 YAML loads")
        .add_card(make_test_card("FILL", "Filler"))
        .hand(0, &["BT4-111"])
        .memory(0)
        .start();
    push_trash(&mut runner, 0, "FILL", 9);

    let memory_before = runner.memory();
    assert!(runner.game.activate_hand_main(0, 0));
    runner.auto_resolve().expect("formula gain has no choices");

    assert_eq!(runner.memory(), memory_before);
}

#[test]
fn bt4_111_security_reveal_gains_two_memory_for_defender() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT4-111")
        .expect("BT4-111 YAML loads")
        .add_card(make_test_card("ATK", "Attacker"))
        .add_card(make_test_card("FILL", "Filler"))
        .security(1, &["BT4-111"])
        .deck(0, &["FILL"])
        .deck(1, &["FILL"])
        .memory(0)
        .start();

    let attacker = runner.place_on_field(0, "ATK", Some(0));
    let memory_before = runner.memory();
    let result = runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("security memory gain");

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(
        runner.memory(),
        memory_before - 2,
        "P1's security effect gains 2 for P1, moving raw memory negative"
    );
}

fn push_trash(runner: &mut DebugRunner, player: u8, card_id: &str, count: usize) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .expect("trash fixture card exists");
    for _ in 0..count {
        let card_index = runner.game.next_card_index();
        runner.game.players[player as usize]
            .trash
            .push(CardSource::new(data_idx, player, card_index));
    }
}
