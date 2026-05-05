//! EX11-020 Hanimon.
//! Printed text covered here:
//! - [On Deletion] If deleted other than in battle, you may play 1 [Shoemon]
//!   trait from your hand without paying the cost.
//! - Inherited: [Opponent's Turn] [Once Per Turn] when one of your opponent's
//!   Digimon attacks, by deleting 1 of your other Digimon, end that attack.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::replacement::ReplacementCause;
use digimon_engine::selection::SelectionKind;

#[test]
#[ignore = "BLOCKED: YAML on_deletion predicates cannot inspect deletion_cause to exclude battle"]
fn ex11_020_on_deletion_may_play_shoemon_trait_from_hand_if_not_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon_trait("SHOEMON-TRAIT", "Shoemon Friend"))
        .add_card(make_test_card("NOT-SHOEMON", "Not Shoemon"))
        .hand(0, &["SHOEMON-TRAIT", "NOT-SHOEMON"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::OpponentEffect);

    assert!(
        runner.pending_is_optional(),
        "play-from-hand prompt must be optional"
    );
    let view = runner
        .pending_selection_view()
        .expect("Shoemon-trait hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the Shoemon-trait card is eligible"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("choose Shoemon-trait card");
    runner.auto_resolve().expect("finish free play");

    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "SHOEMON-TRAIT"),
        "selected Shoemon-trait card enters the battle area"
    );
    assert_eq!(runner.hand_size(0), 1, "non-Shoemon card remains in hand");
}

#[test]
#[ignore = "BLOCKED: YAML on_deletion predicates cannot inspect deletion_cause to exclude battle"]
fn ex11_020_on_deletion_can_decline_free_play() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon_trait("SHOEMON-TRAIT", "Shoemon Friend"))
        .hand(0, &["SHOEMON-TRAIT"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::OwnEffect);
    runner.execute_action(0, PASS).expect("decline free play");

    assert_eq!(runner.hand_size(0), 1, "declining leaves the card in hand");
    assert!(
        runner.pending_selection_view().is_none(),
        "decline resolves without hidden auto-play"
    );
}

#[test]
#[ignore = "BLOCKED: YAML on_deletion predicates cannot inspect deletion_cause to exclude battle"]
fn ex11_020_on_deletion_does_not_fire_when_deleted_in_battle() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_shoemon_trait("SHOEMON-TRAIT", "Shoemon Friend"))
        .hand(0, &["SHOEMON-TRAIT"])
        .memory(0)
        .start();
    let hanimon = runner.place_on_field(0, "EX11-020", Some(0));

    runner
        .game
        .delete_permanent_with_cause(hanimon, ReplacementCause::Battle);

    assert!(
        runner.pending_selection_view().is_none(),
        "battle deletion must not offer the free-play prompt"
    );
    assert_eq!(runner.hand_size(0), 1, "hand card was not played");
}

#[test]
fn ex11_020_inherited_may_delete_other_digimon_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-020", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    let view = runner
        .pending_selection_view()
        .expect("other-Digimon cost selection");
    assert_eq!(view.kind, SelectionKind::OwnField);
    assert!(
        runner.pending_is_optional(),
        "attack cancel must be declinable and cannot auto-delete"
    );
    assert_eq!(
        view.valid_action_ids.len(),
        1,
        "only the other Digimon, not the carrier, is eligible"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("delete other Digimon and end attack");
    runner.auto_resolve().expect("finish attack cancel");

    assert_eq!(runner.security_count(0), 1, "security was not checked");
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .any(|perm| perm.top_card().card_id(&runner.game.card_data) == "CARRIER"),
        "carrier remains because the printed cost says other Digimon"
    );
    assert!(
        runner.game.players[0]
            .battle_area
            .iter()
            .all(|perm| perm.top_card().card_id(&runner.game.card_data) != "OTHER"),
        "selected other Digimon was deleted as the cost"
    );
    assert!(
        runner.game.pending_attack.is_none(),
        "attack state is fully cleared"
    );
}

#[test]
fn ex11_020_inherited_decline_does_not_delete_or_end_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-020")
        .expect("EX11-020 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-020", "CARRIER"]);
    runner.place_on_field(0, "OTHER", Some(0));
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);
    runner
        .execute_action(0, PASS)
        .expect("decline attack cancel");

    assert_eq!(
        runner.battle_area_size(0),
        2,
        "declining does not delete either Digimon"
    );
    assert_eq!(
        runner.security_count(0),
        0,
        "declining does not end the attack, so the security check happens"
    );
}

fn make_shoemon_trait(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.traits = vec!["Shoemon".to_string()];
    card
}
