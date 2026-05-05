//! BT21-086 Marcus Damon
//!
//! Implemented slice:
//! - [Start of Your Main Phase] If opponent has a Digimon, gain 1 memory.
//! - [On Play] 1 of your Marcus Damon Tamers may suspend.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - [All Turns][OPT] when this Tamer suspends, buff one own Digimon and debuff
//!   one opponent Digimon.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt21_086_has_start_main_on_play_and_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT21-086")
        .expect("BT21-086 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT21-086").expect("compiled card");

    for timing in [
        CompiledTiming::StartOfYourMainPhase,
        CompiledTiming::OnPlay,
        CompiledTiming::OnSecurity,
    ] {
        assert!(
            card.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Triggered(t) if t.when.contains(&timing)
            )),
            "BT21-086 must have {timing:?} clause"
        );
    }
}

#[test]
fn bt21_086_on_play_can_suspend_a_marcus_damon() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT21-086")
        .expect("BT21-086 must load")
        .hand(0, &["BT21-086"])
        .memory(5)
        .start();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "BT21-086 On Play should suspend the selected Marcus Damon"
    );
}

#[ignore = "pending: G-DSL-EVENT-TARGET-IS-SELF — on_suspend must fire only when this exact Tamer suspends, then apply Piercing/+3000 and -3000 DP"]
#[test]
fn bt21_086_self_suspend_observer_buffs_and_debuffs() {}
