//! EX4-065 Trident Gaia
//!
//! Gap-routed slice:
//! - Highest-DP opponent deletion and the conditional security trash need an
//!   aggregate DP selector plus "deleted by this effect" event payload.
//!
//! Implemented slice:
//! - Card metadata and Main/Security timing shell are present with explicit
//!   stubs documenting the blocked clause.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex4_065_has_main_and_security_shells() {
    let runner = DebugRunner::builder()
        .dsl_card("EX4-065")
        .expect("EX4-065 must load from embedded DSL pack")
        .memory(8)
        .start();
    let card = runner.compiled_card("EX4-065").expect("compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::MainFromHand)
    )));
    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
    )));
}

#[ignore = "pending: G-HIGHEST-DP-DELETE-WITH-EFFECT-PAYLOAD — needs highest-DP select/delete and deleted-by-this-effect conditional security trash"]
#[test]
fn ex4_065_deletes_highest_dp_and_trashes_security_if_13000_or_more() {}
