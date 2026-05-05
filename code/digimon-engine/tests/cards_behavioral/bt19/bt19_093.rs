//! BT19-093 Queen Device

use digimon_engine::debug_runner::DebugRunner;

#[test]
fn bt19_093_loads_with_gap_stub() {
    DebugRunner::builder()
        .dsl_card("BT19-093")
        .expect("BT19-093 must load from embedded DSL pack")
        .start();
}

#[ignore = "pending: G-OPTION-BATTLE-AREA-CARRIER — Queen Device placement, trash-from-battle trigger, negative color-bypass predicate, and security target pair"]
#[test]
fn bt19_093_option_battle_area_carrier_and_security_pair() {}
