use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt13_030_loads_with_gap_stub() {
    DebugRunner::builder().dsl_card("BT13-030").expect("load").start();
}

#[ignore = "pending: G-FOR-EACH-COUNTED-FIELD-OBJECTS — for each Royal Knight/blue Tamer, trash 2 sources from one opponent Digimon"]
#[test]
fn bt13_030_trashes_sources_for_each_royal_knight_and_blue_tamer() {}
