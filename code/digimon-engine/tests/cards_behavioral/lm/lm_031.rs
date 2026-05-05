//! LM-031 Black Scramble
//!
//! This pass covers the main hand-digivolve clause.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn lm_031_has_main_digivolve_cost_reduction_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("LM-031")
        .expect("LM-031 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("LM-031").expect("LM-031 compiled");
    let main = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::MainFromHand) =>
        {
            Some(triggered)
        }
        _ => None,
    });

    let main = main.expect("LM-031 must have a MainFromHand clause");
    assert!(main
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::EffectInitiatedDigivolve { .. })));
}
