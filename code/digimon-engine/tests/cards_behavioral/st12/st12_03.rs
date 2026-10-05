//! ST12-03 Solarmon
//!
//! Printed text: "[All Turns] Players can't reduce play costs."
//!
//! Same printed text as ST13-08 Chikurimon; DCGO has no ST12_03.cs (shared
//! `CannotReduceCostClass` behavior). Official Q&A: negates "reduce the play
//! cost by X" for both players.

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("ST12-03")
        .expect("ST12-03 YAML parses and compiles")
        .build()
}

#[test]
fn st12_03_has_player_scoped_cannot_reduce_play_cost_flood_gate() {
    let runner = runner();
    let card = runner
        .compiled_card("ST12-03")
        .expect("ST12-03 compiled card present");

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
        .expect("ST12-03 must compile a flood_gate clause");

    assert_eq!(gate.0, "CannotReducePlayCost");
    assert_eq!(
        gate.1,
        Some(CompiledPlayerRef::Any),
        "printed 'Players' must target both players"
    );
}

#[test]
fn st12_03_installs_play_cost_reduction_lock_on_both_players() {
    let mut runner = runner();
    runner.place_on_field(0, "ST12-03", None);

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
