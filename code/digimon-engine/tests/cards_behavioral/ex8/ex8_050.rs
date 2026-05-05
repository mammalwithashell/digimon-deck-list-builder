//! EX8-050 Gogmamon
//!
//! This pass covers the face-up <Blocker> keyword. On-deletion reveal/play and
//! inherited attack-redirection need separate selection/combat passes.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX8-050")
        .expect("EX8-050 YAML parses and compiles")
        .build()
}

#[test]
fn ex8_050_on_field_has_blocker() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX8-050", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}
