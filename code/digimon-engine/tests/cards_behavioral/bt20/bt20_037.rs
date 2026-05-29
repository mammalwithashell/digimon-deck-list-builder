use digimon_engine::action::space::encode_attack;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming, ModifierType};
use digimon_engine::permanent::PermanentHandle;
use digimon_engine::selection::TriggerSource;

#[test]
fn bt20_037_counts_level_six_sources_for_suspend_count_and_memory_then_locks_all_targets() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT20-037")
        .expect("BT20-037 YAML loads")
        .add_card(level_digimon("SRC-6Y", 6))
        .add_card(level_digimon("SRC-6G", 6))
        .add_card(level_digimon("SRC-5", 5))
        .add_card(level_digimon("OPP-A", 4))
        .add_card(level_digimon("OPP-B", 4))
        .add_card(tamer("OPP-TAMER"))
        .memory(0)
        .start();
    let source = runner.place_stack(0, &["SRC-6Y", "SRC-5", "SRC-6G", "BT20-037"]);
    let opp_a = runner.place_on_field(1, "OPP-A", Some(0));
    let opp_b = runner.place_on_field(1, "OPP-B", Some(0));
    let opp_tamer = runner.place_on_field(1, "OPP-TAMER", Some(0));

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
    choose_permanent(&mut runner, opp_a, "select first suspend target");
    choose_permanent(&mut runner, opp_tamer, "select second suspend target");
    runner.auto_resolve().expect("finish BT20-037 effect");

    assert!(runner.game.players[1].battle_area[opp_a.index as usize].is_suspended);
    assert!(runner.game.players[1].battle_area[opp_tamer.index as usize].is_suspended);
    assert!(
        !runner.game.players[1].battle_area[opp_b.index as usize].is_suspended,
        "only two targets should be suspended because only two level 6 sources matched"
    );
    assert_eq!(runner.game.memory, 2);

    for handle in [opp_a, opp_b, opp_tamer] {
        assert!(runner
            .modifiers()
            .has(handle, ModifierType::CannotActivateOnPlayEffects));
        assert!(runner
            .modifiers()
            .has(handle, ModifierType::CannotUnsuspend));
    }
}

fn choose_permanent(runner: &mut DebugRunner, handle: PermanentHandle, label: &str) {
    let view = runner.pending_selection_view().expect(label);
    let action = [
        encode_attack(0, handle.index as u16),
        encode_attack(handle.player as u16, handle.index as u16),
    ]
    .into_iter()
    .find(|action| view.valid_action_ids.contains(action))
    .unwrap_or_else(|| {
        panic!(
            "{label}: expected action for {handle:?}, got {:?}",
            view.valid_action_ids
        )
    });
    runner
        .execute_action(view.selecting_player, action)
        .expect(label);
}

fn level_digimon(id: &str, level: u8) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.level = Some(level);
    card.dp = Some(6000);
    card
}

fn tamer(id: &str) -> digimon_engine::card_data::CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.level = None;
    card.dp = None;
    card
}
