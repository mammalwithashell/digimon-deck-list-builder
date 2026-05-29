use digimon_engine::action::{encode_attack, space::PASS};
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

fn yellow_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Yellow];
    card.dp = Some(1000);
    card
}

fn opponent_digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(6000);
    card
}

fn encode_permanent(handle: PermanentHandle) -> u16 {
    encode_attack(0, handle.index as u16)
}

#[test]
fn bt6_089_start_of_turn_gains_2_when_behind_on_security() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-089")
        .expect("BT6-089 YAML loads")
        .add_card(make_test_card("SEC", "Security"))
        .security(0, &["SEC"])
        .security(1, &["SEC", "SEC"])
        .memory(0)
        .start();
    let tamer = runner.place_on_field(0, "BT6-089", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(tamer));
    runner.game.drain_effect_queue();

    assert_eq!(runner.memory(), 2);
}

#[test]
fn bt6_089_yellow_attacker_suspends_tamer_to_apply_minus_1000_dp() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-089")
        .expect("BT6-089 YAML loads")
        .add_card(yellow_digimon("YELLOW"))
        .add_card(opponent_digimon("OPP"))
        .memory(10)
        .start();
    let tamer = runner.place_on_field(0, "BT6-089", Some(0));
    let attacker = runner.place_on_field(0, "YELLOW", Some(0));
    let opponent = runner.place_on_field(1, "OPP", Some(0));

    runner.game.enter_main_phase();
    assert!(runner.game.can_attack(attacker, false));
    runner
        .game
        .decode_action(encode_attack(attacker.index as u16, opponent.index as u16), attacker.player);
    let prompt = runner.pending_selection_view().expect("optional suspend prompt");
    runner
        .execute_action(prompt.selecting_player, prompt.valid_action_ids[0])
        .expect("accept tamer trigger");
    let target_prompt = runner.pending_selection_view().expect("opponent Digimon target");
    assert!(target_prompt.valid_action_ids.contains(&encode_permanent(opponent)));
    runner
        .execute_action(target_prompt.selecting_player, encode_permanent(opponent))
        .expect("choose opponent Digimon");

    assert!(runner.game.players[0].battle_area[tamer.index as usize].is_suspended);
    assert_eq!(
        runner.game.modifiers.sum(opponent, ModifierType::ChangeDp),
        -1000
    );

    let mut non_yellow = make_test_card("NON-YELLOW", "Non-yellow");
    non_yellow.card_kind = CardKind::Digimon;
    non_yellow.colors = vec![CardColor::Blue];
    let mut runner = DebugRunner::builder()
        .dsl_card("BT6-089")
        .expect("BT6-089 YAML loads")
        .add_card(non_yellow)
        .add_card(opponent_digimon("OPP"))
        .memory(10)
        .start();
    let tamer = runner.place_on_field(0, "BT6-089", Some(0));
    let attacker = runner.place_on_field(0, "NON-YELLOW", Some(0));
    let opponent = runner.place_on_field(1, "OPP", Some(0));
    runner.game.enter_main_phase();
    assert!(runner.game.can_attack(attacker, false));
    runner
        .game
        .decode_action(encode_attack(attacker.index as u16, opponent.index as u16), attacker.player);
    if let Some(prompt) = runner.pending_selection_view() {
        runner.execute_action(prompt.selecting_player, PASS).ok();
    }
    assert!(!runner.game.players[0].battle_area[tamer.index as usize].is_suspended);
}
