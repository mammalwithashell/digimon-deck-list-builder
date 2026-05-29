//! EX10-051 Mummymon.

use digimon_engine::card_data::CardData;
use digimon_engine::card_source::CardSource;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

#[test]
fn ex10_051_on_play_trashes_hand_card_then_de_digivolves() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-051")
        .expect("EX10-051 loads")
        .add_card(filler("DISCARD"))
        .add_card(digimon("BASE", 3, 2000))
        .add_card(digimon("TOP", 5, 7000))
        .hand(0, &["DISCARD"])
        .memory(5)
        .start();
    let mummy = runner.place_on_field(0, "EX10-051", Some(0));
    let target = runner.place_stack(1, &["BASE", "TOP"]);

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(mummy));
    runner.game.drain_effect_queue();

    let hand = runner.pending_selection_view().expect("discard selection");
    runner
        .execute_action(hand.selecting_player, first_non_pass(&hand.valid_action_ids))
        .expect("discard");
    let pick = runner.pending_selection_view().expect("de-digivolve target");
    runner
        .execute_action(pick.selecting_player, encode_permanent(target))
        .expect("target opponent");
    runner.auto_resolve().expect("finish effect");

    assert_eq!(runner.game.players[0].hand.len(), 0);
    assert_eq!(
        runner.game.players[1].battle_area[target.index as usize].top_card().card_id(&runner.game.card_data),
        "BASE"
    );
}

#[test]
fn ex10_051_on_deletion_masks_same_named_tamer_in_trash() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX10-051")
        .expect("EX10-051 loads")
        .add_card(myotismon_tamer("TAMER", "Myotismon Aide"))
        .memory(5)
        .start();
    push_to_trash(&mut runner, 0, "TAMER");
    let mummy = runner.place_on_field(0, "EX10-051", Some(0));
    let _same_name = runner.place_on_field(0, "TAMER", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnDeletion, TriggerSource::Permanent(mummy));
    runner.game.drain_effect_queue();

    assert!(
        runner.pending_selection_view().is_none(),
        "same-name in-play Tamer must be excluded from the trash play selection"
    );
}

fn first_non_pass(ids: &[u16]) -> u16 {
    ids.iter()
        .copied()
        .find(|id| *id != digimon_engine::action::space::PASS)
        .expect("non-pass action")
}

fn encode_permanent(handle: digimon_engine::permanent::PermanentHandle) -> u16 {
    digimon_engine::action::space::encode_attack(0, handle.index as u16)
}

fn filler(id: &str) -> CardData {
    make_test_card(id, id)
}

fn digimon(id: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(dp);
    card
}

fn myotismon_tamer(id: &str, name: &str) -> CardData {
    let mut card = make_test_card(id, name);
    card.card_kind = CardKind::Tamer;
    card.effect_text = "This text mentions Myotismon.".to_string();
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
        .push(CardSource::new(data_idx, player, next));
}
