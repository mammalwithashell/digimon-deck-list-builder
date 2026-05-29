use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

fn cs_digimon(id: &str, level: u8) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.traits = vec!["CS".to_string()];
    card
}

#[test]
fn bt22_101_sets_memory_to_3_and_returns_cs_digimon_on_ally_deletion() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT22-101")
        .expect("BT22-101 YAML loads")
        .add_card(cs_digimon("CS-LV4", 4))
        .add_card(cs_digimon("CS-TRASH", 3))
        .hand(0, &["CS-TRASH"])
        .memory(2)
        .start();
    let trash_card = runner.game.players[0].hand.pop().unwrap();
    runner.game.players[0].trash.push(trash_card);
    let kyoko = runner.place_on_field(0, "BT22-101", Some(0));
    let deleted = runner.place_on_field(0, "CS-LV4", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::StartOfYourTurn, TriggerSource::Permanent(kyoko));
    runner.game.drain_effect_queue();
    assert_eq!(runner.memory(), 3);

    runner.game.delete_permanent_with_effects(deleted);
    let prompt = runner.pending_selection_view().expect("optional suspend trigger");
    runner
        .execute_action(prompt.selecting_player, prompt.valid_action_ids[0])
        .expect("accept Kyoko trigger");
    let prompt = runner.pending_selection_view().expect("trash CS pick");
    runner
        .execute_action(prompt.selecting_player, prompt.valid_action_ids[0])
        .expect("return CS Digimon");

    assert!(runner.game.players[0].battle_area[kyoko.index as usize].is_suspended);
    assert!(runner.game.players[0]
        .hand
        .iter()
        .any(|card| card.card_id(&runner.game.card_data) == "CS-TRASH"));
}

#[test]
fn bt22_101_security_plays_tamer() {
    let runner = DebugRunner::builder()
        .dsl_card("BT22-101")
        .expect("BT22-101 YAML loads")
        .build();
    let card = runner
        .game
        .card_data
        .iter()
        .find(|card| card.card_id == "BT22-101")
        .expect("card data");
    assert_eq!(card.card_kind, CardKind::Tamer);
}
