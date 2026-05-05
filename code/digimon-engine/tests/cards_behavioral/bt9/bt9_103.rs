//! BT9-103 Kongou
//!
//! Implemented slice:
//! - [Main] opponent Digimon with play cost 7 or less can't attack players
//!   until opponent turn end, and opponent can't add cards to security by effect.
//! - [Security] activate Main effects.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt9_103_has_main_and_security_mirror() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-103")
        .expect("BT9-103 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT9-103").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
        )),
        "BT9-103 must have a Main clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT9-103 must have a Security mirror clause"
    );
}
