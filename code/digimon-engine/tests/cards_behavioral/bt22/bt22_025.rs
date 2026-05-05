//! BT22-025 UlforceVeedramon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt22_025_loads_when_attacking_unsuspend_slice() {
    DebugRunner::builder()
        .dsl_card("BT22-025")
        .expect("BT22-025 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-MODAL-LOWEST-LEVEL-BOTTOM-DECK — modal return lowest-level opponent Digimon or play blue Tamer from hand"]
#[test]
fn bt22_025_modal_lowest_level_bottom_deck_or_tamer_play() {}
