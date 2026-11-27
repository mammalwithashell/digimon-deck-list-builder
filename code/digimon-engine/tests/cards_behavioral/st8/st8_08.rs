//! ST8-08 AeroVeedramon — behavioral tests.
//!
//! Printed:
//!   <Jamming>
//!   Inherited: [Your Turn] While you have 8 or more cards in your hand, this
//!     Digimon gains <Security Attack +1>.

use digimon_engine::debug_runner::{make_test_card, DebugRunner};
use digimon_engine::enums::{CardColor, CardKind, Keyword};

#[test]
fn st8_08_has_jamming_and_inherited_sec_plus_1_at_8_cards_in_hand() {
    let mut filler = make_test_card("ST8-08-FILL", "Filler");
    filler.card_kind = CardKind::Digimon;
    filler.level = Some(3);
    filler.dp = Some(2000);

    let mut host = make_test_card("ST8-08-HOST", "UlforceVeedramon");
    host.card_kind = CardKind::Digimon;
    host.level = Some(6);
    host.colors = vec![CardColor::Blue];
    host.dp = Some(11000);

    let hand_8 = vec!["ST8-08-FILL"; 8];
    let mut r = DebugRunner::builder()
        .dsl_card("ST8-08")
        .unwrap()
        .add_card(filler)
        .add_card(host)
        .hand(0, &hand_8)
        .start();
    r.skip_mulligan();

    let aero = r.place_on_field(0, "ST8-08", Some(0));
    assert!(r.game.has_keyword(aero, Keyword::Jamming));

    let stacked = r.place_stack(0, &["ST8-08", "ST8-08-HOST"]);
    r.game.tick_declarative_effects();
    assert_eq!(
        (r.game.effective_security_strike(stacked) - 1),
        1,
        "ST8-08 inherited must grant <Security Attack +1> with 8+ cards in hand"
    );
}
