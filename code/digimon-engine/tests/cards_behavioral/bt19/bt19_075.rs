//! BT19-075 MoonMillenniummon — Track E forced hand-reduction YAML slice.

use digimon_engine::debug_runner::DebugRunner;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT19-075")
        .expect("BT19-075 YAML loads")
        .start()
}

#[test]
fn bt19_075_yaml_loads_as_track_e_fixture_skeleton() {
    let r = runner();
    let compiled = r.compiled_card("BT19-075").expect("compiled BT19-075");
    assert_eq!(
        compiled.effects.len(),
        0,
        "BT19-075 stays load-only until the result-count Tamer deletion rider lands"
    );
}

#[test]
#[ignore = "blocked on G-ANY-RETURNED-CARD-PREDICATE-style result bindings for 'cards trashed by this effect' and Tamer deletion rider"]
fn bt19_075_deletes_one_tamer_per_two_cards_trashed_by_this_effect() {}

#[test]
#[ignore = "blocked on Composite delete-cost replacement authoring for MoonMillenniummon leave prevention"]
fn bt19_075_composite_cost_prevents_leaving_battle_area() {}
