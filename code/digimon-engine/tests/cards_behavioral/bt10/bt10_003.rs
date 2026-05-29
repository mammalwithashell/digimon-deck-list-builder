use digimon_engine::debug_runner::{make_test_card, make_test_card_with_level, DebugRunner};
use digimon_engine::enums::CardColor;

const CARD_ID: &str = "BT10-003";

fn carrier(card_id: &str, name: &str, traits: &[&str]) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card_with_level(card_id, name, 3);
    card.colors = vec![CardColor::Red];
    card.traits = traits.iter().map(|trait_name| (*trait_name).to_string()).collect();
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT10-003 YAML loads")
        .add_card(carrier("XROS-CARRIER", "Xros Carrier", &["Xros Heart"]))
        .add_card(carrier("PLAIN-CARRIER", "Plain Carrier", &[]))
        .add_card(make_test_card("FILL", "Filler"))
        .deck(0, &["FILL", "FILL", "FILL"])
        .memory(10)
        .start()
}

#[test]
fn bt10_003_inherited_draws_when_xros_heart_carrier_attacks() {
    let mut runner = runner();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "XROS-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("resolve inherited draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before + 1,
        "Pickmons source should draw 1 for a Xros Heart carrier"
    );
}

#[test]
fn bt10_003_inherited_does_not_draw_without_xros_heart_carrier() {
    let mut runner = runner();
    runner.game_mut().players[1].security.clear();
    let stack = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);

    let hand_before = runner.hand_size(0);
    runner.attack_player(stack, 1, false);
    runner.auto_resolve().expect("resolve attack without inherited draw");

    assert_eq!(
        runner.hand_size(0),
        hand_before,
        "Pickmons source must be gated by the carrier's Xros Heart trait"
    );
}
