//! BT23-059 Justimon: Blitz Arm
//!
//! This pass covers the printed Blocker keyword. The option-in-battle-area cost,
//! lowest-play-cost delete, unsuspend, and protection clauses need a later pass.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT23-059")
        .expect("BT23-059 YAML parses and compiles")
        .build()
}

#[test]
fn bt23_059_on_field_has_blocker() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "BT23-059", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Blocker));
}
