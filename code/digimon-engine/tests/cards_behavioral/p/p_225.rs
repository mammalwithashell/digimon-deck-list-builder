use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, DelayTrigger};
use digimon_engine::permanent::OptionState;

fn cs_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.traits = vec!["CS".to_string()];
    card
}

#[test]
fn p_225_main_draws_then_places_self_as_delay_option() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-225")
        .expect("P-225 YAML loads")
        .add_card(cs_digimon("CS-ENABLER", 4))
        .add_card(make_test_card("DRAWN", "Drawn"))
        .hand(0, &["P-225"])
        .deck(0, &["DRAWN"])
        .memory(10)
        .start();
    runner.place_on_field(0, "CS-ENABLER", Some(0));
    runner.game.enter_main_phase();

    runner
        .game
        .decode_action(digimon_engine::action::space::HAND_EFFECT_START, 0);
    runner.auto_resolve().expect("finish DigiLab main");

    assert_eq!(runner.game.players[0].hand[0].card_id(&runner.game.card_data), "DRAWN");
    assert!(runner.game.players[0].battle_area.iter().any(|permanent| {
        permanent.top_card().card_id(&runner.game.card_data) == "P-225"
            && matches!(permanent.option_state, OptionState::Delayed { .. })
    }));
}

#[test]
fn p_225_delay_rotates_top_source_to_bottom_and_gains_2_memory() {
    let mut runner = DebugRunner::builder()
        .dsl_card("P-225")
        .expect("P-225 YAML loads")
        .add_card(cs_digimon("CS-BASE", 4))
        .add_card(cs_digimon("CS-MID", 4))
        .add_card(cs_digimon("CS-TOP", 5))
        .memory(0)
        .start();
    let option = runner.place_on_field(0, "P-225", Some(0));
    runner.game.players[0].battle_area[option.index as usize].option_state =
        OptionState::Delayed {
            owner: 0,
            trash_on_turn: u16::MAX,
            trigger: DelayTrigger::MainPhaseActivated,
            placed_on_turn: 0,
        };
    runner.place_stack(0, &["CS-BASE", "CS-MID", "CS-TOP"]);
    runner.game.turn_count = 1;
    runner.game.enter_main_phase();

    assert!(runner.game.activate_delayed_option_main(option));
    let prompt = runner.pending_selection_view().expect("CS Digimon target");
    runner
        .execute_action(prompt.selecting_player, prompt.valid_action_ids[0])
        .expect("choose CS Digimon");

    let permanent = runner.game.players[0]
        .battle_area
        .iter()
        .find(|permanent| permanent.top_card().card_id(&runner.game.card_data) == "CS-TOP")
        .expect("CS stack remains after delayed option is trashed");
    assert_eq!(permanent.card_sources[0].card_id(&runner.game.card_data), "CS-MID");
    assert_eq!(runner.memory(), 2);
}
