//! BT23-057 Gankoomon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_057_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT23-057")
        .expect("BT23-057 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-MULTI-TRASH-TO-DECK-PLACEMENT — trash-return cost reduction and dynamic play-cost delete. Hinukamuy token now registered (token_registry.rs, S2.3); the production BT23-057 card body remains unauthored (Track J PR 3)."]
#[test]
fn bt23_057_trash_return_cost_token_and_dynamic_delete() {}
