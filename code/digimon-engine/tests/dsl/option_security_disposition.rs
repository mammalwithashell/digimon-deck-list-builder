use digimon_engine::combat::AttackResult;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::PlayerId;

fn security_option_to_hand_yaml() -> &'static str {
    r#"
card: TEST-SECURITY-OPTION-TO-HAND
name: Security Option To Hand
kind: option
color: [red]
cost: 1
effects:
  - when: on_security
    process:
      - add_this_option_to_hand: {}
"#
}

fn zone_contains_card(runner: &DebugRunner, player: PlayerId, zone: Zone, card_id: &str) -> bool {
    let player = runner.game.player(player);
    let cards = match zone {
        Zone::Hand => &player.hand,
        Zone::Trash => &player.trash,
    };
    cards.iter().any(|card| {
        runner
            .game
            .card_data_for_handle(card.handle())
            .is_some_and(|data| data.card_id == card_id)
    })
}

enum Zone {
    Hand,
    Trash,
}

#[test]
fn resolving_security_option_can_move_self_to_hand_without_trashing() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(security_option_to_hand_yaml())
        .expect("inline security option DSL parses")
        .add_card(make_test_card("TEST-ATTACKER", "Test Attacker"))
        .security(1, &["TEST-SECURITY-OPTION-TO-HAND"])
        .start();

    let attacker = runner.place_on_field(0, "TEST-ATTACKER", Some(0));
    let result = runner.attack_player(attacker, 1, false);

    assert_eq!(result, AttackResult::SecurityCheckSurvived);
    assert_eq!(runner.security_count(1), 0);
    assert!(zone_contains_card(
        &runner,
        1,
        Zone::Hand,
        "TEST-SECURITY-OPTION-TO-HAND"
    ));
    assert!(!zone_contains_card(
        &runner,
        1,
        Zone::Trash,
        "TEST-SECURITY-OPTION-TO-HAND"
    ));
}
