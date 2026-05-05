//! EX10-034 Blastmon
//!
//! This pass covers printed face-up keywords: Collision, Fragment(3), Blocker.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX10-034")
        .expect("EX10-034 YAML parses and compiles")
        .build()
}

#[test]
fn ex10_034_on_field_has_collision_fragment_and_blocker() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX10-034", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Collision));
    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}
