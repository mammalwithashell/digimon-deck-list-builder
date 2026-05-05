//! BT13-111 Gallantmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_111_loads_rush_slice() {
    DebugRunner::builder()
        .dsl_card("BT13-111")
        .expect("BT13-111 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-COMBINED-TRASH-COUNT-COST and G-EFFECT-RESULT-FALLBACK — trash-count play reduction plus delete fallback"]
#[test]
fn bt13_111_cost_reduction_and_delete_fallback() {}
