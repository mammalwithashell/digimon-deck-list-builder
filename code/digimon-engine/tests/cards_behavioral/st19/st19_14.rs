//! ST19-14 Arisa Kinosaki.
//! Printed text covered here: [Start of Your Turn] memory setter and
//! [Security] play this card. The effect-played Token/Puppet Rush observer is
//! tracked under PUPPETS-G005.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

#[test]
fn st19_14_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("FILLER-ST19-14", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .add_card(filler)
        .deck(0, &["FILLER-ST19-14"])
        .deck(1, &["FILLER-ST19-14"])
        .memory(2)
        .start();

    runner.place_on_field(0, "ST19-14", Some(0));
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
fn st19_14_security_plays_itself_without_paying_cost() {
    let mut attacker = make_test_card("ATTACKER", "Attacker");
    attacker.level = Some(4);
    attacker.dp = Some(9000);

    let mut runner = DebugRunner::builder()
        .dsl_card("ST19-14")
        .expect("ST19-14 YAML loads")
        .add_card(attacker)
        .security(1, &["ST19-14"])
        .memory(10)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "ST19-14"),
        "Arisa is played from the defender's security"
    );
}
