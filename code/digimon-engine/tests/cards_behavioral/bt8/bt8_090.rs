//! BT8-090 Kari Kamiya
//!
//! Implemented slice:
//! - [Start of Your Turn] If memory is 2 or less, set it to 3.
//! - [Security] Play this card free.
//!
//! Gap-routed slice:
//! - [Your Turn] When a card is added to your security stack, may suspend
//!   this Tamer to gain 1 memory.

use digimon_dsl::compiled::{CompiledClause, CompiledTiming};
use digimon_engine::debug_runner::{make_test_card, DebugRunner};

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load from embedded DSL pack")
        .memory(5)
        .start()
}

#[test]
fn bt8_090_has_memory_floor_and_security_play_clauses() {
    let runner = runner();
    let card = runner
        .compiled_card("BT8-090")
        .expect("BT8-090 must be compiled");

    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::StartOfYourTurn)
        )),
        "BT8-090 must have the memory floor clause"
    );
    assert!(
        card.effects.iter().any(|clause| matches!(
            clause,
            CompiledClause::Triggered(t) if t.when.contains(&CompiledTiming::OnSecurity)
        )),
        "BT8-090 must have the Security play clause"
    );
}

#[test]
fn bt8_090_start_of_turn_sets_memory_to_3_when_lte_2() {
    let filler = make_test_card("BT8-090-FILLER", "Filler");
    let mut runner = DebugRunner::builder()
        .dsl_card("BT8-090")
        .expect("BT8-090 must load")
        .add_card(filler)
        .deck(0, &["BT8-090-FILLER"])
        .deck(1, &["BT8-090-FILLER"])
        .memory(2)
        .start();

    runner.place_on_field(0, "BT8-090", None);
    runner.game.memory = 2;
    runner.end_turn();
    runner.end_turn();

    assert_eq!(runner.memory(), 3, "Kari should set memory to 3 at <=2");
}

#[ignore = "pending: G-OWN-SECURITY-ADDED-OBSERVER — DSL/engine needs a global observer for cards added to own security stack with suspend-self cost"]
#[test]
fn bt8_090_security_added_observer_suspends_and_gains_memory() {}
