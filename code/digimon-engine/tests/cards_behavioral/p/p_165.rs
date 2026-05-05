//! P-165 ShoeShoemon.
//! Printed text covered here:
//! [Security] At end of battle, play this card without paying cost.
//! [On Play] [When Digivolving] Play 1 Familiar Token.
//! Inherited: <Barrier>.
//!
//! The delayed cleanup clause, "at end of your opponent's turn, delete that
//! token", needs token-handle provenance from play_token and is intentionally
//! not approximated here.

use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{EffectTiming, Keyword};
use digimon_engine::selection::TriggerSource;

fn familiar_count(runner: &DebugRunner, player: usize) -> usize {
    runner.game.players[player]
        .battle_area
        .iter()
        .filter(|perm| perm.top_card().card_name(&runner.game.card_data) == "Familiar Token")
        .count()
}

#[test]
fn p_165_on_play_plays_one_familiar_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-165")
        .expect("P-165 YAML loads")
        .hand(0, &["P-165"])
        .memory(10)
        .start();

    runner.play(0, 0).expect("P-165 plays from hand");
    runner.auto_resolve().expect("finish On Play token effect");

    assert_eq!(runner.battle_area_size(0), 2, "ShoeShoemon plus token");
    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
fn p_165_when_digivolving_plays_one_familiar_token() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-165")
        .expect("P-165 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .memory(10)
        .start();
    let shoeshoe = runner.place_stack(0, &["BASE", "P-165"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(shoeshoe),
    );
    runner.game.drain_effect_queue();
    runner
        .auto_resolve()
        .expect("finish When Digivolving token effect");

    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "one Familiar Token is played"
    );
}

#[test]
fn p_165_inherited_barrier_is_available_from_stack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-165")
        .expect("P-165 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .start();
    let carrier = runner.place_stack(0, &["P-165", "CARRIER"]);

    assert!(
        runner.game.has_keyword(carrier, Keyword::Barrier),
        "carrier inherits Barrier from P-165"
    );
}

#[test]
fn p_165_security_effect_plays_card_onto_field_after_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-165")
        .expect("P-165 YAML loads")
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .memory(10)
        .start();

    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));
    let p165_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_id == "P-165")
        .expect("P-165 in card_data");
    let next = runner.game.next_card_index();
    runner.game.players[1]
        .security
        .push(CardSource::new(p165_idx, 1, next));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve security play");

    assert_eq!(runner.security_count(1), 0, "security card is removed");
    assert!(
        runner.game.players[1]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "P-165"),
        "P-165 is played onto its owner's field"
    );
}

#[test]
#[ignore = "pending: PUPPETS-G016 - play_token must bind the created token handle and schedule deletion of that exact token at opponent turn end"]
fn p_165_deletes_that_token_at_end_of_opponents_turn() {
    todo!("unignore when token provenance and scheduled cleanup for 'that token' are available")
}
