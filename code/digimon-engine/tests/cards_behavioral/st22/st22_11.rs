//! ST22-11 Defense Plug-In F
//!
//! This pass covers the security De-Digivolve 2 clause. Link and the Main
//! grant clause require link-option support.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn st22_11_has_security_dedigivolve_two() {
    let runner = DebugRunner::builder()
        .dsl_card("ST22-11")
        .expect("ST22-11 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("ST22-11").expect("ST22-11 compiled");
    let security = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let security = security.expect("ST22-11 must have security clause");
    assert!(security.process.iter().any(|step| matches!(
        step,
        CompiledStep::DeDigivolve {
            amount: Some(2),
            ..
        }
    )));
}
