//! BT20-045 Examon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_045_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-045")
        .expect("BT20-045 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-BLAST-DNA-DIGIVOLVE and G-HIGHEST-DP-SWEEP — Blast DNA, DNA-gated highest-DP bottom-deck, and suspend observer unsuspend"]
#[test]
fn bt20_045_blast_dna_highest_dp_sweep_and_suspend_observer() {}
