//! RB1-035 Hokuto Amanokawa
//!
//! Implemented slice:
//! - [Start of Your Turn] If opponent has 3 or more Tamers, gain 1 memory.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - [All Turns] when opponent plays a Digimon, suspend this Tamer and branch
//!   memory/draw by played Digimon level.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn rb1_035_has_start_turn_and_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("RB1-035")
        .expect("RB1-035 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("RB1-035").expect("compiled card");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "RB1-035 must have a start-of-turn clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "RB1-035 must have Security play"
    );
}

#[ignore = "pending: G-OPPONENT-PLAYED-DIGIMON-LEVEL-BRANCH — observer needs event Digimon level predicates and source-bound suspend cost"]
#[test]
fn rb1_035_opponent_played_digimon_observer_branches_by_level() {}
