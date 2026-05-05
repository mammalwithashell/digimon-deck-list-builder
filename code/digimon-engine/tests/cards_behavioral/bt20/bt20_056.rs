//! BT20-056 Alphamon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_056_loads_barrier_recovery_slice() {
    DebugRunner::builder()
        .dsl_card("BT20-056")
        .expect("BT20-056 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-BREEDING-DIGIVOLVE-UNION-ZONES and G-SECURITY-REMOVED-OBSERVER — during-attack breeding digivolve, security-removed debuff, and inherited replacement"]
#[test]
fn bt20_056_breeding_digivolve_security_removed_and_replacement() {}
