//! BT23-057 Gankoomon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_057_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT23-057")
        .expect("BT23-057 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-MULTI-TRASH-TO-DECK-PLACEMENT plus dynamic play-cost delete — trash-return cost reduction remains open; Hinukamuy token is registered; tracker: qa/archetype-qa/dsl/royal-knights-2026-05-03-dsl-engine-gaps.md"]
#[test]
fn bt23_057_trash_return_cost_token_and_dynamic_delete() {}
