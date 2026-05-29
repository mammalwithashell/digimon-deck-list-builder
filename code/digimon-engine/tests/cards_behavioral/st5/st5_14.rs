//! ST5-14 Tai Kamiya: blocker response and security play.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, Keyword};
use digimon_engine::selection::SelectionKind;

fn digimon(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Digimon;
    card.level = Some(4);
    card.dp = Some(dp);
    card
}

fn blocker(id: &str, name: &str, dp: i32) -> digimon_engine::card_data::CardData {
    let mut card = digimon(id, name, dp);
    card.keywords.push(Keyword::Blocker);
    card
}

fn tai_blocker_runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 YAML parses")
        .add_card(blocker("ST5-14-BLOCKER", "Tai Test Blocker", 12000))
        .add_card(digimon("ST5-14-ATTACKER", "Tai Test Attacker", 1000))
        .memory(0)
        .build()
}

#[test]
fn st5_14_opponent_turn_blocker_response_suspends_tai_to_unsuspend_a_digimon() {
    let mut runner = tai_blocker_runner();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;
    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let blocker = runner.place_on_field(0, "ST5-14-BLOCKER", Some(0));
    let attacker = runner.place_on_field(1, "ST5-14-ATTACKER", Some(0));

    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let block_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, block_action)
        .expect("declare ST5 blocker");

    assert!(
        matches!(
            runner.pending_kind(),
            Some(SelectionKind::Replacement | SelectionKind::TriggerOrder)
        ),
        "Tai's optional blocker response should be offered after a Blocker redirect"
    );
    runner
        .accept_optional_trigger()
        .expect("accept Tai blocker response");

    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    let unsuspend_pick = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, unsuspend_pick)
        .expect("unsuspend the blocker with Tai");

    assert!(
        runner.game.players[0].battle_area[tai.index as usize].is_suspended,
        "Tai pays the response cost by suspending"
    );
    assert!(
        !runner.game.players[0].battle_area[blocker.index as usize].is_suspended,
        "Tai can unsuspend the Digimon that blocked"
    );
}

#[test]
fn st5_14_does_not_trigger_when_tai_cannot_pay_suspend_cost() {
    let mut runner = tai_blocker_runner();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;
    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let _blocker = runner.place_on_field(0, "ST5-14-BLOCKER", Some(0));
    let attacker = runner.place_on_field(1, "ST5-14-ATTACKER", Some(0));
    runner.game.players[0].battle_area[tai.index as usize].is_suspended = true;

    let _ = runner.attack_player(attacker, 0, false);
    let block_action = runner.pending_selection_view().unwrap().valid_action_ids[0];
    runner
        .execute_action(0, block_action)
        .expect("declare ST5 blocker");

    assert!(
        runner.pending_selection_view().is_none(),
        "already-suspended Tai cannot pay the suspend cost, so no response prompt is installed"
    );
}

#[test]
fn st5_14_declining_blocker_does_not_offer_tai_response() {
    let mut runner = tai_blocker_runner();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;
    let tai = runner.place_on_field(0, "ST5-14", Some(0));
    let attacker = runner.place_on_field(1, "ST5-14-ATTACKER", Some(0));
    runner.place_on_field(0, "ST5-14-BLOCKER", Some(0));

    let _ = runner.attack_player(attacker, 0, false);
    assert_eq!(runner.pending_kind(), Some(SelectionKind::OwnField));
    runner.execute_action(0, PASS).expect("decline block");

    assert!(
        runner.pending_selection_view().is_none(),
        "without a Blocker target change, Tai has no trigger"
    );
    assert!(
        !runner.game.players[0].battle_area[tai.index as usize].is_suspended,
        "Tai remains unsuspended when no blocker response is offered"
    );
}

#[test]
fn st5_14_security_plays_tai_without_paying_the_cost() {
    let attacker = digimon("ST5-14-SEC-ATTACKER", "Tai Security Attacker", 6000);
    let mut runner = DebugRunner::builder()
        .dsl_card("ST5-14")
        .expect("ST5-14 YAML parses")
        .add_card(attacker)
        .security(0, &["ST5-14"])
        .build();
    runner.game.turn_count = 1;
    runner.game.turn_player_idx = 1;
    let attacker = runner.place_on_field(1, "ST5-14-SEC-ATTACKER", Some(0));

    let before = runner.battle_area_size(0);
    let _ = runner.attack_player(attacker, 0, false);

    assert_eq!(
        runner.battle_area_size(0),
        before + 1,
        "ST5-14 security effect plays Tai into the battle area"
    );
}
