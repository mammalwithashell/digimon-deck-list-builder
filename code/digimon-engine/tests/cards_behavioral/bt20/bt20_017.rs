//! BT20-017 Jesmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_017_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-ALLY-PLAYED-MAY-ATTACK — other-Digimon-played delete/may-attack observer (G-TOKEN-ATHO-RENE-POR closed by Phase 2 Track J PR 1: Atho/René/Por now in token_registry)"]
#[test]
fn bt20_017_token_and_other_played_attack_observer() {}
