//! BT8-094 Digimon Emperor
//!
//! This pass covers the security play clause. The deletion draw and
//! breeding-move memory clauses need event-context/on-move support.

use digimon_dsl::compiled::{CompiledClause, CompiledStep, CompiledTiming};
use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt8_094_has_security_play_from_security_clause() {
    let runner = DebugRunner::builder()
        .dsl_card("BT8-094")
        .expect("BT8-094 YAML parses and compiles")
        .build();
    let card = runner.compiled_card("BT8-094").expect("BT8-094 compiled");
    let security = card.effects.iter().find_map(|clause| match clause {
        CompiledClause::Triggered(triggered)
            if triggered.when.contains(&CompiledTiming::OnSecurity) =>
        {
            Some(triggered)
        }
        _ => None,
    });
    let security = security.expect("BT8-094 must have security clause");
    assert!(security
        .process
        .iter()
        .any(|step| matches!(step, CompiledStep::PlayFromSecurity)));
}
