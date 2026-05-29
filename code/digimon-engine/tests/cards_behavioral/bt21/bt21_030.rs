use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "BT21-030";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("BT21-030 YAML loads")
        .add_card(digimon("SRC-A", "Source A", 3, 1000))
        .add_card(digimon("SRC-B", "Source B", 4, 4000))
        .add_card(digimon("STACKED", "Stacked", 5, 7000))
        .add_card(digimon("NO-SOURCE", "No Source", 4, 4000))
        .memory(10)
        .start()
}

#[test]
fn bt21_030_trashes_sources_then_when_attacking_targets_only_no_source_digimon() {
    let mut runner = runner();
    let superior = runner.place_on_field(0, CARD_ID, Some(0));
    let stacked = runner.place_stack(1, &["SRC-A", "SRC-B", "STACKED"]);
    let no_source = runner.place_on_field(1, "NO-SOURCE", Some(0));

    runner
        .game
        .enqueue_triggered(EffectTiming::OnPlay, TriggerSource::Permanent(superior));
    runner.game.drain_effect_queue();
    let _pending = runner
        .pending_selection()
        .expect("On Play should select an opponent stack to trash");
    runner
        .execute_action(0, encode_attack(0, stacked.index as u16))
        .expect("choose stacked target");
    runner.auto_resolve().expect("trash stacked sources");

    assert_eq!(
        runner.game.players[1].battle_area[stacked.index as usize]
            .card_sources
            .len(),
        1,
        "top 10 stacked-card trash should leave only the visible top card"
    );

    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(superior),
    );
    runner.game.drain_effect_queue();
    let pending = runner
        .pending_selection()
        .expect("When Attacking should offer no-source target");
    assert!(pending.is_optional);
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, no_source.index as u16)));
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, stacked.index as u16)));
}
