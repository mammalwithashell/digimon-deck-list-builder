//! EX10-033 Pyramidimon
//!
//! This pass covers the printed Fragment(3) keyword. The source-placement and
//! source-trash cost/buff clauses need separate source-selection support.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-033")
        .expect("EX10-033 YAML parses and compiles")
        .build()
}

#[test]
fn ex10_033_on_field_has_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX10-033", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}
