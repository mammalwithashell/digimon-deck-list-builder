use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

const CARD_ID: &str = "EX12-004";

fn purple_digimon(id: &str, traits: &[&str]) -> CardData {
    let mut card = make_test_card_with_level(id, id, 3);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Purple];
    card.play_cost = 3;
    card.dp = Some(3000);
    card.traits = traits
        .iter()
        .map(|trait_name| trait_name.to_string())
        .collect();
    card
}

#[test]
fn ex12_004_inherited_grants_execute_to_tb_carrier_on_your_turn() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(purple_digimon("TB-CARRIER", &["TB"]))
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "TB-CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        runner.game.has_keyword(carrier, Keyword::Execute),
        "TB carrier should inherit Execute from EX12-004"
    );
}

#[test]
fn ex12_004_inherited_does_not_grant_execute_to_non_tb_carrier() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-004 YAML loads")
        .add_card(purple_digimon("PLAIN-CARRIER", &[]))
        .memory(5)
        .start();

    let carrier = runner.place_stack(0, &[CARD_ID, "PLAIN-CARRIER"]);
    runner.game.tick_declarative_effects();
    assert!(
        !runner.game.has_keyword(carrier, Keyword::Execute),
        "non-TB carrier should not inherit Execute from EX12-004"
    );
}
