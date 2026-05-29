//! BT7-032 Pulsemon
//!
//! Inherited Effect [When Attacking] [Once Per Turn] If you have 3 security
//! cards, gain 2 memory.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::EffectTiming;
use digimon_engine::selection::TriggerSource;

const YAML: &str = include_str!("../../../cards/bt7/BT7-032.yaml");

#[test]
fn bt7_032_inherited_when_attacking_gains_memory_at_exactly_three_security() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT7-032 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC1", "Security 1"))
        .add_card(make_test_card("SEC2", "Security 2"))
        .add_card(make_test_card("SEC3", "Security 3"))
        .security(0, &["SEC1", "SEC2", "SEC3"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);

    fire_when_attacking(&mut runner, carrier);

    assert_eq!(runner.memory(), 2);
}

#[test]
fn bt7_032_requires_exactly_three_security_and_once_per_turn() {
    let mut runner = DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT7-032 YAML parses")
        .add_card(make_test_card("CARRIER", "Carrier"))
        .add_card(make_test_card("SEC1", "Security 1"))
        .add_card(make_test_card("SEC2", "Security 2"))
        .add_card(make_test_card("SEC3", "Security 3"))
        .security(0, &["SEC1", "SEC2"])
        .memory(0)
        .start();
    let carrier = runner.place_stack(0, &["BT7-032", "CARRIER"]);

    fire_when_attacking(&mut runner, carrier);
    assert_eq!(runner.memory(), 0);

    let idx = runner
        .game
        .card_data
        .iter()
        .position(|card| card.card_id == "SEC3")
        .expect("SEC3 card data exists");
    let next = runner.game.next_card_index();
    runner.game.players[0]
        .security
        .push(digimon_engine::card_source::CardSource::new(idx, 0, next));
    fire_when_attacking(&mut runner, carrier);
    fire_when_attacking(&mut runner, carrier);
    assert_eq!(
        runner.memory(),
        2,
        "once per turn should allow only one gain"
    );
}

fn fire_when_attacking(
    runner: &mut DebugRunner,
    carrier: digimon_engine::permanent::PermanentHandle,
) {
    runner.game.enqueue_triggered(
        EffectTiming::WhenAttacking,
        TriggerSource::Permanent(carrier),
    );
    runner.game.drain_effect_queue();
}
