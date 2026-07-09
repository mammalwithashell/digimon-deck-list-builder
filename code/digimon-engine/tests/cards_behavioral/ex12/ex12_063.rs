use digimon_engine::enums::{CardColor, ModifierType};
use digimon_engine::replacement::ReplacementCause;

use super::support::{
    field_contains, hand_index, is_suspended, plain_digimon, puppet_tb, select_first_non_pass,
    DebugRunner,
};

const CARD_ID: &str = "EX12-063";

#[test]
fn ex12_063_on_play_suspends_opponent_then_locks_unsuspend() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(plain_digimon("OPP", CardColor::Green, 4, 5000))
        .hand(0, &[CARD_ID])
        .memory(10)
        .start();
    let opp = runner.place_on_field(1, "OPP", Some(0));

    let play_slot = hand_index(&runner, 0, CARD_ID);
    runner.play(0, play_slot).expect("play EX12-063");
    select_first_non_pass(&mut runner);
    assert!(is_suspended(&runner, 1, opp.index as usize));

    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish lock");

    assert!(
        runner.modifiers().has(opp, ModifierType::CannotUnsuspend),
        "target cannot unsuspend until their turn ends"
    );
}

#[test]
fn ex12_063_on_deletion_may_play_level4_or_lower_puppet_or_tb_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(puppet_tb("PUPPET-L4", 4))
        .deck(0, &["PUPPET-L4"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let karakurumon = runner.place_on_field(0, CARD_ID, Some(0));

    runner
        .game
        .delete_permanent_with_cause(karakurumon, ReplacementCause::OpponentEffect);
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("finish on-deletion play");

    assert!(field_contains(&runner, 0, "PUPPET-L4"));
}

#[test]
fn ex12_063_inherited_on_deletion_plays_level4_or_lower_tb_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-063 YAML loads")
        .add_card(plain_digimon("CARRIER", CardColor::Purple, 4, 5000))
        .add_card(puppet_tb("TB-L4", 4))
        .deck(0, &["TB-L4"])
        .memory(10)
        .start();
    let trash_card = runner.game.players[0].deck.pop().expect("seed trash card");
    runner.game.players[0].trash.push(trash_card);
    let carrier = runner.place_stack(0, &[CARD_ID, "CARRIER"]);

    runner
        .game
        .delete_permanent_with_cause(carrier, ReplacementCause::OpponentEffect);
    select_first_non_pass(&mut runner);
    runner
        .auto_resolve()
        .expect("finish inherited on-deletion play");

    assert!(field_contains(&runner, 0, "TB-L4"));
}
