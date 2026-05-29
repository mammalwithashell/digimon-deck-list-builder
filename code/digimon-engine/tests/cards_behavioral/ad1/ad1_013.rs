use digimon_engine::action::space::encode_attack;
use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card_with_level, DebugRunner};
use digimon_engine::enums::{CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

const CARD_ID: &str = "AD1-013";

fn digimon(id: &str, name: &str, level: u8, dp: i32) -> CardData {
    let mut card = make_test_card_with_level(id, name, level);
    card.card_kind = CardKind::Digimon;
    card.dp = Some(dp);
    card
}

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card(CARD_ID)
        .expect("AD1-013 YAML loads")
        .add_card(digimon("ZERO-A", "Zero A", 4, 3000))
        .add_card(digimon("ZERO-B", "Zero B", 4, 4000))
        .add_card(digimon("STACK-SRC", "Stack Source", 3, 1000))
        .add_card(digimon("STACK-TOP", "Stack Top", 4, 5000))
        .memory(10)
        .start()
}

#[test]
fn ad1_013_delete_fewest_digivolution_cards_preserves_ties() {
    let mut runner = runner();
    let zeig = runner.place_on_field(0, CARD_ID, Some(0));
    let zero_a = runner.place_on_field(1, "ZERO-A", Some(0));
    let zero_b = runner.place_on_field(1, "ZERO-B", Some(0));
    let stacked = runner.place_stack(1, &["STACK-SRC", "STACK-TOP"]);

    runner.game.enqueue_triggered(
        EffectTiming::WhenDigivolving,
        TriggerSource::Permanent(zeig),
    );
    runner.game.drain_effect_queue();

    let pending = runner.pending_selection().expect("delete prompt");
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, zero_a.index as u16)));
    assert!(pending
        .valid_action_ids
        .contains(&encode_attack(0, zero_b.index as u16)));
    assert!(!pending
        .valid_action_ids
        .contains(&encode_attack(0, stacked.index as u16)));
}
