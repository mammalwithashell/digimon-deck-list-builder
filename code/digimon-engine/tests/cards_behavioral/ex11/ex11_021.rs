//! EX11-021 Kokeshimon.
//! Printed text covered here: [When Digivolving] if you have 1 or fewer
//! Tamers, you may play 1 Mirai Kinosaki from hand without paying cost.
//!
//! Inherited: [Opponent's Turn] [Once Per Turn] when an opponent's Digimon
//! attacks, by deleting 1 of your other Digimon, end that attack.

use digimon_engine::action::space::PASS;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::{SelectionKind, TriggerSource};

#[test]
fn ex11_021_when_digivolving_may_play_mirai_from_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("BASE", "Base"))
        .add_card(make_tamer("MIRAI", "Mirai Kinosaki"))
        .hand(0, &["MIRAI"])
        .memory(10)
        .start();
    let koko = runner.place_stack(0, &["BASE", "EX11-021"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(koko),
    );
    runner.game.drain_effect_queue();

    let view = runner.pending_selection_view().expect("Mirai hand prompt");
    assert_eq!(view.kind, SelectionKind::Hand);
    assert!(
        runner.pending_is_optional(),
        "play-Mirai selection is optional"
    );
    runner
        .execute_action(0, view.valid_action_ids[0])
        .expect("select Mirai");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|perm| perm.top_card().card_name(&runner.game.card_data) == "Mirai Kinosaki"));
}

#[test]
fn ex11_021_inherited_may_delete_other_digimon_to_end_opponent_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
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
fn ex11_021_inherited_lone_carrier_does_not_end_attack_for_free() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    // Lone carrier: EX11-021 is the only stack on field, no OTHER Digimon.
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
    let attacker = runner.place_on_field(1, "ATTACKER", Some(0));
    runner.end_turn();

    runner.attack_player(attacker, 0, false);

    assert!(
        runner.pending_selection_view().is_none(),
        "no other Digimon exists to delete, so no attack-cancel prompt is offered"
    );
    runner
        .auto_resolve()
        .expect("attack resolves with no cancel selection");

    assert_eq!(
        runner.security_count(0),
        0,
        "ending the attack is a paid effect — with no other Digimon to delete, \
         the attack must NOT end for free; security is checked"
    );
}

#[test]
fn ex11_021_inherited_decline_does_not_delete_or_end_attack() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX11-021")
        .expect("EX11-021 YAML loads")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("OTHER", "Other Digimon"))
        .add_card(make_test_card("ATTACKER", "Attacker"))
        .add_card(make_test_card("SECURITY", "Security"))
        .security(0, &["SECURITY"])
        .start();
    runner.place_stack(0, &["EX11-021", "CARRIER"]);
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

fn make_tamer(id: &str, name: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = digimon_engine::enums::CardKind::Tamer;
    card
}
