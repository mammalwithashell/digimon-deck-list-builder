//! BT20-021 Jesmon GX

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_021_loads_ace_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-021")
        .expect("BT20-021 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-UNION-HAND-TRASH-SOURCE-COST and G-SOURCE-COUNT-SECURITY-TRASH — Royal Knight source cost, DP compare, unsuspend, and source-count security trash"]
#[test]
fn bt20_021_source_cost_delete_and_source_count_security_trash() {}
