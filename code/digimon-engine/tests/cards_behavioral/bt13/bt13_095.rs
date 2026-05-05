//! BT13-095 Marcus Damon
//!
//! Implemented slice:
//! - [Start of Your Turn] If memory is 2 or less, set it to 3.
//! - [On Play] This Marcus Damon may suspend.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - On-suspend debuff and conditional Agumon/Greymon memory gain.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_095_has_memory_on_play_and_security_clauses() {
    let runner = DebugRunner::builder()
        .dsl_card("BT13-095")
        .expect("BT13-095 must load from embedded DSL pack")
        .memory(5)
        .start();
    let card = runner.compiled_card("BT13-095").expect("compiled card");

    for timing in [
        CompiledTiming::StartOfYourTurn,
        CompiledTiming::OnPlay,
        CompiledTiming::OnSecurity,
    ] {
        assert!(
            card.effects.iter().any(|clause| matches!(
                clause,
                CompiledClause::Triggered(t) if t.when.contains(&timing)
            )),
            "BT13-095 must have {timing:?} clause"
        );
    }
}

#[test]
fn bt13_095_on_play_can_suspend_self() {
    let mut runner = DebugRunner::builder()
        .dsl_card("BT13-095")
        .expect("BT13-095 must load")
        .hand(0, &["BT13-095"])
        .memory(5)
        .start();

    runner.play(0, 0);
    let _ = runner.auto_resolve();

    assert!(
        runner.game.players[0].battle_area[0].is_suspended,
        "BT13-095 On Play should be able to suspend Marcus"
    );
}

#[ignore = "pending: G-DSL-EVENT-TARGET-IS-SELF — on_suspend must fire only for this exact Marcus, then apply -3000 and conditional memory gain"]
#[test]
fn bt13_095_self_suspend_observer_debuffs_and_checks_agumon_greymon() {}
