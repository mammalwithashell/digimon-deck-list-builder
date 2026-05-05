//! BT20-017 Jesmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt20_017_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT20-017")
        .expect("BT20-017 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-TOKEN-ATHO-RENE-POR and G-ALLY-PLAYED-MAY-ATTACK — token registration plus other-Digimon-played delete/attack"]
#[test]
fn bt20_017_token_and_other_played_attack_observer() {}
