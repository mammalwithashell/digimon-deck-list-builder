//! BT23-047 Examon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_047_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT23-047")
        .expect("BT23-047 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-PARTITION and G-N-TARGET-SUSPEND-UNSUSPEND-LOCK — Partition, suspend 5, next unsuspend lock, may attack, and security-removed option trash/delete"]
#[test]
fn bt23_047_partition_suspend_lock_and_security_removed_tail() {}
