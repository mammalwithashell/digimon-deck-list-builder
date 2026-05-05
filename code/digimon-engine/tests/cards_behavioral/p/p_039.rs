//! P-039 Black Memory Boost!
//!
//! This pass covers the black reveal/search shape and Delay gain-memory body.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-039")
        .expect("P-039 YAML parses and compiles")
        .build()
}

#[test]
fn p_039_has_main_from_hand_and_delay_gain_two() {
    let runner = runner();
    let card = runner.compiled_card("P-039").expect("P-039 compiled card");

    assert!(card.effects.iter().any(|clause| matches!(
        clause,
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::MainFromHand)
    )));

    let delay = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Declarative(CompiledDeclarativeClause::Delay { process, .. }) => {
            Some(process)
        }
        _ => None,
    });

    let process = delay.expect("P-039 must compile a Delay clause");
    assert!(
        process
            .iter()
            .any(|step| matches!(step, CompiledStep::GainMemory(2))),
        "P-039 Delay must gain 2 memory"
    );
}
