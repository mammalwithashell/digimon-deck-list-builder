use digimon_engine::enums::{CardColor, Keyword};
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    field_contains, hand_index, plain_digimon, puppet_tb, push_to_trash, select_first_non_pass,
    DebugRunner,
};

const CARD_ID: &str = "EX12-065";

#[test]
fn ex12_065_has_fortitude_and_grants_blocker_retaliation_to_puppet_or_tb() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET", 4))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    let puppet = runner.place_on_field(0, "PUPPET", Some(0));
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(kaguyamon, Keyword::Fortitude));
    assert!(runner.game.has_keyword(puppet, Keyword::Blocker));
    assert!(runner.game.has_keyword(puppet, Keyword::Retaliation));
}

#[test]
fn ex12_065_on_play_plays_low_cost_puppet_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(puppet_tb("PUPPET-LOW", 4))
        .hand(0, &[CARD_ID])
        .memory(12)
        .start();
    push_to_trash(&mut runner, 0, "PUPPET-LOW");

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-065");
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve trash play");

    assert!(field_contains(&runner, 0, "PUPPET-LOW"));
}

#[test]
fn ex12_065_on_deletion_bottom_decks_opponent_lowest_level_digimon() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-065 YAML loads")
        .add_card(plain_digimon("LOW", CardColor::Purple, 3, 3000))
        .add_card(plain_digimon("HIGH", CardColor::Purple, 5, 7000))
        .start();
    let kaguyamon = runner.place_on_field(0, CARD_ID, Some(0));
    runner.place_on_field(1, "LOW", Some(0));
    runner.place_on_field(1, "HIGH", Some(0));

    runner
        .game
        .delete_permanents_batch(vec![kaguyamon], ReplacementCause::OwnEffect);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve bottom-deck");

    assert_eq!(
        runner.game.players[1]
            .deck
            .last()
            .unwrap()
            .card_id(&runner.game.card_data),
        "LOW"
    );
}
