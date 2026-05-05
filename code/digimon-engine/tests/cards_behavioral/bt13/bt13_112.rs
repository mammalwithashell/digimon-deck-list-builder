//! BT13-112 Omnimon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_112_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT13-112")
        .expect("BT13-112 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-MODAL-BREEDING-SOURCE-PLAY — delete OR play differently named Royal Knights from breeding sources with Rush grant"]
#[test]
fn bt13_112_modal_delete_or_breeding_source_play() {}
