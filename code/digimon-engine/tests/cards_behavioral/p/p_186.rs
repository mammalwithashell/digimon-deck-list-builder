//! P-186 Gallantmon
//!
//! This pass covers printed face-up keywords: Rush and Blocker.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-186")
        .expect("P-186 YAML parses and compiles")
        .build()
}

#[test]
fn p_186_on_field_has_rush_and_blocker() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "P-186", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Rush));
    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}
