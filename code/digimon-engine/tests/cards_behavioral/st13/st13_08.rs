//! ST13-08 Chikurimon
//!
//! Printed text: "[All Turns] Players can't reduce play costs."

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST13-08")
        .expect("ST13-08 YAML parses and compiles")
        .build()
}

#[test]
fn st13_08_has_player_scoped_cannot_reduce_play_cost_flood_gate() {
    let runner = runner();
    let card = runner
        .compiled_card("ST13-08")
        .expect("ST13-08 compiled card present");

    let gate = card
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
        .expect("ST13-08 must compile a flood_gate clause");

    assert_eq!(gate.0, "CannotReducePlayCost");
    assert_eq!(
        gate.1,
        Some(CompiledPlayerRef::Any),
        "printed 'Players' must target both players"
    );
}

#[test]
fn st13_08_installs_play_cost_reduction_lock_on_both_players() {
    let mut runner = runner();
    runner.place_on_field(0, "ST13-08", None);

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
