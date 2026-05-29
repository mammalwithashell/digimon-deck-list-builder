use digimon_engine::action::space::{HAND_EFFECT_START, SEL_REVEAL_START};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;

fn yellow_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card
}

fn ids(cards: &[digimon_engine::card_source::CardSource], game: &digimon_engine::game::Game) -> Vec<String> {
    cards
        .iter()
        .map(|card| card.card_id(&game.card_data).to_string())
        .collect()
}

#[test]
fn bt6_100_main_places_chosen_reveal_in_security_and_remaining_card_in_hand() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-100")
        .expect("BT6-100 YAML loads")
        .add_card(yellow_digimon("YELLOW"))
        .add_card(make_test_card("KEEP", "Keep"))
        .add_card(make_test_card("SECURE", "Secure"))
        .hand(0, &["BT6-100"])
        .deck(0, &["KEEP", "SECURE"])
        .memory(10)
        .start();
    runner.place_on_field(0, "YELLOW", Some(0));
    runner.game.enter_main_phase();

    runner.game.decode_action(HAND_EFFECT_START, 0);
    let secure_action = runner
        .game
        .revealed_cards
        .iter()
        .enumerate()
        .find_map(|(index, card)| {
            (card.card_id(&runner.game.card_data) == "SECURE")
                .then_some(SEL_REVEAL_START + index as u16)
        })
        .expect("SECURE was revealed");
    runner
        .execute_action(0, secure_action)
        .expect("choose revealed card for security");
    runner.auto_resolve().expect("finish remaining-card flow");

    assert!(ids(&runner.game.players[0].security, &runner.game).contains(&"SECURE".to_string()));
    assert!(ids(&runner.game.players[0].hand, &runner.game).contains(&"KEEP".to_string()));
    assert!(runner.game.players[0]
        .battle_area
        .iter()
        .any(|permanent| matches!(permanent.option_state, OptionState::Delayed { .. })));
}

#[test]
fn bt6_100_delay_gains_3_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-100")
        .expect("BT6-100 YAML loads")
        .memory(0)
        .start();
    let boost = runner.place_on_field(0, "BT6-100", Some(0));
    runner.game.players[0].battle_area[boost.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: 0,
        };
    runner.game.turn_count = 1;
    runner.game.enter_main_phase();

    assert!(runner.game.activate_delayed_option_main(boost));

    assert_eq!(runner.memory(), 3);
}
