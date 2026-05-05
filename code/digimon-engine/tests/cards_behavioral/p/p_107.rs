//! P-107 Defense Training
//!
//! This pass covers the black reveal/search shape and Delay digivolve body.

use digimon_dsl::compiled::{
    CompiledClause, CompiledDeclarativeClause, CompiledStep, CompiledTiming,
};
use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("P-107")
        .expect("P-107 YAML parses and compiles")
        .build()
}

#[test]
fn p_107_has_main_from_hand_and_delay_digivolve_body() {
    let runner = runner();
    let card = runner.compiled_card("P-107").expect("P-107 compiled card");

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

    let process = delay.expect("P-107 must compile a Delay clause");
    assert!(
        process
            .iter()
            .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })),
        "P-107 Delay must include an effect-initiated digivolve step"
    );
}
