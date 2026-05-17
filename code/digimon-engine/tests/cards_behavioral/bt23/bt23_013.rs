//! BT23-013 Jesmon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_013_loads_keyword_slice() {
    DebugRunner::builder()
        .dsl_card("BT23-013")
        .expect("BT23-013 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-UNION-HAND-TRASH-NAME-EXCLUSION — token/Sistermon play and other-played attack observer (G-TOKEN-ATHO-RENE-POR closed by Phase 2 Track J PR 1: Atho/René/Por now in token_registry)"]
#[test]
fn bt23_013_token_sistermon_union_play_and_attack_observer() {}
