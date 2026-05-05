//! AD1-017 Dynasmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ad1_017_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("AD1-017")
        .expect("AD1-017 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-SECURITY-TRASH-COST-CHOICE — top-or-bottom security trash as an effect cost, then board-wide -6000 DP"]
#[test]
fn ad1_017_trashes_top_or_bottom_security_to_debuff_all_opponent_digimon() {}
