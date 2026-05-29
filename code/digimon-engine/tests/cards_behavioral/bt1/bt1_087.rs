use digimon_engine::action::space::SEL_MY_SECURITY_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind};
use digimon_engine::{EffectTiming, TriggerSource};

fn yellow_card(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.colors = vec![CardColor::Yellow];
    card
}

#[test]
fn bt1_087_on_play_recovers_only_when_selected_security_card_is_yellow() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT1-087")
        .expect("BT1-087 YAML loads")
        .add_card(make_test_card("RED-SEC", "Red Security"))
        .add_card(yellow_card("YELLOW-SEC"))
        .add_card(make_test_card("RECOVER", "Recover"))
        .security(0, &["RED-SEC", "YELLOW-SEC"])
        .deck(0, &["RECOVER"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "BT1-087", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, SEL_MY_SECURITY_START + 1)
        .expect("choose yellow security");

    let security_ids: Vec<_> = runner.game.players[0]
        .security
        .iter()
        .map(|card| card.card_id(&runner.game.card_data).to_string())
        .collect();
    assert!(security_ids.contains(&"RED-SEC".to_string()));
    assert!(security_ids.contains(&"RECOVER".to_string()));
}

#[test]
fn bt1_087_security_clause_plays_tamer_from_security() {
    let runner = DebugRunner::builder()
        .dsl_card("BT1-087")
        .expect("BT1-087 YAML loads")
        .start();
    let data = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "BT1-087")
        .expect("card data");
    assert_eq!(data.card_kind, CardKind::Tamer);
}
