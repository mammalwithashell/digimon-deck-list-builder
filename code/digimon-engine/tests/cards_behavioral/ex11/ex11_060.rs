//! EX11-060 Arisa Kinosaki.
//!
//! Printed text covered here:
//! - [Start of Your Turn] If you have 2 or less memory, set it to 3.
//! - [Security] Play this card without paying the cost.
//!
//! Partial: the all-turns Token/Puppet deletion observer remains blocked until
//! deletion triggers carry deleted-object context, Overclock cause context, and
//! the suspend-this-Tamer cost plus optional play branch can be surfaced through
//! current action/pending-selection contracts.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardKind;

#[test]
fn ex11_060_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        3,
        "Arisa sets memory to 3 at start of your turn when memory is 2 or less"
    );
}

#[test]
fn ex11_060_start_of_turn_does_not_lower_memory_above_2() {
    let filler = make_test_card("FILLER-EX11-060", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-EX11-060"])
        .deck(1, &["FILLER-EX11-060"])
        .memory(5)
        .start();

    runner.place_on_field(0, "EX11-060", Some(0));
    runner.game.memory = 5;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(
        runner.memory(),
        5,
        "Arisa must not set memory to 3 when memory is above the printed threshold"
    );
}

#[test]
fn ex11_060_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER-EX11-060", "Attacker");
    attacker.card_kind = CardKind::Digimon;
    attacker.level = Some(4);
    attacker.dp = Some(9000);
    attacker.play_cost = 0;

    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-060")
        .expect("EX11-060 YAML loads")
        .add_card(attacker)
        .security(1, &["EX11-060"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER-EX11-060", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(runner.game.players[1]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "EX11-060"));
}

#[test]
#[ignore = "pending: deletion observer needs deleted-object context, Overclock cause, suspend-cost, draw, and optional play pending-selection support"]
fn ex11_060_all_turns_draws_when_own_puppet_is_deleted_by_non_overclock() {
    unimplemented!(
        "When one of your Token/Puppet Digimon is deleted, Arisa should be able to suspend herself to Draw 1."
    );
}

#[test]
#[ignore = "pending: deletion observer needs deleted-object context, Overclock cause, suspend-cost, draw, and optional play pending-selection support"]
fn ex11_060_all_turns_overclock_deletion_may_play_level_4_or_lower_puppet() {
    unimplemented!(
        "If the deleted Token/Puppet was deleted by Overclock, Arisa should also offer the optional level 4 or lower Puppet play from hand."
    );
}
