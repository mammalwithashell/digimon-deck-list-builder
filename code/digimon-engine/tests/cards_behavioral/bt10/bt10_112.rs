//! BT10-112 Jesmon GX — behavioral tests.
//!
//! Printed:
//!   [When Digivolving] You may place 1 Digimon card with the [Royal Knight]
//!     trait and a play cost of 13 or less from your hand or trash as this
//!     Digimon's bottom digivolution card to activate 1 of that card's
//!     [When Digivolving] effects as an effect of this Digimon. Then, <Blitz>.
//!   [All Turns] While this Digimon has a card with the [Royal Knight] trait
//!     in its digivolution cards, this Digimon gains <Piercing>, <Blocker>,
//!     and <Security A. +1> for each card with the [Royal Knight] trait in its
//!     digivolution cards.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn bt10_112_auras_grant_piercing_blocker_and_security_attack_per_royal_knight_source() {
    let mut rk1 = make_test_card("BT10-112-RK1", "Jesmon");
    rk1.card_kind = CardKind::Digimon;
    rk1.level = Some(6);
    rk1.colors = vec![CardColor::Red];
    rk1.dp = Some(12000);
    rk1.traits = vec!["Holy Warrior".to_string(), "Royal Knight".to_string()];

    let mut rk2 = make_test_card("BT10-112-RK2", "Gankoomon");
    rk2.card_kind = CardKind::Digimon;
    rk2.level = Some(6);
    rk2.colors = vec![CardColor::Black];
    rk2.dp = Some(12000);
    rk2.traits = vec!["Holy Warrior".to_string(), "Royal Knight".to_string()];

    let mut r = DebugRunner::builder()
        .dsl_card("BT10-112")
        .unwrap()
        .add_card(rk1)
        .add_card(rk2)
        .start();
    r.skip_mulligan();

    // Without Royal Knight sources -> no Piercing / Blocker.
    let bare = r.place_on_field(0, "BT10-112", Some(0));
    r.game.tick_declarative_effects();
    assert!(!r.game.has_keyword(bare, Keyword::Piercing));
    assert!(!r.game.has_keyword(bare, Keyword::Blocker));

    // With 2 Royal Knight sources -> Piercing, Blocker, and Security Attack +2.
    let stacked = r.place_stack(0, &["BT10-112-RK1", "BT10-112-RK2", "BT10-112"]);
    r.game.tick_declarative_effects();
    assert!(r.game.has_keyword(stacked, Keyword::Piercing));
    assert!(r.game.has_keyword(stacked, Keyword::Blocker));
    assert_eq!(
        (r.game.effective_security_strike(stacked) - 1),
        2,
        "BT10-112 must gain +1 Security Attack per [Royal Knight] source"
    );
}
