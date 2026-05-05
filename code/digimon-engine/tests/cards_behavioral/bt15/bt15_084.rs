//! BT15-084 Kari Kamiya
//!
//! Implemented slice:
//! - [Start of Your Turn] If memory is 2 or less, set it to 3.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - Security-trash and own-security-removed observers that suspend this Tamer
//!   and apply Security Attack minus.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt15_084_has_memory_floor_and_security_play() {
    let runner = DebugRunner::builder()
        .dsl_card("BT15-084")
        .expect("BT15-084 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT15-084").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "BT15-084 must have a start-of-turn memory floor"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT15-084 must have a Security play clause"
    );
}

#[ignore = "pending: G-SECURITY-TRASHED-FROM-STACK-OBSERVER — needs observer for this card being trashed from security by an effect"]
#[test]
fn bt15_084_when_trashed_from_security_applies_security_attack_minus() {}

#[ignore = "pending: G-OWN-SECURITY-REMOVED-SUSPEND-COST — needs own-security-removed observer with source-bound suspend cost"]
#[test]
fn bt15_084_when_security_removed_suspends_and_applies_security_attack_minus() {}
