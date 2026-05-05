//! BT15-092 Revelation of Light

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt15_092_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT15-092")
        .expect("BT15-092 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-SECURITY-TRASHED-SELF-DISPATCH and G-SECURITY-SEARCH-PLAY — security-trash trigger, security search/play, and security-Digimon debuff"]
#[test]
fn bt15_092_security_trash_dispatch_and_security_search_play() {}
