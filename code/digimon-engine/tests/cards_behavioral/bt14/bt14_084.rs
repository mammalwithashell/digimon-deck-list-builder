use digimon_engine::action::space::HAND_EFFECT_START;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::CardColor;
use digimon_engine::{EffectTiming, TriggerSource};

fn yellow_vaccine(card_id: &str) -> CardData {
    let mut card = make_test_card(card_id, card_id);
    card.colors = vec![CardColor::Yellow];
    card.traits = vec!["Vaccine".to_string()];
    card
}

#[test]
fn bt14_084_on_play_pays_top_security_to_hand_before_hand_placement_prompt() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT14-084")
        .expect("BT14-084 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .add_card(yellow_vaccine("HAND-VACCINE"))
        .security(0, &["SEC"])
        .hand(0, &["HAND-VACCINE"])
        .memory(0)
        .start();
    let source = runner.place_on_field(0, "BT14-084", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(source));
    runner.game.drain_effect_queue();
    runner
        .execute_action(0, HAND_EFFECT_START)
        .expect("pay top-security cost");

    assert_eq!(runner.security_count(0), 0);
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "SEC"));
    assert!(
        runner.pending_selection_view().is_some(),
        "after paying the cost, the hand-card placement prompt should be visible"
    );
}
