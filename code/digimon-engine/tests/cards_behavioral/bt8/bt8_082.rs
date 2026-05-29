//! BT8-082 Ophanimon Falldown Mode.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

#[test]
fn bt8_082_when_digivolving_uses_purple_and_yellow_sources() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-082")
        .expect("BT8-082 loads")
        .add_card(digimon("PURPLE-SOURCE", CardColor::Purple, 5))
        .add_card(digimon("YELLOW-SOURCE", CardColor::Yellow, 5))
        .add_card(digimon("OPP-LV4", CardColor::Red, 4))
        .add_card(make_test_card("RECOVER", "Recover"))
        .deck(0, &["RECOVER"])
        .memory(5)
        .start();
    let opha = runner.place_stack(0, &["PURPLE-SOURCE", "YELLOW-SOURCE", "BT8-082"]);
    let target = runner.place_on_field(1, "OPP-LV4", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::WhenDigivolving, TriggerSource::Permanent(opha));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("delete target");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("select level 4");
    runner.auto_resolve().expect("finish Ophanimon");

    assert!(
        runner.game.players[1].battle_area.len() <= target.index as usize,
        "purple source should delete the selected level 4"
    );
    assert_eq!(runner.game.players[0].security.len(), 1);
}

#[test]
fn bt8_082_on_deletion_can_play_yellow_or_purple_level_four_from_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-082")
        .expect("BT8-082 loads")
        .add_card(digimon("TRASH-LV4", CardColor::Purple, 4))
        .memory(5)
        .start();
    push_to_trash(&mut runner, 0, "TRASH-LV4");
    let opha = runner.place_on_field(0, "BT8-082", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(opha));
    runner.game.drain_effect_queue();
    let view = runner.pending_selection_view().expect("trash selection");
    runner
        .execute_action(view.selecting_player, first_non_pass(&view.valid_action_ids))
        .expect("select trash Lv4");
    runner.auto_resolve().expect("finish play");

    assert!(runner.game.players[0].battle_area.iter().any(|perm| {
        perm.top_card().card_id(&runner.game.card_data) == "TRASH-LV4"
    }));
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn digimon(id: &str, color: CardColor, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(5000);
    card.colors = vec![color];
    card
}

fn push_to_trash(runner: &mut DebugRunner, player: u8, card_id: &str) {
    let data_idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == card_id)
        .unwrap_or_else(|| panic!("{card_id} missing from card data"));
    let next = runner.game.next_card_index();
    runner.game.players[player as usize]
        .trash
        .push(digimon_engine::card_source::CardSource::new(data_idx, player, next));
}
