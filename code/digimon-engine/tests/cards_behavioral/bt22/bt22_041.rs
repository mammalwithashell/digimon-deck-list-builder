//! BT22-041 Kentaurosmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt22_041_loads_keyword_security_place_slice() {
    DebugRunner::builder()
        .dsl_card("BT22-041")
        .expect("BT22-041 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-TOTAL-SECURITY-COST-REDUCTION and G-SELF-SUSPEND-SECURITY-COST — total-security cost reduction and suspend-trigger security-trash unsuspend"]
#[test]
fn bt22_041_total_security_cost_reduction_and_suspend_unsuspend_cost() {}
