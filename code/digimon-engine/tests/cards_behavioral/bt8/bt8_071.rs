//! BT8-071 Psychemon
//!
//! [All Turns] Players can't reduce play costs.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

const YAML: &str = include_str!("../../../cards/bt8/BT8-071.yaml");

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .from_dsl_yaml(YAML)
        .expect("BT8-071 YAML parses")
        .start()
}

#[test]
fn bt8_071_compiles_player_scoped_play_cost_flood_gate() {
    let runner = runner();
    let compiled = runner.compiled_card("BT8-071").expect("BT8-071 compiled");
    let gate = compiled
        .effects
        .iter()
        .find_map(|clause| match clause {
            CompiledClause::Declarative(CompiledDeclarativeClause::FloodGate {
                modifier,
                target_player,
                ..
            }) => Some((modifier.as_str(), *target_player)),
            _ => None,
        })
        .expect("BT8-071 must declare a flood_gate");

    assert_eq!(gate.0, "CannotReducePlayCost");
    assert_eq!(gate.1, Some(CompiledPlayerRef::Any));
}

#[test]
fn bt8_071_installs_play_cost_reduction_lock_on_both_players() {
    let mut runner = runner();
    runner.place_on_field(0, "BT8-071", Some(0));

    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotReducePlayCost));
    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotReducePlayCost));
}
