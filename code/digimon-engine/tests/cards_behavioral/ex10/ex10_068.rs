//! EX10-068 Digimon Emperor

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex10_068_loads_delete_and_security_play_slice() {
    DebugRunner::builder()
        .dsl_card("EX10-068")
        .expect("EX10-068 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-DISTINCT-COLOR-COUNT and G-RETURNED-CARD-COLOR-BINDING — start-main color count and same-color hand/trash play tail"]
#[test]
fn ex10_068_color_count_and_returned_card_color_play_tail() {}
