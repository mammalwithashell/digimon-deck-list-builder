//! EX8-067 Close
//!
//! This pass covers the memory setter. The digivolve-triggered trash-to-source
//! placement clause needs source-placement support.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_067_start_of_turn_sets_memory_to_three_when_lte_two() {
    let filler = make_test_card("EX8-067-FILL", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("EX8-067")
        .expect("EX8-067 YAML parses and compiles")
        .add_card(filler)
        .deck(0, &["EX8-067-FILL"])
        .deck(1, &["EX8-067-FILL"])
        .memory(2)
        .start();

    runner.place_on_field(0, "EX8-067", None);
    runner.game.memory = 2;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3);
}

#[test]
fn ex8_067_yaml_compiles() {
    let _ = runner();
}
