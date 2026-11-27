//! EX4-042 DarkMaildramon — behavioral tests.
//!
//! Printed:
//!   [Your Turn] This Digimon and all of your Digimon with [Knightmon] or
//!     [Knightsmon] in their names can't be blocked.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, ModifierType};

#[test]
fn ex4_042_grants_cannot_be_blocked_to_self_and_knightmon_on_your_turn() {
    let mut knightmon = make_test_card("EX4-042-KM", "DarkKnightmon");
    knightmon.card_kind = CardKind::Digimon;
    knightmon.level = Some(5);
    knightmon.colors = vec![CardColor::Black];
    knightmon.dp = Some(8000);

    let mut other = make_test_card("EX4-042-OTH", "Agumon");
    other.card_kind = CardKind::Digimon;
    other.level = Some(3);
    other.colors = vec![CardColor::Red];
    other.dp = Some(2000);

    let mut r = DebugRunner::builder()
        .dsl_card("EX4-042")
        .unwrap()
        .add_card(knightmon)
        .add_card(other)
        .start();
    r.skip_mulligan();

    let dm = r.place_on_field(0, "EX4-042", Some(0));
    let km = r.place_on_field(0, "EX4-042-KM", Some(0));
    let ag = r.place_on_field(0, "EX4-042-OTH", Some(0));
    r.game.tick_declarative_effects();

    assert!(r.game.modifiers.has(dm, ModifierType::CannotBeBlocked));
    assert!(r.game.modifiers.has(km, ModifierType::CannotBeBlocked));
    assert!(!r.game.modifiers.has(ag, ModifierType::CannotBeBlocked));
}
