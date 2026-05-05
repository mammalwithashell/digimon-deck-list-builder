//! EX8-055 Pyramidimon
//!
//! This pass covers the printed Fragment(3) keyword. The source-trash cost,
//! unsuspend/security-attack gain, and end-turn source placement clauses need
//! separate choice/source-placement support.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-055")
        .expect("EX8-055 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_055_on_field_has_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX8-055", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}
