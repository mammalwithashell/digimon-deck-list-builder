//! EX11-044 Pyramidimon
//!
//! This pass covers printed Reboot and Fragment(3). The source-trash
//! highest-cost delete and trash-to-source refill clauses need later support.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("EX11-044")
        .expect("EX11-044 YAML parses and compiles")
        .build()
}

#[test]
fn ex11_044_on_field_has_reboot_and_fragment_three() {
    let mut runner = runner();
    let handle = runner.place_on_field(0, "EX11-044", None);
    runner.game.tick_declarative_effects();

    assert!(runner.game.has_keyword(handle, Keyword::Reboot));
    assert!(runner.game.has_keyword(handle, Keyword::Fragment(3)));
}
