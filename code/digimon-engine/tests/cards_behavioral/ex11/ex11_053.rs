//! EX11-053 Omekamon

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn ex11_053_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("EX11-053")
        .expect("EX11-053 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-HAND-TO-FIELDED-SOURCE and G-UNION-HAND-SOURCE-PLAY — place Royal Knight under fielded King Drasil, play Omnimon X from hand/source, then attach self"]
#[test]
fn ex11_053_hand_to_source_and_union_hand_source_play() {}
