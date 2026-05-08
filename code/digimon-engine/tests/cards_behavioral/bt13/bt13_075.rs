use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_075_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT13-075")
        .expect("load")
        .start();
}

#[ignore = "pending: G-PLAY-COST-GTE-MODIFIER-AURA — after placing a source, opponent play-cost 10+ Digimon cannot attack players"]
#[test]
fn bt13_075_places_source_then_blocks_high_play_cost_attacks() {}
