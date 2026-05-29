use digimon_engine::action::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, ModifierType};
use digimon_engine::permanent::PermanentHandle;

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

fn pulsemon() -> CardData {
    let mut card = make_test_card("PULSE", "Pulsemon");
    card.card_kind = CardKind::Digimon;
    card
}

#[test]
fn bt10_101_at_three_security_applies_dp_minus_and_places_opponent_digimon_on_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT10-101")
        .expect("BT10-101 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .add_card(pulsemon())
        .add_card(make_test_card("OPP-A", "Opponent A"))
        .add_card(make_test_card("OPP-B", "Opponent B"))
        .hand(0, &["BT10-101"])
        .security(0, &["SEC", "SEC", "SEC"])
        .memory(10)
        .start();
    runner.place_on_field(0, "PULSE", Some(0));
    let dp_target = runner.place_on_field(1, "OPP-A", Some(0));
    let sec_target = runner.place_on_field(1, "OPP-B", Some(0));

    assert!(runner.game.activate_hand_main(0, 0));
    let prompt = runner.pending_selection_view().expect("DP target");
    runner
        .execute_action(prompt.selecting_player, encode_permanent(dp_target))
        .expect("choose DP target");
    let prompt = runner.pending_selection_view().expect("security placement target");
    runner
        .execute_action(prompt.selecting_player, encode_permanent(sec_target))
        .expect("choose security target");

    assert_eq!(
        runner.game.modifiers.sum(dp_target, ModifierType::ChangeDp),
        -12000
    );
    assert_eq!(runner.game.players[1].security.len(), 1);
    assert_eq!(
        runner.game.players[1].security.last().unwrap().card_id(&runner.game.card_data),
        "OPP-B"
    );
}

#[test]
fn bt10_101_security_clause_uses_main_effect() {
    let runner = DebugRunner::builder()
        .dsl_card("BT10-101")
        .expect("BT10-101 YAML loads")
        .build();
    let compiled = runner.compiled_card("BT10-101").expect("compiled");
    assert!(
        compiled.effects.iter().any(|effect| format!("{effect:?}").contains("OnDiscardSecurity")),
        "security effect must activate the card's Main effect when trashed from security"
    );
}
