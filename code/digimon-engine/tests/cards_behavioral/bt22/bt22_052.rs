//! BT22-052 Leopardmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt22_052_loads_play_and_blocker_slice() {
    DebugRunner::builder()
        .dsl_card("BT22-052")
        .expect("BT22-052 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-BLAST-DIGIVOLVE and G-OTHER-LEAVE-OBSERVER — Blast Digivolve plus other-Digimon would-leave memory replacement"]
#[test]
fn bt22_052_blast_and_other_leave_memory_observer() {}
