//! BT23-057 Gankoomon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_057_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT23-057")
        .expect("BT23-057 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-MULTI-TRASH-TO-DECK-PLACEMENT and G-TOKEN-HINUKAMUY — trash return cost reduction, Hinukamuy token, and dynamic play-cost delete"]
#[test]
fn bt23_057_trash_return_cost_token_and_dynamic_delete() {}
