//! P-215 Icemon
//!
//! This pass covers inherited <Blocker>. The face-up source-placement and
//! protection clause is a separate effect-immunity/source-placement pass.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::Keyword;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .build()
}

#[test]
fn p_215_inherited_blocker_is_available_from_source() {
    let mut host = make_test_card("HOST", "Host");
    host.dp = Some(5000);

    let mut runner = DebugRunner::builder()
        .dsl_card("P-215")
        .expect("P-215 YAML parses and compiles")
        .add_card(host)
        .build();
    let carrier = runner.place_stack(0, &["P-215", "HOST"]);

    assert!(runner.game.has_keyword(carrier, Keyword::Blocker));
}
