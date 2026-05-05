//! BT9-092 Cool Boy
//!
//! Implemented slice:
//! - [On Play] reveal 3; add 1 X Antibody Digimon and 1 X Antibody Option;
//!   bottom the rest.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - Same-level X Antibody digivolve observer with suspend-self cost.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt9_092_has_on_play_search_and_security_play() {
    let runner = DebugRunner::builder()
        .dsl_card("BT9-092")
        .expect("BT9-092 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT9-092").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnPlay)
        )),
        "BT9-092 must have OnPlay search"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT9-092 must have Security play"
    );
}

#[ignore = "pending: G-SAME-LEVEL-X-DIGIVOLVE-OBSERVER — needs digivolve event predicates for same-level into X Antibody plus suspend-self cost"]
#[test]
fn bt9_092_same_level_x_antibody_digivolve_observer_suspends_draws_and_gains_memory() {}
