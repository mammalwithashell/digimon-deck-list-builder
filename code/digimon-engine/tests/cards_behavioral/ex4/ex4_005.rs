//! EX4-005 Agumon.
//!
//! [Start of Your Main Phase] If you have a red or yellow Tamer in play, gain
//! 1 memory.
//! Inherited: [Your Turn][OPT] When one of your red or yellow Tamers becomes
//! suspended, Draw 1.

use digimon_engine::card_data::CardData;
use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, EffectTiming};
use digimon_engine::selection::TriggerSource;

#[test]
fn ex4_005_start_of_main_gains_memory_with_red_or_yellow_tamer() {
    for color in [CardColor::Red, CardColor::Yellow] {
        let mut runner = DebugRunner::builder()
            .dsl_card("EX4-005")
            .expect("EX4-005 YAML loads")
            .add_card(tamer("TAMER", color))
            .memory(0)
            .start();
        let agumon = runner.place_on_field(0, "EX4-005", Some(0));
        runner.place_on_field(0, "TAMER", Some(0));

        fire_start_main(&mut runner, agumon);
        runner.auto_resolve().expect("gain memory");

        assert_eq!(runner.memory(), 1);
    }
}

#[test]
fn ex4_005_start_of_main_requires_own_red_or_yellow_tamer() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-005")
        .expect("EX4-005 YAML loads")
        .add_card(tamer("GREEN-TAMER", CardColor::Green))
        .add_card(tamer("OPP-RED-TAMER", CardColor::Red))
        .memory(0)
        .start();
    let agumon = runner.place_on_field(0, "EX4-005", Some(0));
    runner.place_on_field(0, "GREEN-TAMER", Some(0));
    runner.place_on_field(1, "OPP-RED-TAMER", Some(0));

    fire_start_main(&mut runner, agumon);
    runner.auto_resolve().expect("no gain");

    assert_eq!(runner.memory(), 0);
}

#[test]
fn ex4_005_inherited_draws_when_own_red_or_yellow_tamer_suspends() {
    let mut runner = DebugRunner::builder()
        .dsl_card("EX4-005")
        .expect("EX4-005 YAML loads")
        .add_card(digimon("CARRIER"))
        .add_card(tamer("RED-TAMER", CardColor::Red))
        .add_card(make_test_card("DRAW", "Draw"))
        .deck(0, &["DRAW"])
        .start();
    runner.place_stack(0, &["EX4-005", "CARRIER"]);
    let tamer = runner.place_on_field(0, "RED-TAMER", Some(0));

    runner.game.suspend(tamer);
    runner.auto_resolve().expect("inherited draw");

    assert_eq!(runner.hand_size(0), 1);
}

fn fire_start_main(runner: &mut DebugRunner, source: digimon_engine::permanent::PermanentHandle) {
    runner.game.enqueue_triggered(
        EffectTiming::StartOfYourMainPhase,
        TriggerSource::Permanent(source),
    );
    runner.game.drain_effect_queue();
}

fn digimon(id: &str) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Digimon;
    card.colors = vec![CardColor::Red];
    card.level = Some(4);
    card.dp = Some(5000);
    card
}

fn tamer(id: &str, color: CardColor) -> CardData {
    let mut card = make_test_card(id, id);
    card.card_kind = CardKind::Tamer;
    card.colors = vec![color];
    card.level = None;
    card.dp = None;
    card
}
