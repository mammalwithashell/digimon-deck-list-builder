//! BT18-009 Shamanmon — behavioral tests.
//!
//! Printed:
//!   [All Turns] Your opponent can't gain memory except with Tamer effects.

use digimon_engine::debug_runner::DebugRunner;
use digimon_engine::enums::ModifierType;

#[test]
fn bt18_009_installs_cannot_gain_memory_except_tamer_floodgate_for_opponent() {
    let mut r = DebugRunner::builder().dsl_card("BT18-009").unwrap().start();
    r.skip_mulligan();
    r.place_on_field(0, "BT18-009", Some(0));
    r.game.tick_declarative_effects();

    assert!(
        r.game
            .modifiers
            .player_has(1, ModifierType::CannotGainMemoryExceptFromTamers),
        "BT18-009 must block opponent from gaining memory except with Tamer effects"
    );
    assert!(
        !r.game
            .modifiers
            .player_has(0, ModifierType::CannotGainMemoryExceptFromTamers),
        "BT18-009 must not block its own controller from gaining memory"
    );
}
