//! BT23-072 King Drasil_7D6

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt23_072_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT23-072")
        .expect("BT23-072 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-HAND-MAIN-SOURCE-PLACEMENT and G-BREEDING-INHERITED-SOURCE-PLAY — hand-main source placement, played-Digimon keyword grant, and breeding source play"]
#[test]
fn bt23_072_hand_main_source_placement_and_breeding_source_play() {}
