//! BT23-035 Dynasmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_035_loads_barrier_debuff_slice() {
    DebugRunner::builder()
        .dsl_card("BT23-035")
        .expect("BT23-035 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-SECURITY-REMOVED-OBSERVER and G-UNTIL-YOUR-TURN-END — security-removed Security A +1 and recovery tail"]
#[test]
fn bt23_035_security_removed_bonus_and_recovery_tail() {}
