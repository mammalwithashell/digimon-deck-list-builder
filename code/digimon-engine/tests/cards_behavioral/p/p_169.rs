//! P-169 Close
//!
//! This pass covers the start-main memory clause. The source-trash
//! trash-to-source placement clause needs source-placement support.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-169")
        .expect("P-169 YAML parses and compiles")
        .build()
}

#[test]
fn p_169_start_main_gains_one_memory_if_opponent_has_digimon() {
    let filler = make_test_card("P-169-FILL", "Filler");
    let mut opponent = make_test_card("P-169-OPP", "Opponent");
    opponent.play_cost = 3;
    let mut runner = DebugRunner::builder()
        .dsl_card("P-169")
        .expect("P-169 YAML parses and compiles")
        .add_card(filler)
        .add_card(opponent)
        .deck(0, &["P-169-FILL"])
        .deck(1, &["P-169-FILL"])
        .memory(0)
        .start();

    runner.place_on_field(0, "P-169", None);
    runner.place_on_field(1, "P-169-OPP", None);
    runner.game.memory = 0;

    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 1);
}

#[test]
fn p_169_yaml_compiles() {
    let _ = runner();
}
