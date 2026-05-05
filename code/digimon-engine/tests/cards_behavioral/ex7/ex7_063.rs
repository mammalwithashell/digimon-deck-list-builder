//! EX7-063 Arisa Kinosaki.
//! Printed text covered here: [Start of Your Main Phase] if opponent has a
//! Digimon, gain 1 memory; [Security] play this card.
//!
//! Partial: all-turns Token/Puppet deletion observer that suspends this Tamer
//! to play a level 3 Puppet remains blocked by event context / suspend-cost
//! follow-up coverage.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn ex7_063_start_of_main_gains_memory_when_opponent_has_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-063")
        .expect("EX7-063 YAML loads")
        .add_card(make_test_card("OPP", "Opponent"))
        .memory(5)
        .start();
    runner.place_on_field(0, "EX7-063", Some(0));
    runner.place_on_field(1, "OPP", Some(0));

    runner.game.enter_main_phase();

    assert_eq!(runner.memory(), 6);
}

#[test]
fn ex7_063_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("EX7-063")
        .expect("EX7-063 YAML loads")
        .add_card(attacker)
        .security(1, &["EX7-063"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX7-063"));
}
