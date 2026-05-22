//! P-165 ShoeShoemon.
//! Printed text covered here:
//! [Security] At end of battle, play this card without paying cost.
//! [On Play] [When Digivolving] Play 1 Familiar Token; at the end of your
//!   opponent's turn, delete that token.
//! Inherited: <Barrier>.

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

/// PUPPETS-G016 — P-165 creates a Familiar Token via On Play.
/// At the end of the **opponent's** turn P-165's token must be deleted.
/// A second Familiar Token placed by a separate effect must NOT be deleted.
/// (This validates the provenance-keyed identity: only the token whose handle
/// was bound by P-165's own `play_token` is scheduled for deletion.)
#[test]
fn p_165_deletes_that_token_at_end_of_opponents_turn() {
    // P1 needs a non-empty deck to survive their draw step (otherwise they
    // deck-out immediately at turn start and `game_over` prevents end_turn
    // from reaching `rotate_turn_player`).
    let dummy = make_test_card("DUMMY", "Dummy");
    let mut runner = DebugRunner::builder()
        .dsl_card("P-165")
        .expect("P-165 YAML loads")
        .add_card(dummy)
        .hand(0, &["P-165"])
        .deck(1, &["DUMMY", "DUMMY", "DUMMY", "DUMMY", "DUMMY"])
        .memory(10)
        .start();

    // P0 plays P-165; this triggers On Play → plays one Familiar Token.
    runner.play(0, 0).expect("P-165 plays from hand");
    runner.auto_resolve().expect("On Play token effect");

    // P0 should now have ShoeShoemon + 1 Familiar Token.
    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "setup: one Familiar Token from P-165 On Play"
    );

    // Place a SECOND Familiar Token by directly pushing a CardSource into the
    // battle area — simulating a token created by a *different* effect.
    // `CardSource::new_token` marks it is_token=true; `Permanent::new` wraps it.
    let familiar_data_idx = runner
        .game
        .card_data
        .iter()
        .position(|c| c.card_name.to_lowercase().contains("familiar token"))
        .expect("Familiar Token must be registered in card_data");
    let second_tok_idx = runner.game.next_card_index();
    let second_tok_src = CardSource::new_token(familiar_data_idx, 0, second_tok_idx);
    runner.game.players[0]
        .battle_area
        .push(digimon_engine::permanent::Permanent::new(
            second_tok_src,
            runner.game.turn_count,
        ));

    // Both tokens are now on P0's field.
    assert_eq!(
        familiar_count(&runner, 0),
        2,
        "setup: two Familiar Tokens on field"
    );

    // The provenance deletion is queued for the opponent-turn boundary.
    assert_eq!(
        runner.game.scheduled_provenance_deletions_opp.len(),
        1,
        "one opponent-turn-end provenance deletion must be queued"
    );

    // End P0's turn → P1's turn starts. The drain must NOT fire here (it
    // fires at the end of P1's turn, not P0's).
    runner.game.enter_main_phase();
    runner.end_turn();
    let _ = runner.auto_resolve();

    // After P0's turn ends (P1's turn starts), both tokens must still be alive.
    assert_eq!(
        familiar_count(&runner, 0),
        2,
        "tokens must survive end of P0's own turn (drain only fires at opponent-turn end)"
    );

    // End P1's turn → `rotate_turn_player` fires `EndOfOpponentsTurn` for P0
    // and drains `scheduled_provenance_deletions_opp`.
    runner.game.enter_main_phase();
    runner.end_turn();
    let _ = runner.auto_resolve();

    // After P1's turn ends: only P-165's token is deleted; the second survives.
    assert_eq!(
        familiar_count(&runner, 0),
        1,
        "only P-165's token is deleted at end of opponent's turn; the second survives"
    );

    // Confirm the surviving token is the second (unrelated) one.
    let second_still_present = runner.game.players[0]
        .battle_area
        .iter()
        .any(|p| p.top_card().card_index == second_tok_idx);
    assert!(
        second_still_present,
        "the second Familiar Token (not created by P-165) must NOT be deleted"
    );

    // Queue must be fully drained.
    assert!(
        runner.game.scheduled_provenance_deletions_opp.is_empty(),
        "the opponent-turn-end provenance deletion queue must be empty after drain"
    );
}
