//! EX8-073 Gallantmon (X Antibody)

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex8_073_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("EX8-073")
        .expect("EX8-073 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-SOURCE-CONDITION and G-EFFECT-RESULT-FALLBACK — source-gated DP swings, delete-or-security fallback, and memory aura immunity"]
#[test]
fn ex8_073_source_condition_delete_fallback_and_memory_aura() {}
