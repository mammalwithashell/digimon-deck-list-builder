//! BT14-009 Gotsumon
//!
//! Printed text: "[All Turns] Players can't play Digimon by effects."

use digimon_dsl::compiled::{CompiledClause, CompiledDeclarativeClause, CompiledPlayerRef};
use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

fn runner() -> DebugRunner {
    DebugRunner::builder()
        .dsl_card("BT14-009")
        .expect("BT14-009 YAML parses and compiles")
        .build()
}

#[test]
fn bt14_009_has_player_scoped_cannot_play_digimon_by_effect_flood_gate() {
    let runner = runner();
    let card = runner
        .compiled_card("BT14-009")
        .expect("BT14-009 compiled card present");

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
        .expect("BT14-009 must compile a flood_gate clause");

    assert_eq!(gate.0, "CannotPlayDigimonByEffect");
    assert_eq!(
        gate.1,
        Some(CompiledPlayerRef::Any),
        "printed 'Players' must target both players"
    );
}

#[test]
fn bt14_009_installs_effect_play_lock_on_both_players_while_in_battle_area() {
    let mut runner = runner();
    runner.place_on_field(0, "BT14-009", None);

    runner.game.tick_declarative_effects();

    assert!(runner
        .game
        .modifiers
        .player_has(0, ModifierType::CannotPlayDigimonByEffect));
    assert!(runner
        .game
        .modifiers
        .player_has(1, ModifierType::CannotPlayDigimonByEffect));
}
