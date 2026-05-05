//! BT13-040 Magnamon

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_040_has_blocker_keyword() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-040")
        .expect("BT13-040 must load from embedded DSL pack")
        .start();
    let card = runner.compiled_card("BT13-040").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Declarative(CompiledDeclarativeClause::GrantKeyword { .. })
    )));
}

#[ignore = "pending: G-WOULD-LEAVE-DRAW-PLAY-VEEMON — replacement observer that draws then may play Veemon from hand or sources"]
#[test]
fn bt13_040_when_leaving_draws_and_may_play_veemon() {}
