use crate::dsl_card_data::card_data_from_compiled;
use digimon_engine::enums::CardColor;
use digimon_engine::permanent::OptionState;

use super::support::{
    field_contains, hand_contains, hand_index, plain_digimon, select_first_non_pass, vb_digimon,
    DebugRunner,
};

const CARD_ID: &str = "EX12-073";

#[test]
fn ex12_073_is_white_option() {
    let data = card_data_from_compiled(CARD_ID);
    assert_eq!(data.colors, vec![CardColor::White]);
}

fn is_delayed_option_on_field(runner: &DebugRunner, player: u8, card_id: &str) -> bool {
    runner.game.players[player as usize]
        .battle_area
        .iter()
        .any(|permanent| {
            permanent.top_card().card_id(&runner.game.card_data) == card_id
                && matches!(permanent.option_state, OptionState::Delayed { .. })
        })
}

#[test]
fn ex12_073_main_searches_trait_card_and_places_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-073 YAML loads")
        .add_card(vb_digimon("VB-ENABLER", CardColor::Yellow, 4, 5000))
        .add_card(vb_digimon("HIT", CardColor::Yellow, 4, 5000))
        .add_card(plain_digimon("MISS-A", CardColor::Purple, 3, 2000))
        .add_card(plain_digimon("MISS-B", CardColor::Purple, 3, 2000))
        .hand(0, &[CARD_ID])
        .deck(0, &["HIT", "MISS-A", "MISS-B"])
        .memory(5)
        .start();

    runner.place_on_field(0, "VB-ENABLER", Some(0));
    let option_slot = hand_index(&runner, 0, CARD_ID);
    assert!(runner.game.activate_hand_main(0, option_slot));
    assert!(
        !runner
            .pending_selection_view()
            .expect("Giant Meat should require choosing a revealed trait card")
            .is_optional,
        "printed add-1 reveal effect is mandatory when a hit is revealed"
    );
    select_first_non_pass(&mut runner);
    runner.auto_resolve().expect("resolve Giant Meat main");

    assert!(hand_contains(&runner, 0, "HIT"));
    assert!(field_contains(&runner, 0, CARD_ID));
    assert!(is_delayed_option_on_field(&runner, 0, CARD_ID));
}

#[test]
fn ex12_073_security_places_this_card_in_the_battle_area() {
    let mut runner = DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("EX12-073 YAML loads")
        .add_card(plain_digimon("ATTACKER", CardColor::Purple, 4, 6000))
        .security(1, &[CARD_ID])
        .memory(0)
        .start();
    let attacker = runner.place_on_field(0, "ATTACKER", Some(0));

    runner.attack_player(attacker, 1, false);
    runner.auto_resolve().expect("resolve Giant Meat security");

    assert!(is_delayed_option_on_field(&runner, 1, CARD_ID));
    assert_eq!(runner.security_count(1), 0);
}
